use image::{Rgba, RgbaImage};
use qrcode::QrCode;

use crate::models::QrErrorCorrection;

/// A realistic ~220-byte sample payload for preview QR codes, matching the
/// typical size of an ecash note so preview density reflects the real output.
pub const SAMPLE_QR_DATA: &str = "fed11qgqzc2nhwden5te0vejkg6tdd9h8gepwvejkg6tdd9h8garhduhx6ct5d9hxgmmjv9kx7pqdq4ux6t5vdhk6m0d4hjqfurvx6z5rn8s28fc2fnd40krnh34t3scp7lypl5y6r7x6qlzcgpaq3aq8vqdmqmv2c3tkjqp50phwtsyvfwn5ylh9uz26a9tjnr0abcdefghijklmnopqrstuvwx";

/// Generate a QR code as PNG bytes with transparent background.
/// Generate a QR code as PNG bytes with transparent background (for PDF overlay).
pub fn generate_qr_png(
    data: &str,
    ec_level: QrErrorCorrection,
    module_size: u32,
) -> anyhow::Result<Vec<u8>> {
    generate_qr_png_inner(data, ec_level, module_size, Rgba([0, 0, 0, 0]))
}

/// Generate a QR code as PNG bytes with white background (for display).
pub fn generate_qr_png_white(
    data: &str,
    ec_level: QrErrorCorrection,
    module_size: u32,
) -> anyhow::Result<Vec<u8>> {
    generate_qr_png_inner(data, ec_level, module_size, Rgba([255, 255, 255, 255]))
}

fn generate_qr_png_inner(
    data: &str,
    ec_level: QrErrorCorrection,
    module_size: u32,
    bg: Rgba<u8>,
) -> anyhow::Result<Vec<u8>> {
    let code = QrCode::with_error_correction_level(data, ec_level.to_qrcode_ec())?;
    let modules = code.to_colors();
    let width = code.width() as u32;
    let quiet_zone = 1u32;
    let img_size = (width + 2 * quiet_zone) * module_size;

    let mut img = RgbaImage::from_pixel(img_size, img_size, bg);

    for (y, row) in modules.chunks(width as usize).enumerate() {
        for (x, &color) in row.iter().enumerate() {
            if color == qrcode::Color::Dark {
                let px = (x as u32 + quiet_zone) * module_size;
                let py = (y as u32 + quiet_zone) * module_size;
                for dy in 0..module_size {
                    for dx in 0..module_size {
                        img.put_pixel(px + dx, py + dy, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
    }

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        img_size,
        img_size,
        image::ExtendedColorType::Rgba8,
    )?;

    Ok(png_bytes)
}

/// Overlay an icon image in the center of a QR code image.
/// Both input and output are PNG bytes.
pub fn overlay_icon(qr_png: &[u8], icon_png: &[u8], icon_size_percent: u32) -> anyhow::Result<Vec<u8>> {
    let qr_img = image::load_from_memory_with_format(qr_png, image::ImageFormat::Png)?
        .into_rgba8();
    let icon_img = image::load_from_memory_with_format(icon_png, image::ImageFormat::Png)?
        .into_rgba8();

    let qr_w = qr_img.width();
    let qr_h = qr_img.height();
    let icon_target = qr_w * icon_size_percent / 100;

    let resized = image::imageops::resize(
        &icon_img,
        icon_target,
        icon_target,
        image::imageops::FilterType::Lanczos3,
    );

    let mut result = qr_img;
    let offset_x = (qr_w - icon_target) / 2;
    let offset_y = (qr_h - icon_target) / 2;

    image::imageops::overlay(&mut result, &resized, offset_x as i64, offset_y as i64);

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    image::ImageEncoder::write_image(
        encoder,
        result.as_raw(),
        result.width(),
        result.height(),
        image::ExtendedColorType::Rgba8,
    )?;

    Ok(png_bytes)
}
