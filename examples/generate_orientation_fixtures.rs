use libheif_rs::*;
fn main() -> Result<()> {
    let lib = LibHeif::new();
    for (format, ext) in [
        (CompressionFormat::Hevc, "heic"),
        (CompressionFormat::Av1, "avif"),
    ] {
        let mut image = Image::new(80, 48, ColorSpace::Rgb(RgbChroma::Rgb))?;
        image.create_plane(Channel::Interleaved, 80, 48, 8)?;
        let plane = image.planes_mut().interleaved.unwrap();
        for y in 0..48 {
            for x in 0..80 {
                let color = match (x < 40, y < 24) {
                    (true, true) => [240, 20, 20],
                    (false, true) => [20, 240, 20],
                    (true, false) => [20, 20, 240],
                    (false, false) => [240, 240, 20],
                };
                let offset = y * plane.stride + x * 3;
                plane.data[offset..offset + 3].copy_from_slice(&color);
            }
        }
        let mut context = HeifContext::new()?;
        let mut encoder = lib.encoder_for_format(format)?;
        encoder.set_quality(EncoderQuality::Lossy(95))?;
        let mut options = EncodingOptions::default();
        options.set_image_orientation(ImageOrientation::Rotate90Cw);
        let handle = context.encode_image(&image, &mut encoder, Some(options))?;
        context.add_exif_metadata(
            &handle,
            &[
                b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0,
                0, 0,
            ],
        )?;
        context.write_to_file(&format!("tests/fixtures/portrait-orientation.{ext}"))?;
    }
    Ok(())
}
