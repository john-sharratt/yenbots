//! What a file is, read from its bytes, never its name (§12.6): the kinds an
//! app may keep — pictures and PDFs — each told by how its bytes begin.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Png,
    Jpeg,
    Webp,
    Gif,
    Pdf,
}

impl FileKind {
    /// The kind `bytes` are, if they are one an app may keep.
    pub fn of(bytes: &[u8]) -> Option<FileKind> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(FileKind::Png)
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            Some(FileKind::Jpeg)
        } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(FileKind::Webp)
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some(FileKind::Gif)
        } else if bytes.starts_with(b"%PDF-") {
            Some(FileKind::Pdf)
        } else {
            None
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            FileKind::Png => "image/png",
            FileKind::Jpeg => "image/jpeg",
            FileKind::Webp => "image/webp",
            FileKind::Gif => "image/gif",
            FileKind::Pdf => "application/pdf",
        }
    }

    pub fn parse(s: &str) -> Option<FileKind> {
        [
            FileKind::Png,
            FileKind::Jpeg,
            FileKind::Webp,
            FileKind::Gif,
            FileKind::Pdf,
        ]
        .into_iter()
        .find(|k| k.name() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            FileKind::Png => "png",
            FileKind::Jpeg => "jpeg",
            FileKind::Webp => "webp",
            FileKind::Gif => "gif",
            FileKind::Pdf => "pdf",
        }
    }

    /// Whether it is a picture, which gets a thumbnail.
    pub fn is_picture(self) -> bool {
        self != FileKind::Pdf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_told_by_its_bytes() {
        assert_eq!(FileKind::of(b"\x89PNG\r\n\x1a\nrest"), Some(FileKind::Png));
        assert_eq!(
            FileKind::of(&[0xff, 0xd8, 0xff, 0xe0]),
            Some(FileKind::Jpeg)
        );
        assert_eq!(FileKind::of(b"RIFF\0\0\0\0WEBPVP8 "), Some(FileKind::Webp));
        assert_eq!(FileKind::of(b"GIF89a.."), Some(FileKind::Gif));
        assert_eq!(FileKind::of(b"%PDF-1.7"), Some(FileKind::Pdf));
        for no in [
            &b"<svg xmlns"[..],
            b"<html>",
            b"MZ\x90\0",
            b"",
            b"RIFF\0\0\0\0WAVE",
        ] {
            assert_eq!(FileKind::of(no), None, "{no:?}");
        }
        for k in [
            FileKind::Png,
            FileKind::Jpeg,
            FileKind::Webp,
            FileKind::Gif,
            FileKind::Pdf,
        ] {
            assert_eq!(FileKind::parse(k.name()), Some(k));
        }
        assert!(!FileKind::Pdf.is_picture());
    }
}
