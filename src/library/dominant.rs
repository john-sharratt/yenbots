//! A picture's dominant colour, and the palette colour it is nearest
//! (`tokens.css`), for searching the library by colour (§15.3).
//!
//! A flat illustration is mostly its soft ground — cream, beige, a pale wall
//! — so the colour that says what a picture *is* comes from its vivid
//! pixels: saturated, and not pale. The most common colour among them is
//! read in buckets of 16 levels a channel and answered as the bucket's mean.
//! A picture with no vivid pixels — a grey texture — is read over all of
//! them. Transparent pixels are never read.
//!
//! A tile is the exception: what it shows on the landing page is its flat
//! field, so its colour is the field's, read at its edge ([`field`]).

use std::collections::BTreeMap;

use image::{Rgba, RgbaImage};

use crate::apps::theme::colour::Colour;
use crate::apps::theme::tokens::Tokens;

/// The palette's colours by name, as `web/_/ui/palette.js` lists them.
pub const PALETTE: [&str; 8] = [
    "coral", "rose", "amber", "sage", "teal", "sky", "violet", "slate",
];

/// The edition of how a colour is read, raised whenever the reading
/// changes, so the colours cached by the last one are read again.
pub const EDITION: u32 = 4;

/// A pixel this far between its highest and lowest channel is saturated.
const SATURATED: u8 = 48;

/// A pixel whose lowest channel reaches this is pale — cream, beige, a
/// pastel: a ground, not a subject. Every palette colour has a channel at 93
/// or below.
const PALE: u8 = 144;

/// The dominant colour of `img`; `None` for a picture with no opaque pixel.
pub fn dominant(img: &RgbaImage) -> Option<Colour> {
    read(img.pixels())
}

/// The colour of a tile's field: read over the ring of pixels at its edge,
/// a thirty-second of its side deep, which a tile's flat field fills
/// whatever stands in its middle.
pub fn field(img: &RgbaImage) -> Option<Colour> {
    let (w, h) = img.dimensions();
    let deep = (w.min(h) / 32).max(1);
    read(
        img.enumerate_pixels()
            .filter(|(x, y, _)| *x < deep || *y < deep || *x >= w - deep || *y >= h - deep)
            .map(|(_, _, p)| p),
    )
}

/// The dominant colour of `pixels`.
fn read<'a>(pixels: impl Iterator<Item = &'a Rgba<u8>>) -> Option<Colour> {
    // bucket -> (count, sum r, sum g, sum b)
    let mut all: BTreeMap<u16, (u64, u64, u64, u64)> = BTreeMap::new();
    let mut vivid: BTreeMap<u16, (u64, u64, u64, u64)> = BTreeMap::new();
    for p in pixels {
        let [r, g, b, a] = p.0;
        if a < 128 {
            continue;
        }
        let bucket = (u16::from(r >> 4) << 8) | (u16::from(g >> 4) << 4) | u16::from(b >> 4);
        let add = |m: &mut BTreeMap<u16, (u64, u64, u64, u64)>| {
            let e = m.entry(bucket).or_default();
            e.0 += 1;
            e.1 += u64::from(r);
            e.2 += u64::from(g);
            e.3 += u64::from(b);
        };
        add(&mut all);
        let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
        if hi - lo >= SATURATED && lo < PALE {
            add(&mut vivid);
        }
    }
    let read = if vivid.is_empty() { &all } else { &vivid };
    // The most common bucket; of two as common, the lower.
    let (_, &(n, r, g, b)) = read
        .iter()
        .max_by(|(ka, a), (kb, b)| a.0.cmp(&b.0).then(kb.cmp(ka)))?;
    let mean = |s: u64| ((s + n / 2) / n) as u8;
    Some(Colour {
        r: mean(r),
        g: mean(g),
        b: mean(b),
    })
}

/// The palette colour nearest `c`, by distance in RGB weighted for the eye.
pub fn nearest(c: Colour) -> &'static str {
    let t = Tokens::platform();
    let far = |p: Colour| {
        let d = |a: u8, b: u8| (i64::from(a) - i64::from(b)).pow(2);
        2 * d(c.r, p.r) + 4 * d(c.g, p.g) + 3 * d(c.b, p.b)
    };
    PALETTE
        .into_iter()
        .min_by_key(|name| {
            t.colour(name, false)
                .map(far)
                .expect("every palette colour is a token")
        })
        .expect("the palette is not empty")
}

#[cfg(test)]
mod tests {
    use image::Rgba;

    use super::*;

    fn picture(pixels: &[([u8; 4], u32)]) -> RgbaImage {
        let total: u32 = pixels.iter().map(|p| p.1).sum();
        let mut img = RgbaImage::new(total, 1);
        let mut x = 0;
        for &(px, n) in pixels {
            for _ in 0..n {
                img.put_pixel(x, 0, Rgba(px));
                x += 1;
            }
        }
        img
    }

    /// **The saturated colour wins over a larger soft ground**, read as the
    /// mean of its bucket.
    #[test]
    fn the_subject_not_the_ground_is_dominant() {
        let img = picture(&[
            ([250, 244, 236, 255], 10), // cream ground
            ([228, 216, 176, 255], 8),  // a beige wall: saturated, but pale
            ([222, 209, 169, 255], 8),  // and its shade
            ([212, 87, 61, 255], 3),    // coral
            ([210, 85, 60, 255], 1),    // coral, same bucket
            ([47, 116, 192, 255], 2),   // sky
            ([0, 0, 255, 0], 20),       // transparent, never read
        ]);
        assert_eq!(dominant(&img).unwrap().to_string(), "#d4573d");
        // (212*3 + 210) / 4 = 211.5 → 212; (87*3+85)/4 = 86.5 → 87; (61*3+60)/4 = 60.75 → 61.
    }

    /// **A tile's colour is its field's**, however vivid its subject: a
    /// soft sage field with a coral pot reads sage.
    #[test]
    fn a_tiles_colour_is_its_field() {
        let mut img = RgbaImage::from_pixel(64, 64, Rgba([160, 185, 150, 255]));
        for y in 8..56 {
            for x in 8..56 {
                img.put_pixel(x, y, Rgba([212, 87, 61, 255]));
            }
        }
        assert_eq!(dominant(&img).unwrap().to_string(), "#d4573d");
        let f = field(&img).unwrap();
        assert_eq!(f.to_string(), "#a0b996");
        assert_eq!(nearest(f), "sage");
    }

    #[test]
    fn a_grey_picture_is_read_over_every_pixel() {
        let img = picture(&[([200, 200, 200, 255], 3), ([90, 90, 92, 255], 1)]);
        assert_eq!(dominant(&img).unwrap().to_string(), "#c8c8c8");
        assert_eq!(dominant(&picture(&[([1, 2, 3, 0], 4)])), None);
    }

    #[test]
    fn each_palette_colour_is_nearest_itself() {
        let t = Tokens::platform();
        for name in PALETTE {
            assert_eq!(nearest(t.colour(name, false).unwrap()), name);
        }
        assert_eq!(nearest(Colour::parse("#e06040").unwrap()), "coral");
        assert_eq!(nearest(Colour::parse("#3080d0").unwrap()), "sky");
    }

    /// **The palette here is the palette the pages use.**
    #[test]
    fn the_palette_is_palette_js() {
        let js = include_str!("../../web/_/ui/palette.js");
        let listed = js
            .split("export const PALETTE = [")
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap()
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .collect::<Vec<_>>();
        assert_eq!(listed, PALETTE);
    }
}
