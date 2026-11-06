//! A colour as an app's theme writes it (`#d94f70`), and the arithmetic the
//! contrast check runs on it: relative luminance and contrast ratio as WCAG
//! 2.2 defines them, and a mix of two colours.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Colour {
    pub const WHITE: Colour = Colour {
        r: 255,
        g: 255,
        b: 255,
    };

    /// `#rrggbb` or `#rgb`, either case; nothing else.
    pub fn parse(text: &str) -> Option<Colour> {
        let hex = text.trim().strip_prefix('#')?;
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let digit = |i: usize, w: usize| u8::from_str_radix(&hex[i..i + w], 16).ok();
        match hex.len() {
            6 => Some(Colour {
                r: digit(0, 2)?,
                g: digit(2, 2)?,
                b: digit(4, 2)?,
            }),
            3 => Some(Colour {
                r: digit(0, 1)? * 17,
                g: digit(1, 1)? * 17,
                b: digit(2, 1)? * 17,
            }),
            _ => None,
        }
    }

    /// Relative luminance, 0 for black to 1 for white.
    pub fn luminance(self) -> f64 {
        let lin = |c: u8| {
            let s = f64::from(c) / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// `self` moved `t` of the way to `other`, channel by channel.
    pub fn mix(self, other: Colour, t: f64) -> Colour {
        let ch = |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
        Colour {
            r: ch(self.r, other.r),
            g: ch(self.g, other.g),
            b: ch(self.b, other.b),
        }
    }
}

/// The contrast ratio of two colours, 1 to 21.
pub fn contrast(a: Colour, b: Colour) -> f64 {
    let (la, lb) = (a.luminance(), b.luminance());
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

impl fmt::Display for Colour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl TryFrom<String> for Colour {
    type Error = String;

    fn try_from(text: String) -> Result<Colour, String> {
        Colour::parse(&text)
            .ok_or_else(|| format!("`{text}` is not a colour: write it as `#d94f70`"))
    }
}

impl From<Colour> for String {
    fn from(c: Colour) -> String {
        c.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Colour = Colour { r: 0, g: 0, b: 0 };

    #[test]
    fn a_colour_is_written_as_hex() {
        assert_eq!(
            Colour::parse("#C2375B"),
            Some(Colour {
                r: 194,
                g: 55,
                b: 91
            })
        );
        assert_eq!(Colour::parse("#fff"), Some(Colour::WHITE));
        assert_eq!(Colour::parse(" #000000 "), Some(BLACK));
        for bad in ["pink", "#12345", "c2375b", "#gggggg", "#c2375b00", ""] {
            assert_eq!(Colour::parse(bad), None, "{bad}");
        }
        assert_eq!(Colour::parse("#C2375B").unwrap().to_string(), "#c2375b");
    }

    /// **Contrast is WCAG's**: black on white is 21:1, and `#777` on white
    /// the well-known 4.48:1, just short of AA.
    #[test]
    fn contrast_is_wcags() {
        assert_eq!(contrast(Colour::WHITE, BLACK), 21.0);
        assert_eq!(contrast(BLACK, Colour::WHITE), 21.0);
        let grey = Colour::parse("#777").unwrap();
        assert_eq!(format!("{:.2}", contrast(grey, Colour::WHITE)), "4.48");
        assert_eq!(contrast(grey, grey), 1.0);
    }

    #[test]
    fn colours_mix_channel_by_channel() {
        assert_eq!(
            BLACK.mix(Colour::WHITE, 0.5),
            Colour {
                r: 128,
                g: 128,
                b: 128
            }
        );
        let accent = Colour::parse("#c2375b").unwrap();
        assert_eq!(accent.mix(Colour::WHITE, 0.0), accent);
        assert_eq!(accent.mix(Colour::WHITE, 1.0), Colour::WHITE);
        assert_eq!(accent.mix(Colour::WHITE, 0.25).to_string(), "#d16984");
    }
}
