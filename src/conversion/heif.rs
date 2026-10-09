use crate::conversion::errors::ConversionError;
use image::{DynamicImage, RgbaImage};
use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
use std::path::Path;

pub fn decode_heif(path: &Path) -> Result<DynamicImage, ConversionError> {
    let bytes = std::fs::read(path)?;
    let context = HeifContext::read_from_bytes(&bytes)
        .map_err(|error| ConversionError::Heif(error.to_string()))?;
    let handle = context
        .primary_image_handle()
        .map_err(|error| ConversionError::Heif(error.to_string()))?;
    let decoded = LibHeif::new()
        .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)
        .map_err(|error| ConversionError::Heif(error.to_string()))?;
    let plane = decoded
        .planes()
        .interleaved
        .ok_or_else(|| ConversionError::Heif("decoder returned no RGBA plane".into()))?;
    if plane.storage_bits_per_pixel != 32 {
        return Err(ConversionError::Heif(format!(
            "expected 32-bit RGBA pixels, got {} bits",
            plane.storage_bits_per_pixel
        )));
    }
    let width = usize::try_from(plane.width)
        .map_err(|_| ConversionError::Heif("image width is too large".into()))?;
    let height = usize::try_from(plane.height)
        .map_err(|_| ConversionError::Heif("image height is too large".into()))?;
    let row_bytes = width
        .checked_mul(4)
        .ok_or_else(|| ConversionError::Heif("image row size overflow".into()))?;
    if plane.stride < row_bytes {
        return Err(ConversionError::Heif(
            "decoder returned an invalid pixel stride".into(),
        ));
    }
    let capacity = row_bytes
        .checked_mul(height)
        .ok_or_else(|| ConversionError::Heif("image allocation size overflow".into()))?;
    let mut packed = Vec::with_capacity(capacity);
    for row in plane.data.chunks_exact(plane.stride).take(height) {
        packed.extend_from_slice(&row[..row_bytes]);
    }
    if packed.len() != capacity {
        return Err(ConversionError::Heif(
            "decoder returned incomplete pixel data".into(),
        ));
    }
    let image = RgbaImage::from_raw(plane.width, plane.height, packed)
        .ok_or_else(|| ConversionError::Heif("invalid decoded dimensions".into()))?;
    Ok(DynamicImage::ImageRgba8(image))
}

pub fn linked_libheif_version() -> [u8; 3] {
    LibHeif::new().version()
}
