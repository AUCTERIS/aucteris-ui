//! Design tokens.
//!
//! The app is painted from a single token set with two appearances ("Midnight"
//! and "Daylight"), the same arrangement komet uses: colors are data, numbers
//! are functions of the layout, and no view ever hardcodes a hex value.
//!
//! Daylight is designed, not inverted. Chrome (the sidebar) goes *darker* than
//! the chat plane in light mode — the opposite of a literal inversion — and
//! floating surfaces earn their separation from a hairline instead of from a
//! lighter fill, because there is nothing lighter than white to climb to.
//!
//! Hierarchy comes from three ink levels and size, never from a fourth grey.
//! Accent is rare: links, focus, the selected row. Bot colors belong to bots.

use gpui::{App, Global, Hsla, SharedString, hsla};

/// Which skin the app is painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
}

impl Appearance {
    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Midnight",
            Self::Light => "Daylight",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    /// Follow the OS, so the first paint is never the wrong skin.
    pub fn from_window(appearance: gpui::WindowAppearance) -> Self {
        use gpui::WindowAppearance::*;
        match appearance {
            Light | VibrantLight => Self::Light,
            Dark | VibrantDark => Self::Dark,
        }
    }
}

/// Every color the UI is allowed to name.
///
/// The table is complete on purpose: a menu or card surface that has not landed
/// yet still has a token, so the next view picks one instead of inventing a hex.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Theme {
    pub appearance: Appearance,

    // Surfaces
    /// The chat plane.
    pub bg: Hsla,
    /// The sidebar plane.
    pub panel: Hsla,
    /// A card resting in the page (bot panel sections).
    pub card: Hsla,
    /// Selected sidebar row, user bubble plate.
    pub raised: Hsla,
    pub raised_hover: Hsla,
    /// Recessed field (search).
    pub inset: Hsla,
    /// A small control plate (chips, icon buttons).
    pub control: Hsla,
    pub menu: Hsla,
    pub menu_hover: Hsla,
    pub menu_active: Hsla,
    /// The composer plate.
    pub composer: Hsla,
    /// A dialog's surface: the highest plane in the app.
    pub dialog: Hsla,
    /// What sits behind a dialog. Nothing else dims the app.
    pub scrim: Hsla,

    // Ink
    pub ink: Hsla,
    pub ink_secondary: Hsla,
    pub ink_tertiary: Hsla,

    // Lines
    pub hairline: Hsla,
    pub edge: Hsla,

    // Accent
    pub accent: Hsla,
    pub accent_text: Hsla,
    /// Label on an accent fill.
    pub on_accent: Hsla,
    pub focus: Hsla,

    // Status
    pub success: Hsla,
    pub warning: Hsla,
    pub danger: Hsla,
    pub danger_ink: Hsla,

    // Bot identity
    identity: [Hsla; 8],

    // Type
    pub font_sans: SharedString,
    pub font_mono: SharedString,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Global for Theme {}

impl Theme {
    pub fn dark() -> Self {
        let ink = hsla(0.11, 0.05, 0.95, 1.0);
        Self {
            appearance: Appearance::Dark,
            bg: hsla(0.0, 0.0, 0.133, 1.0),
            panel: hsla(0.0, 0.0, 0.082, 1.0),
            card: hsla(0.0, 0.0, 0.145, 1.0),
            raised: hsla(0.0, 0.0, 0.157, 1.0),
            raised_hover: hsla(0.0, 0.0, 0.18, 1.0),
            inset: hsla(0.0, 0.0, 0.095, 1.0),
            control: hsla(0.0, 0.0, 0.16, 1.0),
            menu: hsla(0.0, 0.0, 0.15, 1.0),
            menu_hover: hsla(0.0, 0.0, 0.215, 1.0),
            menu_active: hsla(0.0, 0.0, 0.255, 1.0),
            composer: hsla(0.0, 0.0, 0.13, 1.0),
            dialog: hsla(0.0, 0.0, 0.155, 1.0),
            scrim: hsla(0.0, 0.0, 0.0, 0.62),
            ink,
            ink_secondary: hsla(0.09, 0.03, 0.65, 1.0),
            ink_tertiary: hsla(0.09, 0.03, 0.475, 1.0),
            hairline: hsla(0.0, 0.0, 1.0, 0.10),
            edge: hsla(0.0, 0.0, 1.0, 0.15),
            accent: ink,
            accent_text: ink,
            on_accent: hsla(0.0, 0.0, 0.075, 1.0),
            // Focus is ink, not a hue: the accent stays reserved for links and
            // the selected state, and a neutral ring reads calm on every skin.
            focus: hsla(0.0, 0.0, 1.0, 0.32),
            success: hsla(0.42, 0.55, 0.55, 1.0),
            warning: hsla(0.11, 0.85, 0.62, 1.0),
            danger: hsla(0.02, 0.72, 0.62, 1.0),
            danger_ink: hsla(0.02, 0.35, 0.12, 1.0),
            identity: identity_palette(true),
            font_sans: "Adwaita Sans".into(),
            font_mono: "Adwaita Mono".into(),
        }
    }

