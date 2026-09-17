//! Adaptive image enhancement for low-quality inputs.
//!
//! Applies auto-contrast stretching and unsharp masking only when the image
//! quality is detected as low (measured by pixel variance). High-quality images
//! pass through untouched — zero cost.

use image::{DynamicImage, GrayImage, ImageBuffer, Luma, Rgb, RgbImage};

/// Compute the variance of grayscale pixel values. Low variance = low contrast.
pub fn compute_variance(gray: &GrayImage) -> f32 {
    let (w, h) = gray.dimensions();
    let total = (w as f64) * (h as f64);
    if total == 0.0 {
        return 0.0;
    }

    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;

    for &Luma([v]) in gray.pixels() {
        let fv = v as f64;
        sum += fv;
        sum_sq += fv * fv;
    }

    let mean = sum / total;
    let variance = (sum_sq / total) - (mean * mean);
    variance as f32
}

/// Apply histogram stretching (auto-contrast) to an RGB image.
///
/// Maps the actual min/max luminance range to the full 0–255 range,
/// improving visibility of text in dark or washed-out images.
fn auto_contrast(img: &RgbImage) -> RgbImage {
    // Find global min/max across all channels
    let mut global_min = 255u8;
    let mut global_max = 0u8;

    for Rgb([r, g, b]) in img.pixels() {
        let luma = (((*r as u16) * 77 + (*g as u16) * 150 + (*b as u16) * 29) >> 8) as u8;
        if luma < global_min {
            global_min = luma;
        }
        if luma > global_max {
            global_max = luma;
        }
    }

    // If already full range or nearly flat, return clone
    let range = global_max.saturating_sub(global_min);
    if range >= 200 || range == 0 {
        return img.clone();
    }

    let min_f = global_min as f32;
    let scale = 255.0 / range as f32;

    ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
        let Rgb([r, g, b]) = *img.get_pixel(x, y);
        let nr = ((r as f32 - min_f) * scale).clamp(0.0, 255.0) as u8;
        let ng = ((g as f32 - min_f) * scale).clamp(0.0, 255.0) as u8;
        let nb = ((b as f32 - min_f) * scale).clamp(0.0, 255.0) as u8;
        Rgb([nr, ng, nb])
    })
}

/// Apply a simple 3x3 box blur to a grayscale image (used for unsharp mask).
fn box_blur_3x3(gray: &GrayImage) -> GrayImage {
    let (w, h) = gray.dimensions();
    let mut out = GrayImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u32;
            let mut count = 0u32;

            for dy in 0..3i32 {
                for dx in 0..3i32 {
                    let nx = x as i32 + dx - 1;
                    let ny = y as i32 + dy - 1;
                    if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                        sum += gray.get_pixel(nx as u32, ny as u32).0[0] as u32;
                        count += 1;
                    }
                }
            }

            out.put_pixel(x, y, Luma([(sum / count) as u8]));
        }
    }

    out
}

/// Apply unsharp masking to sharpen text in an RGB image.
///
/// Uses a 3x3 box blur as the base, then amplifies the difference from the original.
/// Strength controls the sharpening amount (typical: 0.5–1.5).
fn unsharp_mask(img: &RgbImage, strength: f32) -> RgbImage {
    let gray = DynamicImage::ImageRgb8(img.clone()).to_luma8();
    let blurred = box_blur_3x3(&gray);

    ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
        let Rgb([r, g, b]) = *img.get_pixel(x, y);
        let orig_luma = gray.get_pixel(x, y).0[0] as f32;
        let blur_luma = blurred.get_pixel(x, y).0[0] as f32;
        let diff = orig_luma - blur_luma;
        let factor = 1.0 + strength * diff / 128.0;

        let nr = (r as f32 * factor).clamp(0.0, 255.0) as u8;
        let ng = (g as f32 * factor).clamp(0.0, 255.0) as u8;
        let nb = (b as f32 * factor).clamp(0.0, 255.0) as u8;
        Rgb([nr, ng, nb])
    })
}

/// Apply adaptive enhancement to an image if it appears to be low quality.
///
/// Returns the enhanced image if quality is below the variance threshold,
/// or the original image unchanged if quality is already good.
///
/// # Arguments
/// * `img` — input image
/// * `variance_threshold` — variance below which enhancement kicks in (typical: 1500.0)
pub fn enhance_if_needed(img: &DynamicImage, variance_threshold: f32) -> DynamicImage {
    let gray = img.to_luma8();
    let variance = compute_variance(&gray);

    if variance >= variance_threshold {
        // Image quality is fine, no enhancement needed
        return img.clone();
    }

    tracing::debug!(
        "Low image quality detected (variance={:.0}, threshold={:.0}). Applying enhancement.",
        variance,
        variance_threshold
    );

    let rgb = img.to_rgb8();
    let contrasted = auto_contrast(&rgb);
    let sharpened = unsharp_mask(&contrasted, 0.8);

    DynamicImage::ImageRgb8(sharpened)
}
