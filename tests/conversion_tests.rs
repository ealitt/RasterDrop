use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use rasterdrop::conversion::formats::{detect_bytes, detect_file, same_format};
use rasterdrop::conversion::worker::{
    ConversionOutcome, ConversionRequest, composite_alpha_on_white, convert_file,
};
use rasterdrop::models::{CollisionPolicy, InputFormat, OutputFormat, OutputLocation};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tempfile::tempdir;

fn request(source: &std::path::Path, format: OutputFormat) -> ConversionRequest {
    ConversionRequest {
        source: source.to_path_buf(),
        output_format: format,
        output_location: OutputLocation::BesideSource,
        collision_policy: CollisionPolicy::CreateNumberedFile,
        jpeg_quality: 90,
    }
}

#[test]
fn detects_supported_content_signatures_and_format_families() {
    let cases: &[(&[u8], InputFormat)] = &[
        (b"\x89PNG\r\n\x1a\n", InputFormat::Png),
        (b"\xff\xd8\xff\xe0", InputFormat::Jpeg),
        (b"II*\0", InputFormat::Tiff),
        (b"MM\0*", InputFormat::Tiff),
        (b"BMrest", InputFormat::Bmp),
        (b"RIFF\x08\0\0\0WEBP", InputFormat::WebP),
        (b"\0\0\0\x18ftypavif", InputFormat::Avif),
        (b"\0\0\0\x18ftypheic", InputFormat::Heif),
    ];
    for (bytes, expected) in cases {
        assert_eq!(detect_bytes(bytes).unwrap(), *expected);
    }
    assert!(same_format(InputFormat::Jpeg, OutputFormat::Jpeg));
    assert!(same_format(InputFormat::Tiff, OutputFormat::Tiff));
    assert!(!same_format(InputFormat::Heif, OutputFormat::Avif));
}

#[test]
fn png_to_jpeg_composites_transparency_on_white_and_preserves_source() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("alpha.png");
    let pixels = ImageBuffer::from_fn(4, 3, |x, _| {
        if x == 0 {
            Rgba([255, 0, 0, 0])
        } else {
            Rgba([10, 20, 30, 255])
        }
    });
    DynamicImage::ImageRgba8(pixels).save(&source).unwrap();
    let before = std::fs::read(&source).unwrap();

    let outcome = convert_file(
        &request(&source, OutputFormat::Jpeg),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let output = match outcome {
        ConversionOutcome::Completed(path) => path,
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(detect_file(&output).unwrap(), InputFormat::Jpeg);
    let converted = image::open(output).unwrap();
    assert_eq!(converted.dimensions(), (4, 3));
    let white = converted.to_rgb8().get_pixel(0, 0).0;
    assert!(white.iter().all(|channel| *channel > 230), "{white:?}");
    assert_eq!(std::fs::read(&source).unwrap(), before);
}

#[test]
fn same_format_is_skipped_without_rewriting() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("wrong-extension.jpg");
    DynamicImage::new_rgb8(2, 2)
        .save_with_format(&source, image::ImageFormat::Png)
        .unwrap();
    let before = std::fs::metadata(&source).unwrap().modified().unwrap();
    let outcome = convert_file(
        &request(&source, OutputFormat::Png),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    assert!(matches!(
        outcome,
        ConversionOutcome::Skipped {
            format: InputFormat::Png,
            extension_mismatch: true
        }
    ));
    assert_eq!(
        std::fs::metadata(&source).unwrap().modified().unwrap(),
        before
    );
    assert!(!dir.path().join("wrong-extension.png").exists());
}

#[test]
fn standard_formats_convert_by_content() {
    let dir = tempdir().unwrap();
    for (input, extension, output, expected) in [
        (
            image::ImageFormat::Jpeg,
            "jpg",
            OutputFormat::Png,
            InputFormat::Png,
        ),
        (
            image::ImageFormat::Png,
            "png",
            OutputFormat::WebP,
            InputFormat::WebP,
        ),
        (
            image::ImageFormat::Tiff,
            "tif",
            OutputFormat::Jpeg,
            InputFormat::Jpeg,
        ),
        (
            image::ImageFormat::WebP,
            "webp",
            OutputFormat::Png,
            InputFormat::Png,
        ),
        (
            image::ImageFormat::Png,
            "source",
            OutputFormat::Avif,
            InputFormat::Avif,
        ),
        (
            image::ImageFormat::Png,
            "other",
            OutputFormat::Bmp,
            InputFormat::Bmp,
        ),
    ] {
        let source = dir.path().join(format!("input-{extension}.{extension}"));
        DynamicImage::new_rgba8(3, 2)
            .save_with_format(&source, input)
            .unwrap();
        let outcome =
            convert_file(&request(&source, output), Arc::new(AtomicBool::new(false))).unwrap();
        let path = match outcome {
            ConversionOutcome::Completed(path) => path,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(detect_file(&path).unwrap(), expected);
        let decoded = if expected == InputFormat::Avif {
            rasterdrop::conversion::heif::decode_heif(&path).unwrap()
        } else {
            image::open(path).unwrap()
        };
        assert_eq!(decoded.dimensions(), (3, 2));
    }
}

#[test]
fn cancellation_and_failures_leave_no_temporary_files() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("bad.png");
    std::fs::write(&source, b"not an image").unwrap();
    assert!(
        convert_file(
            &request(&source, OutputFormat::Jpeg),
            Arc::new(AtomicBool::new(false))
        )
        .is_err()
    );
    assert!(std::fs::read_dir(dir.path()).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".rasterdrop-")
    }));

    let valid = dir.path().join("valid.png");
    DynamicImage::new_rgb8(2, 2).save(&valid).unwrap();
    let cancelled = Arc::new(AtomicBool::new(true));
    assert!(convert_file(&request(&valid, OutputFormat::Jpeg), cancelled).is_err());
}