    pub fn light() -> Self {
        let ink = hsla(0.11, 0.06, 0.115, 1.0);
        Self {
            appearance: Appearance::Light,
            bg: hsla(0.0, 0.0, 1.0, 1.0),
            panel: hsla(0.12, 0.08, 0.955, 1.0),
            card: hsla(0.0, 0.0, 1.0, 1.0),
            raised: hsla(0.12, 0.05, 0.895, 1.0),
            raised_hover: hsla(0.12, 0.05, 0.86, 1.0),
            inset: hsla(0.12, 0.06, 0.945, 1.0),
            control: hsla(0.12, 0.05, 0.93, 1.0),
            menu: hsla(0.0, 0.0, 1.0, 1.0),
            menu_hover: hsla(0.12, 0.05, 0.945, 1.0),
            menu_active: hsla(0.12, 0.05, 0.915, 1.0),
            composer: hsla(0.12, 0.08, 0.975, 1.0),
            dialog: hsla(0.0, 0.0, 1.0, 1.0),
            scrim: hsla(0.0, 0.0, 0.0, 0.30),
            ink,
            ink_secondary: hsla(0.09, 0.04, 0.36, 1.0),
            ink_tertiary: hsla(0.09, 0.03, 0.545, 1.0),
            hairline: hsla(0.0, 0.0, 0.0, 0.10),
            edge: hsla(0.0, 0.0, 0.0, 0.145),
            accent: ink,
            accent_text: ink,
            on_accent: hsla(0.0, 0.0, 0.99, 1.0),
            focus: hsla(0.0, 0.0, 0.0, 0.24),
            success: hsla(0.42, 0.62, 0.36, 1.0),
            warning: hsla(0.10, 0.78, 0.40, 1.0),
            danger: hsla(0.01, 0.70, 0.47, 1.0),
            danger_ink: hsla(0.0, 0.0, 0.99, 1.0),
            identity: identity_palette(false),
            font_sans: "Adwaita Sans".into(),
            font_mono: "Adwaita Mono".into(),
        }
    }

    pub fn for_appearance(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Dark => Self::dark(),
            Appearance::Light => Self::light(),
        }
    }

    /// Install as the gpui global. The only place the appearance may change.
    pub fn install(appearance: Appearance, cx: &mut App) {
        cx.set_global(Self::for_appearance(appearance));
    }

    pub fn of(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    pub fn is_dark(&self) -> bool {
        self.appearance.is_dark()
    }

    /// A bot's own color. `slot` wraps, so any bot always gets one.
    pub fn identity(&self, slot: usize) -> Hsla {
        self.identity[slot % self.identity.len()]
    }

    /// Hover/pressed fills are ink at low opacity, so they work on every skin.
    pub fn hover_wash(&self) -> Hsla {
        self.ink.opacity(0.055)
    }

    /// The user's own message plate: a tint, not a second sidebar.
    pub fn bubble(&self) -> Hsla {
        self.ink.opacity(if self.is_dark() { 0.07 } else { 0.05 })
    }
}

fn identity_palette(dark: bool) -> [Hsla; 8] {
    // Grok Bot's mascot palette: red, orange, yellow, green, cyan, blue,
    // violet and magenta. Keep the original colors in both appearances.
    let _ = dark;
    [
        hsla(0.98310, 1.0, 0.57451, 1.0),
        hsla(0.06732, 1.0, 0.5, 1.0),
        hsla(0.09935, 1.0, 0.5, 1.0),
        hsla(0.42786, 1.0, 0.39412, 1.0),
        hsla(0.48050, 1.0, 0.36863, 1.0),
        hsla(0.58543, 0.99167, 0.52941, 1.0),
        hsla(0.72323, 0.98802, 0.67255, 1.0),
        hsla(0.91385, 1.0, 0.59412, 1.0),
    ]
}

/// Layout numbers. Spacing is a 4px grid, control heights are 24/28/32/36.
pub mod metrics {
    pub const SIDEBAR_WIDTH: f32 = 256.0;
    pub const PANEL_WIDTH: f32 = 332.0;
    /// Shared centered width for the transcript, status line, and composer.
    pub const CHAT_COLUMN_WIDTH: f32 = 768.0;
    pub const HEADER_HEIGHT: f32 = 44.0;
    pub const STATUS_HEIGHT: f32 = 26.0;

    pub const SPACE_1: f32 = 4.0;
    pub const SPACE_2: f32 = 8.0;
    pub const SPACE_3: f32 = 12.0;
    pub const SPACE_4: f32 = 16.0;
    pub const SPACE_5: f32 = 24.0;

    pub const RADIUS_MENU_ROW: f32 = 6.0;
    pub const RADIUS_CONTROL: f32 = 6.0;
    pub const RADIUS_CARD: f32 = 10.0;
    pub const RADIUS_DIALOG: f32 = 14.0;
    pub const RADIUS_BUBBLE: f32 = 18.0;
    pub const RADIUS_COMPOSER: f32 = 22.0;

    pub const CONTROL_SM: f32 = 24.0;
    pub const CONTROL_MD: f32 = 28.0;
    /// Form fields, which need room for a caret and a value.
    pub const CONTROL_LG: f32 = 32.0;
    pub const MENU_ROW: f32 = 30.0;

    /// One inset per surface: everything starts on this left edge.
    pub const GUTTER: f32 = 12.0;
}

/// The type table. No other sizes exist in the app.
pub mod text {
    pub const SHEET_TITLE: f32 = 17.0;
    pub const BUBBLE: f32 = 15.0;
    pub const BUBBLE_LINE: f32 = 22.0;
    pub const MENU_ITEM: f32 = 14.0;
    pub const BODY: f32 = 13.0;
    pub const META: f32 = 12.0;
    pub const META_LINE: f32 = 16.0;
    pub const TINY: f32 = 11.0;
}
