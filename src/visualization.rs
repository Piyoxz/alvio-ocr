use crate::types::{OcrResult, TextRegion};
use image::{DynamicImage, Rgb, RgbImage};

pub fn annotate_image(img: &DynamicImage, result: &OcrResult) -> DynamicImage {
    let mut rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();

    for region in &result.regions {
        let conf = region.confidence;
        let color = if conf >= 0.90 {
            Rgb([46, 204, 113]) // Emerald Green
        } else if conf >= 0.75 {
            Rgb([52, 152, 219]) // Sky Blue
        } else if conf >= 0.50 {
            Rgb([243, 156, 18]) // Amber Orange
        } else {
            Rgb([231, 76, 60])  // Alizarin Red
        };

        draw_region_polygon(&mut rgb, &region.region, color, 2, w, h);
    }

    DynamicImage::ImageRgb8(rgb)
}

fn draw_region_polygon(
    img: &mut RgbImage,
    region: &TextRegion,
    color: Rgb<u8>,
    thickness: i32,
    img_w: u32,
    img_h: u32,
) {
    let pts = &region.polygon;
    if pts.len() < 2 {
        let (x1, y1, x2, y2) = region.aabb();
        draw_rect(img, x1, y1, x2, y2, color, thickness, img_w, img_h);
        return;
    }

    let n = pts.len();
    for i in 0..n {
        let p1 = pts[i];
        let p2 = pts[(i + 1) % n];
        draw_line(
            img,
            p1.x.round() as i32,
            p1.y.round() as i32,
            p2.x.round() as i32,
            p2.y.round() as i32,
            color,
            thickness,
            img_w,
            img_h,
        );
    }
}

fn draw_rect(
    img: &mut RgbImage,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    color: Rgb<u8>,
    thickness: i32,
    img_w: u32,
    img_h: u32,
) {
    let ix1 = x1.round() as i32;
    let iy1 = y1.round() as i32;
    let ix2 = x2.round() as i32;
    let iy2 = y2.round() as i32;

    draw_line(img, ix1, iy1, ix2, iy1, color, thickness, img_w, img_h);
    draw_line(img, ix2, iy1, ix2, iy2, color, thickness, img_w, img_h);
    draw_line(img, ix2, iy2, ix1, iy2, color, thickness, img_w, img_h);
    draw_line(img, ix1, iy2, ix1, iy1, color, thickness, img_w, img_h);
}

fn draw_line(
    img: &mut RgbImage,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: Rgb<u8>,
    thickness: i32,
    img_w: u32,
    img_h: u32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let half = thickness / 2;

    loop {
        for ox in -half..=half {
            for oy in -half..=half {
                let px = x0 + ox;
                let py = y0 + oy;
                if px >= 0 && px < img_w as i32 && py >= 0 && py < img_h as i32 {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}
