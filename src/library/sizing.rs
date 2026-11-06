//! A committed picture sized to its kind's sizes (§15.2): cut to the kind's
//! shape from its centre, scaled with Lanczos-3, and encoded as lossy WebP
//! by libwebp at [`QUALITY`]. The image model's flat art carries a fine
//! grain that lossless coding keeps at nearly a PNG's size; lossy coding
//! makes a 512 tile a fraction of that with no visible change. WebP keeps a
//! sticker's transparency, so one format serves every kind.

use anyhow::{anyhow, Context, Result};
use image::imageops::{self, FilterType};
use image::{ImageFormat, RgbaImage};
use webp::Encoder;

/// libwebp's quality, 0 to 100.
pub const QUALITY: f32 = 82.0;

/// The picture `png`, decoded.
pub fn decode(png: &[u8]) -> Result<RgbaImage> {
    Ok(image::load_from_memory_with_format(png, ImageFormat::Png)
        .context("not a PNG")?
        .to_rgba8())
}

/// The centre of `img` with the shape `w`×`h`.
pub fn cut(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let (sw, sh) = img.dimensions();
    // Compare the shapes in integers: sw/sh against w/h.
    let (cw, ch) = if u64::from(sw) * u64::from(h) > u64::from(sh) * u64::from(w) {
        ((u64::from(sh) * u64::from(w) / u64::from(h)) as u32, sh)
    } else {
        (sw, (u64::from(sw) * u64::from(h) / u64::from(w)) as u32)
    };
    imageops::crop_imm(img, (sw - cw) / 2, (sh - ch) / 2, cw, ch).to_image()
}

/// `img` at `w`×`h`: cut to that shape, then scaled.
pub fn sized(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let shaped = cut(img, w, h);
    if shaped.dimensions() == (w, h) {
        return shaped;
    }
    imageops::resize(&shaped, w, h, FilterType::Lanczos3)
}

/// `img` as a lossy WebP at [`QUALITY`], its transparency kept.
pub fn webp(img: &RgbaImage) -> Result<Vec<u8>> {
    let (w, h) = img.dimensions();
    let encoded = Encoder::from_rgba(img.as_raw(), w, h)
        .encode_simple(false, QUALITY)
        .map_err(|e| anyhow!("encoding WebP: {e:?}"))?;
    Ok(encoded.to_vec())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{load_from_memory_with_format, Rgba};
    use sha2::{Digest, Sha256};

    use super::*;

    /// A picture `w`×`h` whose left half is red and right half blue.
    fn halves(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, _| {
            if x < w / 2 {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([0, 0, 255, 255])
            }
        })
    }

    #[test]
    fn a_wide_picture_is_cut_from_its_centre() {
        // 8×2 cut to a square keeps columns 3 to 4: one red, one blue.
        let img = RgbaImage::from_fn(8, 2, |x, _| Rgba([x as u8, 0, 0, 255]));
        let sq = cut(&img, 1, 1);
        assert_eq!(sq.dimensions(), (2, 2));
        assert_eq!(sq.get_pixel(0, 0).0, [3, 0, 0, 255]);
        assert_eq!(sq.get_pixel(1, 1).0, [4, 0, 0, 255]);
        // 2×8 cut to 2:1 keeps rows 3 to 3.
        let tall = RgbaImage::from_fn(2, 8, |_, y| Rgba([0, y as u8, 0, 255]));
        let wide = cut(&tall, 2, 1);
        assert_eq!(wide.dimensions(), (2, 1));
        assert_eq!(wide.get_pixel(0, 0).0, [0, 3, 0, 255]);
        // A picture of the shape already is left whole.
        assert_eq!(cut(&halves(16, 9), 1024, 576).dimensions(), (16, 9));
    }

    /// **A sized picture is the same bytes every time**, and keeps its
    /// transparency exactly: libwebp codes the alpha plane without loss.
    #[test]
    fn a_sized_picture_is_lossy_webp_with_exact_alpha() {
        let mut img = halves(16, 16);
        for y in 0..16 {
            img.put_pixel(0, y, Rgba([10, 20, 30, 0]));
        }
        let bytes = webp(&img).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WEBP");
        let digest: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(
            digest,
            "b95924f812444991b83d8bef5e3bb54013a8dd6a5137b7eba0e5e96be7dd9ce0"
        );
        let back = load_from_memory_with_format(&bytes, ImageFormat::WebP)
            .unwrap()
            .to_rgba8();
        assert_eq!(back.dimensions(), (16, 16));
        for (x, y, p) in back.enumerate_pixels() {
            assert_eq!(p.0[3], if x == 0 { 0 } else { 255 }, "alpha at {x},{y}");
        }
    }

    #[test]
    fn a_picture_is_sized_to_the_kinds_shape() {
        let img = halves(1024, 1024);
        let out = sized(&img, 1024, 576);
        assert_eq!(out.dimensions(), (1024, 576), "cut, not scaled");
        let small = sized(&img, 128, 128);
        assert_eq!(small.dimensions(), (128, 128));
        assert_eq!(small.get_pixel(10, 64).0, [255, 0, 0, 255]);
        assert_eq!(small.get_pixel(117, 64).0, [0, 0, 255, 255]);
    }

    #[test]
    fn only_a_png_is_read() {
        let mut png = Cursor::new(Vec::new());
        halves(2, 2).write_to(&mut png, ImageFormat::Png).unwrap();
        assert_eq!(decode(png.get_ref()).unwrap().dimensions(), (2, 2));
        assert!(decode(b"not a picture").is_err());
        assert!(decode(&webp(&halves(2, 2)).unwrap()).is_err());
    }
}