#[test]
fn alpha_compositing_math_uses_white_background() {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(1, 1, Rgba([255, 0, 0, 128])));
    let rgb = composite_alpha_on_white(&image);
    let pixel = rgb.get_pixel(0, 0).0;
    assert!(pixel[0] >= 254);
    assert!((126..=129).contains(&pixel[1]));
    assert!((126..=129).contains(&pixel[2]));
}

#[test]
fn heic_fixture_converts_to_jpeg_and_png_with_container_transformations() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("representative.heic");
    std::fs::copy("tests/fixtures/representative.heic", &source).unwrap();
    let original = std::fs::read(&source).unwrap();

    for output_format in [OutputFormat::Jpeg, OutputFormat::Png] {
        let outcome = convert_file(
            &request(&source, output_format),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        let output = match outcome {
            ConversionOutcome::Completed(path) => path,
            other => panic!("unexpected {other:?}"),
        };
        let decoded = image::open(&output).unwrap();
        assert_eq!(decoded.dimensions(), (1652, 1791));
        assert_eq!(
            detect_file(&output).unwrap(),
            if output_format == OutputFormat::Jpeg {
                InputFormat::Jpeg
            } else {
                InputFormat::Png
            }
        );
    }
    assert_eq!(std::fs::read(source).unwrap(), original);
}

fn orientation_exif(orientation: u8) -> Vec<u8> {
    vec![
        b'E',
        b'x',
        b'i',
        b'f',
        0,
        0,
        b'I',
        b'I',
        42,
        0,
        8,
        0,
        0,
        0,
        1,
        0,
        0x12,
        0x01,
        3,
        0,
        1,
        0,
        0,
        0,
        orientation,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ]
}

#[test]
fn jpeg_exif_orientations_preserve_dimensions_and_pixel_positions() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("rotated.jpg");
    let pixels = image::RgbImage::from_fn(3, 2, |x, y| {
        image::Rgb([(x * 70) as u8, (y * 160) as u8, 80])
    });
    pixels.save(&source).unwrap();
    let jpeg = std::fs::read(&source).unwrap();
    let original = image::open(&source).unwrap().to_rgb8();
    // Expected source pixel indices in output row order for each EXIF value.
    let arrangements = [
        [0, 1, 2, 3, 4, 5],
        [2, 1, 0, 5, 4, 3],
        [5, 4, 3, 2, 1, 0],
        [3, 4, 5, 0, 1, 2],
        [0, 3, 1, 4, 2, 5],
        [3, 0, 4, 1, 5, 2],
        [5, 2, 4, 1, 3, 0],
        [2, 5, 1, 4, 0, 3],
    ];
    for orientation in 1..=8 {
        let payload = orientation_exif(orientation);
        let mut oriented = jpeg[..2].to_vec();
        oriented.extend_from_slice(&[0xff, 0xe1]);
        oriented.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        oriented.extend_from_slice(&payload);
        oriented.extend_from_slice(&jpeg[2..]);
        std::fs::write(&source, &oriented).unwrap();
        let ConversionOutcome::Completed(output) = convert_file(
            &request(&source, OutputFormat::Png),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap() else {
            panic!("conversion skipped")
        };
        let decoded = image::open(output).unwrap().to_rgb8();
        assert_eq!(
            decoded.dimensions(),
            if orientation >= 5 { (2, 3) } else { (3, 2) }
        );
        let expected: Vec<_> = arrangements[orientation as usize - 1]
            .iter()
            .map(|&i| *original.get_pixel(i % 3, i / 3))
            .collect();
        assert_eq!(
            decoded.pixels().copied().collect::<Vec<_>>(),
            expected,
            "EXIF {orientation}"
        );
        assert_eq!(std::fs::read(&source).unwrap(), oriented);
    }
}

