//! Embedded assets: the icon set and mascots are compiled into the binary,
//! so the app has no runtime asset directory to lose.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use image::ImageEncoder;

fn tinted_mascot(path: &str) -> Option<Vec<u8>> {
    let name = path.strip_prefix("mascots/tint/")?.strip_suffix(".png")?;
    let (shape, hex) = name.rsplit_once('-')?;
    if hex.len() != 6 {
        return None;
    }
    let rgb = [
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ];
    let source_name = format!("mascots/bot-{shape}@2x.png");
    let source = MASCOTS.iter().find(|(asset, _)| *asset == source_name)?.1;
    let image = image::load_from_memory(source).ok()?.into_rgba8();
    recolor_png(image, rgb)
}

fn tinted_mascot_frame(path: &str) -> Option<Vec<u8>> {
    let name = path
        .strip_prefix("mascots/animated/")?
        .strip_suffix(".png")?;
    let (identity, frame) = name.rsplit_once('-')?;
    let frame: u32 = frame.parse().ok()?;
    if frame >= 29 {
        return None;
    }
    let (shape, hex) = identity.rsplit_once('-')?;
    if hex.len() != 6 {
        return None;
    }
    let rgb = [
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ];
    let source_name = format!("mascots/bot-{shape}-face@2x.png");
    let source = MASCOTS.iter().find(|(asset, _)| *asset == source_name)?.1;
    let strip = image::load_from_memory(source).ok()?.into_rgba8();
    let (width, height) = strip.dimensions();
    if width % 29 != 0 {
        return None;
    }
    let frame_width = width / 29;
    let frame = image::imageops::crop_imm(&strip, frame * frame_width, 0, frame_width, height)
        .to_image();
    recolor_png(frame, rgb)
}

fn recolor_png(mut image: image::RgbaImage, rgb: [u8; 3]) -> Option<Vec<u8>> {
    for pixel in image.pixels_mut() {
        pixel.0[0] = rgb[0];
        pixel.0[1] = rgb[1];
        pixel.0[2] = rgb[2];
    }
    let (width, height) = image.dimensions();
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;
    Some(bytes)
}

macro_rules! icon_table {
    ($($name:literal),* $(,)?) => {
        pub const ICONS: &[(&str, &[u8])] = &[
            $(
                (
                    concat!("icons/", $name, ".svg"),
                    include_bytes!(concat!("../assets/icons/", $name, ".svg")) as &[u8],
                ),
            )*
        ];
    };
}

icon_table!(
    "plus",
    "search",
    "send",
    "paperclip",
    "chevron-down",
    "chevron-right",
    "monitor",
    "clock",
    "folder",
    "terminal",
    "sparkle",
    "check",
    "x",
    "stop",
    "panel",
    "refresh",
    "mail",
    "calendar",
    "globe",
    "more",
    "bot",
    "sun",
    "moon",
    "edit",
    "file",
    "copy",
    "gear",
    "sliders",
    "users",
    "mascot",
);

macro_rules! mascot_table {
    ($($shape:literal),* $(,)?) => {
        pub const MASCOTS: &[(&str, &[u8])] = &[
            $(
                (
                    concat!("mascots/bot-", $shape, ".png"),
                    include_bytes!(concat!("../assets/mascots/bot-", $shape, ".png")) as &[u8],
                ),
                (
                    concat!("mascots/bot-", $shape, "@2x.png"),
                    include_bytes!(concat!("../assets/mascots/bot-", $shape, "@2x.png")) as &[u8],
                ),
                (
                    concat!("mascots/bot-", $shape, "-face.png"),
                    include_bytes!(concat!("../assets/mascots/bot-", $shape, "-face.png")) as &[u8],
                ),
                (
                    concat!("mascots/bot-", $shape, "-face@2x.png"),
                    include_bytes!(concat!("../assets/mascots/bot-", $shape, "-face@2x.png")) as &[u8],
                ),
            )*
            (
                "mascots/brand.png",
                include_bytes!("../assets/mascots/brand.png") as &[u8],
            ),
            (
                "mascots/brand@2x.png",
                include_bytes!("../assets/mascots/brand@2x.png") as &[u8],
            ),
            (
                "mascots/working.png",
                include_bytes!("../assets/mascots/working.png") as &[u8],
            ),
            (
                "mascots/working@2x.png",
                include_bytes!("../assets/mascots/working@2x.png") as &[u8],
            ),
        ];
    };
}

mascot_table!(
    "arch",
    "bean",
    "blob",
    "capsule",
    "cloud",
    "clover",
    "crystal",
    "cylinder",
    "dome",
    "egg",
    "flower",
    "gem",
    "heart",
    "hex",
    "house",
    "leaf",
    "pebble",
    "pumpkin",
    "shield",
    "sparkle",
    "square",
    "squircle",
    "star",
    "tablet",
    "teardrop",
    "wedge",
);

macro_rules! mascot_svg_table {
    ($($shape:literal),* $(,)?) => {
        pub const MASCOT_SVGS: &[(&str, &[u8])] = &[
            $(
                (
                    concat!("icons/mascots/shape-", $shape, ".svg"),
                    include_bytes!(concat!("../assets/icons/mascots/shape-", $shape, ".svg")) as &[u8],
                ),
                (
                    concat!("icons/mascots/face-", $shape, ".svg"),
                    include_bytes!(concat!("../assets/icons/mascots/face-", $shape, ".svg")) as &[u8],
                ),
            )*
        ];
    };
}

mascot_svg_table!(
    "arch",
    "bean",
    "blob",
    "capsule",
    "cloud",
    "clover",
    "crystal",
    "cylinder",
    "dome",
    "egg",
    "flower",
    "gem",
    "heart",
    "hex",
    "house",
    "leaf",
    "pebble",
    "pumpkin",
    "shield",
    "sparkle",
    "square",
    "squircle",
    "star",
    "tablet",
    "teardrop",
    "wedge",
);

/// gpui asset source over embedded [`ICONS`], [`MASCOTS`] and [`MASCOT_SVGS`], with filesystem fallback.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(png) = tinted_mascot_frame(path) {
            return Ok(Some(Cow::Owned(png)));
        }
        if let Some(png) = tinted_mascot(path) {
            return Ok(Some(Cow::Owned(png)));
        }
        if let Some((_, bytes)) = ICONS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(*bytes)));
        }
        if let Some((_, bytes)) = MASCOTS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(*bytes)));
        }
        if let Some((_, bytes)) = MASCOT_SVGS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(*bytes)));
        }
        // Fallback to reading file from assets/ directory if present
        if let Ok(bytes) = std::fs::read(format!("assets/{}", path)) {
            return Ok(Some(Cow::Owned(bytes)));
        }
        Ok(None)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut results: Vec<SharedString> = ICONS
            .iter()
            .chain(MASCOTS.iter())
            .chain(MASCOT_SVGS.iter())
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect();
        results.sort();
        results.dedup();
        Ok(results)
    }
}
