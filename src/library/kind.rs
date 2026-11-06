//! The five kinds of picture in the art library (§15.1), and the sizes each
//! is served at. A picture is drawn once at the image model's size and
//! committed as it was drawn; the platform sizes it down to these.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// App icons for the landing page: one subject on a soft shape.
    Tiles,
    /// Scenes for headers and onboarding.
    Illustrations,
    /// Empty-state art: "nothing here yet".
    Empty,
    /// Soft textures behind a page or a card.
    Backgrounds,
    /// Small cut-out objects and badges, on transparency.
    Stickers,
}

impl Kind {
    pub const ALL: [Kind; 5] = [
        Kind::Tiles,
        Kind::Illustrations,
        Kind::Empty,
        Kind::Backgrounds,
        Kind::Stickers,
    ];

    /// The folder under `library/`, and the word a catalogue line and a URL
    /// use.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Tiles => "tiles",
            Kind::Illustrations => "illustrations",
            Kind::Empty => "empty",
            Kind::Backgrounds => "backgrounds",
            Kind::Stickers => "stickers",
        }
    }

    pub fn parse(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.name() == name)
    }

    /// The sizes it is served at, `(width, height)`, largest first. A
    /// picture of another shape is cut to this shape from its centre.
    pub fn sizes(self) -> &'static [(u32, u32)] {
        match self {
            Kind::Tiles => &[(512, 512), (256, 256), (128, 128)],
            Kind::Illustrations => &[(1024, 576), (512, 288)],
            Kind::Empty => &[(600, 450), (300, 225)],
            Kind::Backgrounds => &[(1024, 1024), (512, 512)],
            Kind::Stickers => &[(256, 256), (128, 128)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_reads_back_its_name() {
        for k in Kind::ALL {
            assert_eq!(Kind::parse(k.name()), Some(k));
        }
        assert_eq!(Kind::parse("tile"), None);
        assert_eq!(Kind::parse("Tiles"), None);
    }

    /// **Every size of a kind has the shape of its largest**, so one cut
    /// serves them all.
    #[test]
    fn a_kinds_sizes_share_one_shape() {
        for k in Kind::ALL {
            let (w0, h0) = k.sizes()[0];
            for &(w, h) in k.sizes() {
                assert_eq!(w * h0, h * w0, "{k:?} {w}x{h}");
                assert!(w <= w0, "{k:?}: largest first");
            }
        }
    }
}
