//! Aucteris UI — a desktop workspace for a team of AI agents.
//!
//! Rust-native desktop interface with a local simulated agent engine.

mod assets;
mod engine;
mod model;
mod theme;
mod ui;

use gpui::{App, AppContext as _, Bounds, SharedString, WindowBounds, WindowOptions, px, size};

use theme::Appearance;
use ui::shell::Shell;

/// Preference order for the UI face. The first family the machine actually has
/// wins, so the app never renders in a fallback that was never chosen.
const SANS: [&str; 7] = [
    "Inter",
    "Geist",
    "Adwaita Sans",
    "Cantarell",
    "Noto Sans",
    "Ubuntu",
    "DejaVu Sans",
];

const MONO: [&str; 6] = [
    "JetBrains Mono",
    "Geist Mono",
    "Adwaita Mono",
    "Noto Sans Mono",
    "Ubuntu Mono",
    "DejaVu Sans Mono",
];

fn pick(available: &[String], wanted: &[&str], fallback: SharedString) -> SharedString {
    for candidate in wanted {
        if available.iter().any(|name| name == candidate) {
            return SharedString::from(*candidate);
        }
    }
    fallback
}

fn main() {
    gpui_platform::application()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            let available = cx.text_system().all_font_names();
            let mut boot_theme = theme::Theme::dark();
            boot_theme.font_sans = pick(&available, &SANS, boot_theme.font_sans.clone());
            boot_theme.font_mono = pick(&available, &MONO, boot_theme.font_mono.clone());
            cx.set_global(boot_theme);

            let bounds = Bounds::centered(None, size(px(980.0), px(660.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(900.0), px(560.0))),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Aucteris UI".into()),
                        appears_transparent: false,
                        ..Default::default()
                    }),
                    app_id: Some("aucteris-ui".into()),
                    ..Default::default()
                },
                |window, cx| {
                    let appearance = Appearance::from_window(window.appearance());
                    // Install the skin the OS is already using before the first
                    // frame, then keep the fonts boot picked.
                    let mut theme = theme::Theme::for_appearance(appearance);
                    let available = cx.text_system().all_font_names();
                    theme.font_sans = pick(&available, &SANS, theme.font_sans.clone());
                    theme.font_mono = pick(&available, &MONO, theme.font_mono.clone());
                    cx.set_global(theme);
                    cx.new(|cx| Shell::new(appearance, cx))
                },
            )
            .expect("failed to open the Aucteris UI window");
            cx.activate(true);
        });
}
