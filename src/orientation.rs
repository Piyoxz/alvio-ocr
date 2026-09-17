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
