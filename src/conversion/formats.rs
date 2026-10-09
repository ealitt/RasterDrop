use crate::conversion::errors::ConversionError;
use crate::models::{InputFormat, OutputFormat};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn detect_file(path: &Path) -> Result<InputFormat, ConversionError> {
    let mut file = File::open(path)?;
    let mut header = [0_u8; 64];
    let count = file.read(&mut header)?;
    if count == 0 {
        return Err(ConversionError::UnsupportedFormat);
    }
    detect_bytes(&header[..count])
}

pub fn detect_bytes(bytes: &[u8]) -> Result<InputFormat, ConversionError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(InputFormat::Png);
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Ok(InputFormat::Jpeg);
    }
    if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        return Ok(InputFormat::Tiff);
    }
    if bytes.starts_with(b"BM") {
        return Ok(InputFormat::Bmp);
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok(InputFormat::WebP);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Ok(InputFormat::Gif);
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brands = &bytes[8..];
        if brands
            .chunks(4)
            .any(|brand| matches!(brand, b"avif" | b"avis"))
        {
            return Ok(InputFormat::Avif);
        }
        if brands.chunks(4).any(|brand| {
            matches!(
                brand,
                b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"mif1" | b"msf1"
            )
        }) {
            return Ok(InputFormat::Heif);
        }
    }
    Err(ConversionError::UnsupportedFormat)
}

pub const fn same_format(input: InputFormat, output: OutputFormat) -> bool {
    matches!(
        (input, output),
        (InputFormat::Jpeg, OutputFormat::Jpeg)
            | (InputFormat::Png, OutputFormat::Png)
            | (InputFormat::WebP, OutputFormat::WebP)
            | (InputFormat::Tiff, OutputFormat::Tiff)
            | (InputFormat::Bmp, OutputFormat::Bmp)
            | (InputFormat::Avif, OutputFormat::Avif)
    )
}

pub const fn expected_extensions(format: InputFormat) -> &'static [&'static str] {
    match format {
        InputFormat::Heif => &["heic", "heif"],
        InputFormat::Jpeg => &["jpg", "jpeg"],
        InputFormat::Png => &["png"],
        InputFormat::WebP => &["webp"],
        InputFormat::Tiff => &["tif", "tiff"],
        InputFormat::Bmp => &["bmp"],
        InputFormat::Avif => &["avif"],
        InputFormat::Gif => &["gif"],
    }
}
