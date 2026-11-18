//! The composer: a single rounded input row with attachment and send controls.

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};

use crate::theme::{Theme, metrics, text as typo};
use crate::ui::shell::Shell;
use crate::ui::widgets;

impl Shell {
    pub(crate) fn view_composer(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let focused = self.focus.is_focused(window);
        let name = self.chat_name();
        let empty = self.draft.is_empty();
        let running = self.running();

        let placeholder = format!("Message {name}…");

        // The caret sits after the text, not at the end of the row.
        let input = div()
            .min_h(px(22.0))
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .items_end()
            .gap(px(2.0))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(2.0))
                    .child(if empty {
                        div()
                            .text_size(px(typo::BUBBLE))
                            .line_height(px(typo::BUBBLE_LINE))
                            .text_color(theme.ink_tertiary)
                            .child(placeholder)
                            .into_any_element()
                    } else {
                        div()
                            .text_size(px(typo::BUBBLE))
                            .line_height(px(typo::BUBBLE_LINE))
                            .text_color(theme.ink)
                            .child(self.draft.clone())
                            .into_any_element()
                    })
                    .when(focused, |this| {
                        this.child(
                            div()
                                .flex_shrink_0()
                                .w(px(1.5))
                                .h(px(18.0))
                                .mb(px(2.0))
                                .bg(theme.ink_secondary)
                                .into_any_element(),
                        )
                    }),
            )
            .child(div().flex_1());

        let accent = theme.accent;
        let on_accent = theme.on_accent;
        let control = theme.control;
        let send = {
                let enabled = !empty && !running;
                div()
                    .id("send")
                    .flex_shrink_0()
                    .size(px(metrics::CONTROL_MD))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(if enabled { accent } else { control })
                    .when(enabled, |this| this.cursor_pointer())
                    .text_color(if enabled {
                        on_accent
                    } else {
                        theme.ink_tertiary
                    })
                    .on_click(cx.listener(|this, _event, window, cx| {
                        if !this.draft.trim().is_empty() {
                            this.submit(window, cx);
                            window.focus(&this.focus.clone(), cx);
                        }
                    }))
                    .child(widgets::icon(
                        "send",
                        15.0,
                        if enabled {
                            on_accent
                        } else {
                            theme.ink_tertiary
                        },
                    ))
                    .into_any_element()
            };

        div()
            .flex_shrink_0()
            .child(
                div()
                    .w_full()
                    .max_w(px(metrics::CHAT_COLUMN_WIDTH))
                    .mx_auto()
                    .px(px(metrics::SPACE_4))
                    .pb(px(metrics::SPACE_2))
                    .pt(px(metrics::SPACE_1))
                    .child(
                        div()
                            .id("composer")
                            .w_full()
                            .h(px(44.0))
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(8.0))
                            .rounded(px(metrics::RADIUS_COMPOSER))
                            .bg(theme.composer)
                            .border_1()
                            .border_color(if focused { theme.focus } else { theme.hairline })
                            .track_focus(&self.focus)
                            .on_key_down(cx.listener(Self::on_composer_key))
                            .on_click(cx.listener(|this, _event, window, cx| {
                                window.focus(&this.focus.clone(), cx);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .id("attach")
                                    .size(px(metrics::CONTROL_MD))
                                    .rounded_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .bg(theme.control)
                                    .on_click(cx.listener(|this, _event, _window, cx| {
                                        if !this.draft.is_empty() && !this.draft.ends_with(' ') {
                                            this.draft.push(' ');
                                        }
                                        this.draft.push_str("[attached: file] ");
                                        cx.notify();
                                    }))
                                    .child(widgets::icon("plus", 15.0, theme.ink_secondary)),
                            )
                            .child(input)
                            .child(send),
                    ),
            )
            .into_any_element()
    }
}
