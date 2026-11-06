use std::io::Cursor;

use image::{load_from_memory_with_format, ImageFormat, Rgba, RgbaImage};

use super::*;

/// A PNG `w`×`h` of one colour.
fn png(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbaImage::from_pixel(w, h, Rgba(px))
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

fn put(root: &Path, path: &str, bytes: &[u8]) {
    let p = root.join(path);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, bytes).unwrap();
}

const CATALOGUE: &str = "tiles/bell-coral:\n  tags: [bell, reminders]\n  seed: 7\n  prompt: A bell.\nstickers/star:\n  tags: [star]\n  seed: 8\n  prompt: A star.\n";

fn library() -> (tempfile::TempDir, Library) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("live");
    put(&root, catalogue::PATH, CATALOGUE.as_bytes());
    put(
        &root,
        "library/tiles/bell-coral.png",
        &png(64, 64, [212, 87, 61, 255]),
    );
    put(
        &root,
        "library/stickers/star.png",
        &png(32, 48, [47, 116, 192, 255]),
    );
    let lib = Library::new(root, dir.path().join("cache"));
    lib.reload();
    (dir, lib)
}

/// **Every picture is sized to its kind and its colour read**; a request
/// names a size that is there, and no other.
#[test]
fn the_library_sizes_every_picture() {
    let (_dir, lib) = library();
    let bell = lib.get("tiles/bell-coral").unwrap();
    assert_eq!(bell.colour.to_string(), "#d4573d");
    assert_eq!(bell.palette, "coral");
    let shown = bell.shown();
    assert_eq!(
        shown
            .sizes
            .iter()
            .map(|s| (s.width, s.height))
            .collect::<Vec<_>>(),
        [(512, 512), (256, 256), (128, 128)]
    );
    assert_eq!(
        shown.sizes[1].url,
        format!("/_library/tiles/bell-coral-256.webp?v={}", bell.digest)
    );
    let file = lib.file("tiles", "bell-coral-256.webp").unwrap();
    let back =
        load_from_memory_with_format(&std::fs::read(file).unwrap(), ImageFormat::WebP).unwrap();
    assert_eq!((back.width(), back.height()), (256, 256));
    assert_eq!(lib.file("tiles", "bell-coral-300.webp"), None);
    assert_eq!(lib.file("tiles", "bell-coral-256.png"), None);
    assert_eq!(lib.file("stickers", "bell-coral-256.webp"), None);

    let star = lib.get("stickers/star").unwrap();
    assert_eq!(star.palette, "sky");
    assert!(lib.file("stickers", "star-128.webp").unwrap().is_file());
}

#[test]
fn the_library_is_found_by_kind_tag_and_colour() {
    let (_dir, lib) = library();
    let keys = |f: Find| -> Vec<String> { lib.find(&f).iter().map(|p| p.entry.key()).collect() };
    assert_eq!(keys(Find::default()), ["stickers/star", "tiles/bell-coral"]);
    assert_eq!(
        keys(Find {
            kind: Some(Kind::Tiles),
            ..Find::default()
        }),
        ["tiles/bell-coral"]
    );
    assert_eq!(
        keys(Find {
            tag: Some("star".into()),
            ..Find::default()
        }),
        ["stickers/star"]
    );
    assert_eq!(
        keys(Find {
            colour: Some("coral".into()),
            kind: Some(Kind::Stickers),
            ..Find::default()
        }),
        Vec::<String>::new()
    );
}

/// **A picture is sized once**: a second load reads the cache, and a
/// picture taken out of the library takes its cache with it.
#[test]
fn a_picture_is_sized_once_and_its_cache_goes_with_it() {
    let (dir, lib) = library();
    let bell = lib.get("tiles/bell-coral").unwrap();
    let cached = dir.path().join("cache").join(&bell.digest);
    // A colour written into the cache by hand is what the next load reads.
    std::fs::write(cached.join("colour"), "#010203").unwrap();
    lib.reload();
    assert_eq!(
        lib.get("tiles/bell-coral").unwrap().colour.to_string(),
        "#010203"
    );

    let root = dir.path().join("live");
    put(
        &root,
        catalogue::PATH,
        CATALOGUE.split("stickers/").next().unwrap().as_bytes(),
    );
    lib.reload();
    assert!(lib.get("stickers/star").is_none());
    assert_eq!(
        std::fs::read_dir(dir.path().join("cache")).unwrap().count(),
        1,
        "only the bell's cache is left"
    );
}

/// **A catalogue that does not parse leaves the library as it was**; a
/// line whose picture is missing is left out alone.
#[test]
fn a_bad_catalogue_keeps_the_library_and_a_missing_picture_is_left_out() {
    let (dir, lib) = library();
    let root = dir.path().join("live");
    put(&root, catalogue::PATH, b"tiles/bell-coral: [unclosed\n");
    lib.reload();
    assert_eq!(lib.find(&Find::default()).len(), 2);

    put(
        &root,
        catalogue::PATH,
        format!(
            "{CATALOGUE}empty/no-recipes:\n  tags: [cooking]\n  seed: 9\n  prompt: An empty pot.\n"
        )
        .as_bytes(),
    );
    put(&root, "library/stickers/star.png", b"not a png");
    lib.reload();
    let keys: Vec<String> = lib
        .find(&Find::default())
        .iter()
        .map(|p| p.entry.key())
        .collect();
    assert_eq!(keys, ["tiles/bell-coral"]);
}
