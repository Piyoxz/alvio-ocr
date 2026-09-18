use crate::types::TextRegion;
use image::{DynamicImage, Rgb, RgbImage};
use imageproc::geometric_transformations::{rotate_about_center, Interpolation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PageOrientation {
    #[default]
    Upright,
    Rotate90,
    Rotate180,
    Rotate270,
}

impl PageOrientation {
    pub fn angle_degrees(&self) -> f32 {
        match self {
            Self::Upright => 0.0,
            Self::Rotate90 => 90.0,
            Self::Rotate180 => 180.0,
            Self::Rotate270 => 270.0,
        }
    }
}

pub fn rotate_image_steps(img: &DynamicImage, steps: u8) -> DynamicImage {
    match steps % 4 {
        1 => img.rotate90(),
        2 => img.rotate180(),
        3 => img.rotate270(),
        _ => img.clone(),
    }
}

pub fn detect_skew_angle(regions: &[TextRegion]) -> f32 {
    if regions.is_empty() {
        return 0.0;
    }

    let mut angle_sum = 0.0f32;
    let mut count = 0usize;

    for r in regions {
        let pts = &r.polygon;
        if pts.len() >= 4 {
            let dx = pts[1].x - pts[0].x;
            let dy = pts[1].y - pts[0].y;
            let len = (dx * dx + dy * dy).sqrt();

            if len > 20.0 {
                let angle_rad = dy.atan2(dx);
                let angle_deg = angle_rad.to_degrees();

                if angle_deg.abs() < 45.0 {
                    angle_sum += angle_deg;
                    count += 1;
                }
            }
        }
    }

    if count > 0 {
        angle_sum / count as f32
    } else {
        0.0
    }
}

pub fn deskew_image(img: &DynamicImage, angle_deg: f32) -> DynamicImage {
    if angle_deg.abs() < 0.5 {
        return img.clone();
    }

    let rad = -angle_deg.to_radians();
    let rgb = img.to_rgb8();
    let rotated = rotate_about_center(
        &rgb,
        rad,
        Interpolation::Bilinear,
        Rgb([255, 255, 255]),
    );
    DynamicImage::ImageRgb8(rotated)
}

pub fn deskew_rgb(rgb: &RgbImage, angle_deg: f32) -> RgbImage {
    if angle_deg.abs() < 0.5 {
        return rgb.clone();
    }

    let rad = -angle_deg.to_radians();
    rotate_about_center(
        rgb,
        rad,
        Interpolation::Bilinear,
        Rgb([255, 255, 255]),
    )
}

pub fn extract_oriented_crop(img: &DynamicImage, polygon: &[crate::types::Point]) -> Option<RgbImage> {
    if polygon.len() < 4 {
        return None;
    }

    let p0 = polygon[0];
    let p1 = polygon[1];
    let p2 = polygon[2];
    let p3 = polygon[3];

    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let angle_rad = dy.atan2(dx);
    let angle_deg = angle_rad.to_degrees();

    let width_top = (dx * dx + dy * dy).sqrt();
    let dx_bot = p2.x - p3.x;
    let dy_bot = p2.y - p3.y;
    let width_bot = (dx_bot * dx_bot + dy_bot * dy_bot).sqrt();
    let crop_w = width_top.max(width_bot).round() as u32;

    let dy_l = p3.y - p0.y;
    let dx_l = p3.x - p0.x;
    let height_left = (dx_l * dx_l + dy_l * dy_l).sqrt();
    let dy_r = p2.y - p1.y;
    let dx_r = p2.x - p1.x;
    let height_right = (dx_r * dx_r + dy_r * dy_r).sqrt();
    let crop_h = height_left.max(height_right).round() as u32;

    if crop_w < 3 || crop_h < 3 {
        return None;
    }

    if angle_deg.abs() < 1.5 {
        let min_x = p0.x.min(p1.x).min(p2.x).min(p3.x).max(0.0) as u32;
        let min_y = p0.y.min(p1.y).min(p2.y).min(p3.y).max(0.0) as u32;
        let max_x = (p0.x.max(p1.x).max(p2.x).max(p3.x) as u32).min(img.width());
        let max_y = (p0.y.max(p1.y).max(p2.y).max(p3.y) as u32).min(img.height());
        let w = max_x.saturating_sub(min_x);
        let h = max_y.saturating_sub(min_y);
        if w < 3 || h < 3 {
            return None;
        }
        return Some(img.crop_imm(min_x, min_y, w, h).to_rgb8());
    }

    let rgb = img.to_rgb8();
    let (src_w, src_h) = (rgb.width() as f32, rgb.height() as f32);
    let mut out = RgbImage::new(crop_w, crop_h);

    let target_w_f = crop_w as f32;
    let target_h_f = crop_h as f32;

    for y in 0..crop_h {
        let v = (y as f32 + 0.5) / target_h_f;
        for x in 0..crop_w {
            let u = (x as f32 + 0.5) / target_w_f;

            let sx = (1.0 - u) * (1.0 - v) * p0.x
                + u * (1.0 - v) * p1.x
                + u * v * p2.x
                + (1.0 - u) * v * p3.x;

            let sy = (1.0 - u) * (1.0 - v) * p0.y
                + u * (1.0 - v) * p1.y
                + u * v * p2.y
                + (1.0 - u) * v * p3.y;

            if sx >= 0.0 && sx < src_w - 1.0 && sy >= 0.0 && sy < src_h - 1.0 {
                let x0 = sx.floor() as u32;
                let y0 = sy.floor() as u32;
                let x1 = x0 + 1;
                let y1 = y0 + 1;

                let fx = sx - x0 as f32;
                let fy = sy - y0 as f32;

                let p00 = rgb.get_pixel(x0, y0).0;
                let p10 = rgb.get_pixel(x1, y0).0;
                let p01 = rgb.get_pixel(x0, y1).0;
                let p11 = rgb.get_pixel(x1, y1).0;

                let mut res = [0u8; 3];
                for c in 0..3 {
                    let v0 = p00[c] as f32 * (1.0 - fx) + p10[c] as f32 * fx;
                    let v1 = p01[c] as f32 * (1.0 - fx) + p11[c] as f32 * fx;
                    res[c] = (v0 * (1.0 - fy) + v1 * fy).clamp(0.0, 255.0) as u8;
                }
                out.put_pixel(x, y, image::Rgb(res));
            } else {
                out.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
    }

    Some(out)
}
