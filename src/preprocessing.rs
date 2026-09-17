use fast_image_resize::{images::Image as FirImage, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, GenericImageView, RgbImage};
use ndarray::Array4;

const DET_MEAN_R: f32 = 0.485;
const DET_MEAN_G: f32 = 0.456;
const DET_MEAN_B: f32 = 0.406;
const DET_INV_STD_R: f32 = 1.0 / 0.229;
const DET_INV_STD_G: f32 = 1.0 / 0.224;
const DET_INV_STD_B: f32 = 1.0 / 0.225;
const INV_255: f32 = 1.0 / 255.0;
const INV_127_5: f32 = 1.0 / 127.5;

pub struct DetPreprocessed {
    pub tensor: Array4<f32>,
    pub target_w: u32,
    pub target_h: u32,
    pub scale_x: f32,
    pub scale_y: f32,
}

pub fn preprocess_detection(img: &DynamicImage, max_side_len: u32) -> DetPreprocessed {
    let (orig_w, orig_h) = img.dimensions();
    let max_side = orig_w.max(orig_h);

    let scale = if max_side > max_side_len {
        max_side_len as f32 / max_side as f32
    } else {
        1.0f32
    };

    let mut target_w = ((orig_w as f32 * scale) / 32.0).round() as u32 * 32;
    let mut target_h = ((orig_h as f32 * scale) / 32.0).round() as u32 * 32;
    target_w = target_w.max(32);
    target_h = target_h.max(32);

    let scale_x = orig_w as f32 / target_w as f32;
    let scale_y = orig_h as f32 / target_h as f32;

    let resized_rgb = fast_resize_rgb(img, target_w, target_h);
    let raw = resized_rgb.as_raw();

    let tw = target_w as usize;
    let th = target_h as usize;
    let total = tw * th;

    let mut data = vec![0.0f32; 3 * total];
    let (plane_r, rest) = data.split_at_mut(total);
    let (plane_g, plane_b) = rest.split_at_mut(total);

    let stride = tw * 3;
    for y in 0..th {
        let row_offset = y * stride;
        let plane_offset = y * tw;
        for x in 0..tw {
            let px = row_offset + x * 3;
            let idx = plane_offset + x;
            unsafe {
                let r = *raw.get_unchecked(px);
                let g = *raw.get_unchecked(px + 1);
                let b = *raw.get_unchecked(px + 2);
                *plane_r.get_unchecked_mut(idx) = (r as f32 * INV_255 - DET_MEAN_R) * DET_INV_STD_R;
                *plane_g.get_unchecked_mut(idx) = (g as f32 * INV_255 - DET_MEAN_G) * DET_INV_STD_G;
                *plane_b.get_unchecked_mut(idx) = (b as f32 * INV_255 - DET_MEAN_B) * DET_INV_STD_B;
            }
        }
    }

    let tensor =
        Array4::from_shape_vec((1, 3, th, tw), data).expect("shape mismatch in det preprocessing");

    DetPreprocessed {
        tensor,
        target_w,
        target_h,
        scale_x,
        scale_y,
    }
}

pub fn preprocess_recognition(crop: &RgbImage) -> Array4<f32> {
    let (crop_w, crop_h) = crop.dimensions();
    let target_h = 48u32;
    let ratio = crop_w as f32 / crop_h.max(1) as f32;
    let target_w = ((target_h as f32 * ratio).round() as u32).clamp(16, 320);

    let resized = fast_resize_rgb_from_raw(crop, target_w, target_h);
    let raw = resized.as_raw();

    let tw = target_w as usize;
    let th = target_h as usize;
    let total = tw * th;

    let mut data = vec![0.0f32; 3 * total];
    let (plane_r, rest) = data.split_at_mut(total);
    let (plane_g, plane_b) = rest.split_at_mut(total);

    let stride = tw * 3;
    for y in 0..th {
        let row_offset = y * stride;
        let plane_offset = y * tw;
        for x in 0..tw {
            let px = row_offset + x * 3;
            let idx = plane_offset + x;
            unsafe {
                let r = *raw.get_unchecked(px);
                let g = *raw.get_unchecked(px + 1);
                let b = *raw.get_unchecked(px + 2);
                *plane_r.get_unchecked_mut(idx) = r as f32 * INV_127_5 - 1.0;
                *plane_g.get_unchecked_mut(idx) = g as f32 * INV_127_5 - 1.0;
                *plane_b.get_unchecked_mut(idx) = b as f32 * INV_127_5 - 1.0;
            }
        }
    }

    Array4::from_shape_vec((1, 3, th, tw), data).expect("shape mismatch in rec preprocessing")
}

