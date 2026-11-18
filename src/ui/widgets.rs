//! Shared components. Anything used twice lives here, so a row in the sidebar
//! and a row in a menu cannot drift apart.

use gpui::{
    Animation, AnimationExt, AnyElement, App, ClickEvent, FontWeight, Hsla, IntoElement,
    SharedString, Window, div, img, prelude::*, px, svg,
};
use std::time::Duration;

use crate::model::{MascotShape, Presence};
use crate::theme::{Theme, metrics, text as typo};

/// A mask-shaped icon from the embedded set, tinted to `color`.
pub fn icon(glyph: &'static str, size: f32, color: Hsla) -> AnyElement {
    svg()
        .path(SharedString::from(format!("icons/{glyph}.svg")))
        .flex_shrink_0()
        .w(px(size))
        .h(px(size))
        .text_color(color)
        .into_any_element()
}

/// An agent mascot mark tinted with the agent's configured color.
pub fn mascot(
    shape: MascotShape,
    color: Hsla,
    presence: Presence,
    size: f32,
    id: usize,
) -> AnyElement {
    let rgb = hsla_to_rgb(color);
    if presence == Presence::Working {
        let shape = shape.id_str();
        let frames = mascot_motion_frames(id);
        let animations = frames
            .iter()
            .map(|(_, milliseconds)| Animation::new(Duration::from_millis(*milliseconds)))
            .collect();
        div()
            .flex_shrink_0()
            .size(px(size))
            .with_animations(
                ("agent-mascot", id),
                animations,
                move |element, index, _| {
                    let frame = frames[index].0;
                    let path = format!(
                        "mascots/animated/{shape}-{:02x}{:02x}{:02x}-{frame}.png",
                        rgb[0], rgb[1], rgb[2]
                    );
                    element.child(
                        img(SharedString::from(path))
                            .size(px(size))
                            .flex_shrink_0(),
                    )
                },
            )
            .into_any_element()
    } else {
        img(SharedString::from(format!(
            "mascots/tint/{}-{:02x}{:02x}{:02x}.png",
            shape.id_str(), rgb[0], rgb[1], rgb[2]
        )))
        .flex_shrink_0()
        .size(px(size))
        .into_any_element()
    }
}