#[test]
fn heif_and_avif_container_orientation_is_not_applied_twice() {
    let dir = tempdir().unwrap();
    for extension in ["heic", "avif"] {
        let source = dir.path().join(format!("portrait.{extension}"));
        std::fs::copy(
            format!("tests/fixtures/portrait-orientation.{extension}"),
            &source,
        )
        .unwrap();
        let original = std::fs::read(&source).unwrap();
        let exif = exif::Reader::new()
            .read_from_container(&mut std::io::BufReader::new(
                std::fs::File::open(&source).unwrap(),
            ))
            .unwrap();
        assert_eq!(
            exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
                .unwrap()
                .value
                .get_uint(0),
            Some(6)
        );
        for format in [
            OutputFormat::Jpeg,
            OutputFormat::Png,
            OutputFormat::WebP,
            OutputFormat::Tiff,
            OutputFormat::Bmp,
            OutputFormat::Avif,
        ] {
            if extension == "avif" && format == OutputFormat::Avif {
                continue;
            }
            let ConversionOutcome::Completed(output) =
                convert_file(&request(&source, format), Arc::new(AtomicBool::new(false))).unwrap()
            else {
                panic!("conversion skipped")
            };
            let decoded = if format == OutputFormat::Avif {
                rasterdrop::conversion::heif::decode_heif(&output).unwrap()
            } else {
                image::open(&output).unwrap()
            }
            .to_rgb8();
            assert_eq!(decoded.dimensions(), (48, 80), "{extension} to {format:?}");
            // Clockwise rotation puts blue/red above yellow/green.
            for (x, y, expected) in [
                (12, 20, [20u8, 20, 240]),
                (36, 20, [240, 20, 20]),
                (12, 60, [240, 240, 20]),
                (36, 60, [20, 240, 20]),
            ] {
                let actual = decoded.get_pixel(x, y).0;
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(&a, e)| a.abs_diff(e) < 35),
                    "{extension} to {format:?}: ({x}, {y}) = {actual:?}, expected {expected:?}"
                );
            }
        }
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
}

#[test]
fn jpeg_to_webp_uses_compact_lossy_encoding_with_good_visual_quality() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("photo.jpg");
    let photo = image::RgbImage::from_fn(640, 480, |x, y| {
        let detail = ((x.wrapping_mul(17) + y.wrapping_mul(31)) % 23) as u8;
        image::Rgb([
            ((x * 255 / 639) as u8).saturating_add(detail / 3),
            ((y * 255 / 479) as u8).saturating_add(detail / 4),
            (((x + y) * 255 / 1118) as u8).saturating_add(detail / 2),
        ])
    });
    let mut source_file = std::io::BufWriter::new(std::fs::File::create(&source).unwrap());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut source_file, 90)
        .encode_image(&DynamicImage::ImageRgb8(photo))
        .unwrap();
    drop(source_file);
    let source_size = std::fs::metadata(&source).unwrap().len();
    let decoded_source = image::open(&source).unwrap().to_rgb8();

    let output = match convert_file(
        &request(&source, OutputFormat::WebP),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
    {
        ConversionOutcome::Completed(path) => path,
        other => panic!("unexpected {other:?}"),
    };
    let output_size = std::fs::metadata(&output).unwrap().len();
    eprintln!("WebP regression sample: JPEG {source_size} bytes -> WebP {output_size} bytes");
    assert!(
        output_size <= source_size,
        "WebP should not exceed the JPEG: {output_size} > {source_size}"
    );

    let decoded_output = image::open(output).unwrap().to_rgb8();
    let total_error: u64 = decoded_source
        .pixels()
        .zip(decoded_output.pixels())
        .flat_map(|(left, right)| left.0.into_iter().zip(right.0))
        .map(|(left, right)| u64::from(left.abs_diff(right)))
        .sum();
    let channel_count = u64::from(decoded_source.width()) * u64::from(decoded_source.height()) * 3;
    let mean_absolute_error = total_error as f64 / channel_count as f64;
    assert!(
        mean_absolute_error < 10.0,
        "mean channel error was {mean_absolute_error:.2}"
    );
}