pub fn preprocess_recognition_batch(crops: &[RgbImage]) -> (Array4<f32>, Vec<u32>) {
    if crops.is_empty() {
        return (
            Array4::zeros((0, 3, 48, 16)),
            Vec::new(),
        );
    }

    let target_h = 48u32;

    let target_widths: Vec<u32> = crops
        .iter()
        .map(|crop| {
            let (cw, ch) = crop.dimensions();
            let ratio = cw as f32 / ch.max(1) as f32;
            ((target_h as f32 * ratio).round() as u32).clamp(16, 320)
        })
        .collect();

    let max_w = *target_widths.iter().max().unwrap_or(&16) as usize;
    let batch_size = crops.len();
    let th = target_h as usize;
    let total_per_image = max_w * th;

    let mut data = vec![0.0f32; batch_size * 3 * total_per_image];

    for (b, (crop, &tw)) in crops.iter().zip(target_widths.iter()).enumerate() {
        let tw_usize = tw as usize;
        let resized = fast_resize_rgb_from_raw(crop, tw, target_h);
        let raw = resized.as_raw();
        let stride = tw_usize * 3;

        let batch_offset = b * 3 * total_per_image;
        let r_offset = batch_offset;
        let g_offset = batch_offset + total_per_image;
        let b_offset = batch_offset + 2 * total_per_image;

        for y in 0..th {
            let row_src = y * stride;
            let row_dst = y * max_w;
            for x in 0..tw_usize {
                let px = row_src + x * 3;
                let idx = row_dst + x;
                unsafe {
                    let rv = *raw.get_unchecked(px);
                    let gv = *raw.get_unchecked(px + 1);
                    let bv = *raw.get_unchecked(px + 2);
                    *data.get_unchecked_mut(r_offset + idx) = rv as f32 * INV_127_5 - 1.0;
                    *data.get_unchecked_mut(g_offset + idx) = gv as f32 * INV_127_5 - 1.0;
                    *data.get_unchecked_mut(b_offset + idx) = bv as f32 * INV_127_5 - 1.0;
                }
            }
        }
    }

    let tensor = Array4::from_shape_vec((batch_size, 3, th, max_w), data)
        .expect("shape mismatch in batch rec preprocessing");

    (tensor, target_widths)
}

fn fast_resize_rgb(img: &DynamicImage, target_w: u32, target_h: u32) -> RgbImage {
    let rgb = img.to_rgb8();
    fast_resize_rgb_from_raw(&rgb, target_w, target_h)
}

fn fast_resize_rgb_from_raw(rgb: &RgbImage, target_w: u32, target_h: u32) -> RgbImage {
    let (src_w, src_h) = rgb.dimensions();

    if src_w == target_w && src_h == target_h {
        return rgb.clone();
    }

    let mut src_bytes = rgb.as_raw().clone();

    let src_image =
        FirImage::from_slice_u8(src_w, src_h, &mut src_bytes, PixelType::U8x3)
            .expect("failed to create source FIR image");

    let mut dst_image = FirImage::new(target_w, target_h, PixelType::U8x3);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(
        fast_image_resize::FilterType::Bilinear,
    ));
    resizer
        .resize(&src_image, &mut dst_image, Some(&options))
        .expect("resize failed");

    let dst_raw = dst_image.into_vec();
    RgbImage::from_raw(target_w, target_h, dst_raw).expect("failed to create output RgbImage")
}