fn hsla_to_rgb(color: Hsla) -> [u8; 3] {
    let hue = color.h.rem_euclid(1.0) * 6.0;
    let chroma = (1.0 - (2.0 * color.l - 1.0).abs()) * color.s;
    let x = chroma * (1.0 - (hue.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hue as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = color.l - chroma / 2.0;
    [r, g, b].map(|channel| ((channel + m).clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The face strip has 29 poses: a long rest, a short blink, and three possible
/// small glances, played with calm holds between expressions.
fn mascot_motion_frames(id: usize) -> Vec<(u32, u64)> {
    let rest = 2_600 + (id as u64 % 5) * 170;
    let mut frames = vec![(0, rest)];
    frames.extend([(1, 18), (2, 18), (3, 18), (4, 35), (5, 75), (6, 75), (7, 75), (0, 50)]);
    let glance = 8 + ((id / 7) % 3) as u32 * 7;
    frames.push((0, rest));
    frames.extend([
        (glance, 45),
        (glance + 1, 45),
        (glance + 2, 45),
        (glance + 3, 700 + (id as u64 % 5) * 40),
        (glance + 4, 45),
        (glance + 5, 45),
        (glance + 6, 45),
        (0, 50),
    ]);
    frames
}

/// A room's mark: a small team glyph colored by its first member.
pub fn room_mark(color: Hsla, size: f32, dark: bool) -> AnyElement {
    div()
        .flex_shrink_0()
        .size(px(size))
        .rounded_full()
        .bg(color.opacity(if dark { 0.16 } else { 0.13 }))
        .flex()
        .items_center()
        .justify_center()
        .child(icon("users", size * 0.52, color))
        .into_any_element()
}

/// Three states, one mark. Resting is nearly silent on purpose.
pub fn presence_dot(presence: Presence, theme: &Theme, size: f32) -> AnyElement {
    let color = match presence {
        Presence::Working => theme.success,
        Presence::Waiting => theme.warning,
        Presence::Resting => theme.ink_tertiary.opacity(0.45),
    };
    div()
        .flex_shrink_0()
        .size(px(size))
        .rounded_full()
        .bg(color)
        .into_any_element()
}

/// A small static pill: a model name, a machine, a word.
pub fn chip(label: impl Into<SharedString>, theme: &Theme) -> AnyElement {
    div()
        .flex_shrink_0()
        .h(px(20.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .rounded_full()
        .border_1()
        .border_color(theme.hairline)
        .text_size(px(typo::TINY))
        .text_color(theme.ink_secondary)
        .child(label.into())
        .into_any_element()
}

/// A pill with a leading glyph, for "which machine" style facts.
pub fn chip_with_icon(
    glyph: &'static str,
    label: impl Into<SharedString>,
    theme: &Theme,
) -> AnyElement {
    div()
        .flex_shrink_0()
        .h(px(20.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .gap(px(5.0))
        .rounded_full()
        .border_1()
        .border_color(theme.hairline)
        .text_size(px(typo::TINY))
        .text_color(theme.ink_secondary)
        .child(icon(glyph, 11.0, theme.ink_tertiary))
        .child(label.into())
        .into_any_element()
}

/// The one shape an icon-only action takes.
pub fn icon_button<F>(
    id: (&'static str, usize),
    glyph: &'static str,
    theme: &Theme,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let hover = theme.hover_wash();
    let color = theme.ink_secondary;
    div()
        .id(id)
        .flex_shrink_0()
        .size(px(26.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(metrics::RADIUS_CONTROL))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .child(icon(glyph, 16.0, color))
        .into_any_element()
}

/// A pill that is one choice among a few: skins, interface mode, machines.
/// Selection is a fill plus a stronger edge, never a hue.
pub fn segment<F>(
    id: (&'static str, usize),
    label: impl Into<SharedString>,
    selected: bool,
    theme: &Theme,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let hover = theme.hover_wash();
    div()
        .id(id)
        .flex_shrink_0()
        .h(px(26.0))
        .px(px(10.0))
        .flex()
        .items_center()
        .rounded_full()
        .border_1()
        .border_color(if selected { theme.edge } else { theme.hairline })
        .when(selected, |this| this.bg(theme.ink.opacity(0.07)))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .text_size(px(typo::TINY))
        .font_weight(if selected {
            FontWeight::MEDIUM
        } else {
            FontWeight::NORMAL
        })
        .text_color(if selected {
            theme.ink
        } else {
            theme.ink_secondary
        })
        .on_click(on_click)
        .child(label.into())
        .into_any_element()
}

/// A two-state switch. The track carries the state; nothing animates.
pub fn switch<F>(id: (&'static str, usize), on: bool, theme: &Theme, on_click: F) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let track = if on {
        theme.ink.opacity(0.85)
    } else {
        theme.ink.opacity(0.16)
    };
    let knob = if on {
        theme.on_accent
    } else {
        theme.ink_secondary
    };
    div()
        .id(id)
        .flex_shrink_0()
        .w(px(34.0))
        .h(px(20.0))
        .p(px(2.0))
        .flex()
        .items_center()
        .when(on, |this| this.justify_end())
        .rounded_full()
        .bg(track)
        .cursor_pointer()
        .on_click(on_click)
        .child(div().size(px(16.0)).rounded_full().bg(knob))
        .into_any_element()
}

/// A keyboard key, as text. Mono, so the letters sit on one baseline.
pub fn keycap(label: impl Into<SharedString>, theme: &Theme) -> AnyElement {
    div()
        .flex_shrink_0()
        .h(px(20.0))
        .px(px(6.0))
        .flex()
        .items_center()
        .rounded(px(5.0))
        .bg(theme.ink.opacity(0.05))
        .border_1()
        .border_color(theme.hairline)
        .font_family(theme.font_mono.clone())
        .text_size(px(typo::TINY))
        .text_color(theme.ink_secondary)
        .child(label.into())
        .into_any_element()
}

/// A group label. Sentence case, 12px, medium — never ALL CAPS.
pub fn section_label(label: impl Into<SharedString>, theme: &Theme) -> AnyElement {
    div()
        .text_size(px(typo::META))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.ink_tertiary)
        .child(label.into())
        .into_any_element()
}

/// A 1px vertical rule between panes.
pub fn edge(color: Hsla) -> AnyElement {
    div()
        .w(px(1.0))
        .h_full()
        .flex_shrink_0()
        .bg(color)
        .into_any_element()
}

/// A usage bar: one fill, one track, tabular numbers beside it.
pub fn meter(fraction: f32, theme: &Theme) -> AnyElement {
    let fraction = fraction.clamp(0.0, 1.0);
    let track = theme.ink.opacity(if theme.is_dark() { 0.08 } else { 0.07 });
    div()
        .h(px(4.0))
        .w_full()
        .rounded_full()
        .bg(track)
        .child(
            div()
                .h(px(4.0))
                .rounded_full()
                .bg(theme.ink_secondary.opacity(0.85))
                .w(gpui::relative(fraction)),
        )
        .into_any_element()
}

/// `769k` style numbers, so the meter does not jitter.
pub fn compact_count(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{}k", value / 1_000)
    } else {
        value.to_string()
    }
}
