use crate::conversion::errors::ConversionError;
use crate::conversion::formats::{detect_file, expected_extensions, same_format};
use crate::conversion::heif::decode_heif;
use crate::conversion::naming::{reserve_destination, resolve_output_directory};
use crate::models::{CollisionPolicy, InputFormat, OutputFormat, OutputLocation};
use exif::{In, Reader, Tag};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, RgbImage};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const WEBP_QUALITY: f32 = 82.0;

#[derive(Debug, Clone)]
pub struct ConversionRequest {
    pub source: PathBuf,
    pub output_format: OutputFormat,
    pub output_location: OutputLocation,
    pub collision_policy: CollisionPolicy,
    pub jpeg_quality: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionOutcome {
    Completed(PathBuf),
    Skipped {
        format: InputFormat,
        extension_mismatch: bool,
    },
}

pub fn convert_file(
    request: &ConversionRequest,
    cancelled: Arc<AtomicBool>,
) -> Result<ConversionOutcome, ConversionError> {
    check_cancelled(&cancelled)?;
    let source = request.source.canonicalize()?;
    if !source.is_file() {
        return Err(ConversionError::UnsupportedFormat);
    }
    let input_format = detect_file(&source)?;
    if same_format(input_format, request.output_format) {
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        return Ok(ConversionOutcome::Skipped {
            format: input_format,
            extension_mismatch: !expected_extensions(input_format).contains(&extension.as_str()),
        });
    }
    if input_format == InputFormat::Gif || is_animated_webp(&source, input_format)? {
        return Err(ConversionError::AnimatedUnsupported);
    }

    let output_directory = resolve_output_directory(&source, &request.output_location)?;
    validate_writable(&output_directory)?;
    let destination = reserve_destination(
        &source,
        &output_directory,
        request.output_format,
        request.collision_policy,
    )?;
    check_cancelled(&cancelled)?;

    let image = match input_format {
        // libheif already applies container rotation and mirroring. EXIF can
        // repeat that orientation, so applying it here would rotate twice.
        InputFormat::Heif | InputFormat::Avif => decode_heif(&source)?,
        _ => apply_orientation(
            image::ImageReader::open(&source)?
                .with_guessed_format()?
                .decode()?,
            read_orientation(&source),
        ),
    };
    check_cancelled(&cancelled)?;

    encode_to_temp(
        &image,
        request.output_format,
        request.jpeg_quality,
        &destination.temp_path,
    )?;
    check_cancelled(&cancelled)?;
    finalize(
        &destination.temp_path,
        &destination.final_path,
        request.collision_policy,
    )?;
    Ok(ConversionOutcome::Completed(destination.final_path.clone()))
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ConversionError> {
    if cancelled.load(Ordering::Relaxed) {
        Err(ConversionError::Cancelled)
    } else {
        Ok(())
    }
}

fn validate_writable(directory: &Path) -> Result<(), ConversionError> {
    let probe = directory.join(format!(
        ".rasterdrop-write-test-{}",
        uuid::Uuid::new_v4().simple()
    ));
    match File::options().write(true).create_new(true).open(&probe) {
        Ok(file) => {
            drop(file);
            fs::remove_file(probe)?;
            Ok(())
        }
        Err(error) => Err(ConversionError::Io(error)),
    }
}

fn encode_to_temp(
    image: &DynamicImage,
    format: OutputFormat,
    jpeg_quality: u8,
    path: &Path,
) -> Result<(), ConversionError> {
    let file = File::options().write(true).create_new(true).open(path)?;
    let mut writer = BufWriter::new(file);
    match format {
        OutputFormat::Jpeg => {
            let rgb = composite_alpha_on_white(image);
            JpegEncoder::new_with_quality(&mut writer, jpeg_quality.clamp(1, 100))
                .encode_image(&DynamicImage::ImageRgb8(rgb))?;
        }
        OutputFormat::Bmp => {
            DynamicImage::ImageRgb8(composite_alpha_on_white(image))
                .write_to(&mut writer, ImageFormat::Bmp)?;
        }
        OutputFormat::Png => image.write_to(&mut writer, ImageFormat::Png)?,
        OutputFormat::WebP => {
            let rgba = image.to_rgba8();
            let encoder = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height());
            let mut config = webp::WebPConfig::new().map_err(|error| {
                ConversionError::WebP(format!("could not initialize encoder: {error:?}"))
            })?;
            config.quality = WEBP_QUALITY;
            config.method = 6;
            config.alpha_quality = 90;
            let encoded = encoder
                .encode_advanced(&config)
                .map_err(|error| ConversionError::WebP(format!("{error:?}")))?;
            writer.write_all(&encoded)?;
        }
        OutputFormat::Tiff => image.write_to(&mut writer, ImageFormat::Tiff)?,
        OutputFormat::Avif => image.write_to(&mut writer, ImageFormat::Avif)?,
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}

fn finalize(
    temp: &Path,
    destination: &Path,
    policy: CollisionPolicy,
) -> Result<(), ConversionError> {
    if policy == CollisionPolicy::CreateNumberedFile && destination.exists() {
        return Err(ConversionError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "destination appeared during conversion",
        )));
    }
    atomic_install(temp, destination)
}

#[cfg(not(windows))]
fn atomic_install(temp: &Path, destination: &Path) -> Result<(), ConversionError> {
    fs::rename(temp, destination)?;
    Ok(())
}

#[cfg(windows)]
fn atomic_install(temp: &Path, destination: &Path) -> Result<(), ConversionError> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows_sys::Win32::Storage::FileSystem::{REPLACEFILE_WRITE_THROUGH, ReplaceFileW};

    if !destination.exists() {
        fs::rename(temp, destination)?;
        return Ok(());
    }
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let temp_wide: Vec<u16> = temp
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe {
        ReplaceFileW(
            destination_wide.as_ptr(),
            temp_wide.as_ptr(),
            ptr::null(),
            REPLACEFILE_WRITE_THROUGH,
            ptr::null(),
            ptr::null(),
        )
    };
    if result == 0 {
        return Err(ConversionError::Io(std::io::Error::last_os_error()));
    }
    Ok(())
}

pub fn composite_alpha_on_white(image: &DynamicImage) -> RgbImage {
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let pixel = rgba.get_pixel(x, y).0;
        let alpha = u16::from(pixel[3]);
        image::Rgb([
            ((u16::from(pixel[0]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8,
            ((u16::from(pixel[1]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8,
            ((u16::from(pixel[2]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8,
        ])
    })
}

fn read_orientation(path: &Path) -> u32 {
    let Ok(file) = File::open(path) else { return 1 };
    let mut reader = BufReader::new(file);
    Reader::new()
        .read_from_container(&mut reader)
        .ok()
        .and_then(|exif| {
            exif.get_field(Tag::Orientation, In::PRIMARY)
                .and_then(|field| field.value.get_uint(0))
        })
        .unwrap_or(1)
}

fn apply_orientation(image: DynamicImage, orientation: u32) -> DynamicImage {
    match orientation {
        2 => image.fliph(),
        3 => image.rotate180(),
        4 => image.flipv(),
        5 => image.rotate90().fliph(),
        6 => image.rotate90(),
        7 => image.rotate270().fliph(),
        8 => image.rotate270(),
        _ => image,
    }
}

fn is_animated_webp(path: &Path, format: InputFormat) -> Result<bool, ConversionError> {
    if format != InputFormat::WebP {
        return Ok(false);
    }
    let bytes = fs::read(path)?;
    Ok(bytes.windows(4).any(|window| window == b"ANIM"))
}
