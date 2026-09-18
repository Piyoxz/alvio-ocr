use image::{DynamicImage, GrayImage, ImageBuffer, Rgb, RgbImage};

pub fn compute_variance(gray: &GrayImage) -> f32 {
    let raw = gray.as_raw();
    let total = raw.len() as f64;
    if total == 0.0 {
        return 0.0;
    }

    let mut sum = 0u64;
    let mut sum_sq = 0u64;

    for chunk in raw.chunks_exact(4) {
        let a = chunk[0] as u64;
        let b = chunk[1] as u64;
        let c = chunk[2] as u64;
        let d = chunk[3] as u64;
        sum += a + b + c + d;
        sum_sq += a * a + b * b + c * c + d * d;
    }

    for &v in raw.chunks_exact(4).remainder() {
        let fv = v as u64;
        sum += fv;
        sum_sq += fv * fv;
    }

    let mean = sum as f64 / total;
    let variance = (sum_sq as f64 / total) - (mean * mean);
    variance as f32
}

fn auto_contrast(img: &RgbImage) -> RgbImage {
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

    let range = global_max.saturating_sub(global_min);
    if range >= 200 || range == 0 {
        return img.clone();
    }

    let mut lut = [0u8; 256];
    let min_f = global_min as f32;
    let scale = 255.0 / range as f32;
    for i in 0..256 {
        lut[i] = ((i as f32 - min_f) * scale).clamp(0.0, 255.0) as u8;
    }

    ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
        let Rgb([r, g, b]) = *img.get_pixel(x, y);
        Rgb([lut[r as usize], lut[g as usize], lut[b as usize]])
    })
}

fn box_blur_3x3(gray: &GrayImage) -> GrayImage {
    let (w, h) = gray.dimensions();
    if w <= 1 || h <= 1 {
        return gray.clone();
    }
    let wu = w as usize;
    let hu = h as usize;
    let src = gray.as_raw();

    let mut temp = vec![0u16; wu * hu];
    for y in 0..hu {
        let row = y * wu;
        for x in 0..wu {
            let x0 = if x > 0 { x - 1 } else { 0 };
            let x2 = if x + 1 < wu { x + 1 } else { wu - 1 };
            unsafe {
                temp[row + x] = *src.get_unchecked(row + x0) as u16
                    + *src.get_unchecked(row + x) as u16
                    + *src.get_unchecked(row + x2) as u16;
            }
        }
    }

    let mut out = GrayImage::new(w, h);
    let dst = out.as_mut();
    for y in 0..hu {
        let y0 = if y > 0 { y - 1 } else { 0 };
        let y2 = if y + 1 < hu { y + 1 } else { hu - 1 };
        for x in 0..wu {
            let idx = y * wu + x;
            let sum = unsafe {
                *temp.get_unchecked(y0 * wu + x)
                    + *temp.get_unchecked(y * wu + x)
                    + *temp.get_unchecked(y2 * wu + x)
            };
            dst[idx] = (sum / 9) as u8;
        }
    }

    out
}

fn unsharp_mask(img: &RgbImage, strength: f32) -> RgbImage {
    let gray = DynamicImage::ImageRgb8(img.clone()).to_luma8();
    let blurred = box_blur_3x3(&gray);
    let inv_128 = strength / 128.0;

    ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
        let Rgb([r, g, b]) = *img.get_pixel(x, y);
        let orig_luma = gray.get_pixel(x, y).0[0] as f32;
        let blur_luma = blurred.get_pixel(x, y).0[0] as f32;
        let factor = 1.0 + (orig_luma - blur_luma) * inv_128;

        let nr = (r as f32 * factor).clamp(0.0, 255.0) as u8;
        let ng = (g as f32 * factor).clamp(0.0, 255.0) as u8;
        let nb = (b as f32 * factor).clamp(0.0, 255.0) as u8;
        Rgb([nr, ng, nb])
    })
}

pub fn remove_shadows(img: &RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    if w < 16 || h < 16 {
        return img.clone();
    }

    let gray = DynamicImage::ImageRgb8(img.clone()).to_luma8();
    let dw = (w / 8).max(4);
    let dh = (h / 8).max(4);

    let down_gray = image::imageops::resize(&gray, dw, dh, image::imageops::FilterType::Triangle);
    let bg_down = box_blur_3x3(&box_blur_3x3(&down_gray));
    let bg = image::imageops::resize(&bg_down, w, h, image::imageops::FilterType::Triangle);

    ImageBuffer::from_fn(w, h, |x, y| {
        let Rgb([r, g, b]) = *img.get_pixel(x, y);
        let bg_val = bg.get_pixel(x, y).0[0].max(10) as f32;
        let factor = 220.0 / bg_val;

        let nr = ((r as f32) * factor).clamp(0.0, 255.0) as u8;
        let ng = ((g as f32) * factor).clamp(0.0, 255.0) as u8;
        let nb = ((b as f32) * factor).clamp(0.0, 255.0) as u8;
        Rgb([nr, ng, nb])
    })
}

pub fn adaptive_threshold(gray: &GrayImage, c_offset: i32) -> GrayImage {
    let (w, h) = gray.dimensions();
    let blurred = box_blur_3x3(gray);

    ImageBuffer::from_fn(w, h, |x, y| {
        let val = gray.get_pixel(x, y).0[0] as i32;
        let mean = blurred.get_pixel(x, y).0[0] as i32;
        if val > mean - c_offset {
            image::Luma([255u8])
        } else {
            image::Luma([0u8])
        }
    })
}

use std::borrow::Cow;

pub fn enhance_if_needed<'a>(img: &'a DynamicImage, variance_threshold: f32) -> Cow<'a, DynamicImage> {
    let gray = img.to_luma8();
    let variance = compute_variance(&gray);

    if variance >= variance_threshold {
        return Cow::Borrowed(img);
    }

    tracing::debug!(
        "Low image quality detected (variance={:.0}, threshold={:.0}). Applying enhancement.",
        variance,
        variance_threshold
    );

    let rgb = img.to_rgb8();
    let contrasted = auto_contrast(&rgb);
    let sharpened = unsharp_mask(&contrasted, 0.8);

    Cow::Owned(DynamicImage::ImageRgb8(sharpened))
}
