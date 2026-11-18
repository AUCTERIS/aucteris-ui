//! The chat: header, transcript, status strip, composer.
//!
//! A reply is one bubble-less column: the bot's words and the steps it took sit
//! in the same stream, because the work is the answer. Only the person's own
//! messages get a plate, so the eye always knows who is talking.

use gpui::{
    AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::*, px,
};

use crate::model::{Author, Message, Part};
use crate::theme::{Theme, metrics, text as typo};
use crate::ui::shell::Shell;
use crate::ui::widgets;

impl Shell {
    pub(crate) fn view_chat(
        &self,
        theme: &Theme,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.bg)
            .child(self.chat_header(theme, cx))
            .when(self.voice_call_active, |this| this.child(self.view_voice_hud(theme, cx)))
            .child(self.transcript(theme, cx))
            .when(self.running(), |this| this.child(self.status_strip(theme, cx)))
            .child(self.view_composer(theme, window, cx))
            .into_any_element()
    }

    pub(crate) fn chat_name(&self) -> String {
        self.workspace.chat().title.to_string()
    }

    fn chat_header(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let chat = self.workspace.chat().clone();
        div()
            .h(px(metrics::HEADER_HEIGHT))
            .flex_shrink_0()
            .px(px(metrics::SPACE_3))
            .bg(theme.panel)
            .border_b_1()
            .border_color(theme.hairline)
            .flex()
            .items_center()
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .h(px(metrics::CONTROL_MD))
                    .px(px(metrics::SPACE_2))
                    .rounded_full()
                    .bg(theme.card)
                    .child(if let Some(bot) = chat.bot.and_then(|id| self.workspace.bot(id)) {
                        widgets::mascot(
                            bot.mascot,
                            theme.identity(bot.color),
                            bot.presence,
                            20.0,
                            bot.id as usize * 1000 + 4,
                        )
                    } else {
                        widgets::icon("bot", 16.0, theme.ink_secondary)
                    })
                    .child(
                        div()
                            .text_size(px(typo::BODY))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.ink)
                            .child(chat.title.to_string()),
                    ),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id("toggle-right-panel")
                    .size(px(metrics::CONTROL_MD))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(metrics::RADIUS_CONTROL))
                    .when(self.panel_open, |this| this.bg(theme.raised))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.raised))
                    .on_click(cx.listener(|this, _event, _window, cx| this.toggle_panel(cx)))
                    .child(widgets::icon("panel", 16.0, theme.ink_secondary)),
            )
            .into_any_element()
    }

    fn view_voice_hud(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let chat = self.workspace.chat();
        let bot = chat.bot.and_then(|id| self.workspace.bot(id));
        let (mascot, color, name, presence, id) = match bot {
            Some(b) => (b.mascot, theme.identity(b.color), b.name.to_string(), b.presence, b.id as usize),
            None => (crate::model::MascotShape::Sparkle, theme.accent, "Grok Bot".to_string(), crate::model::Presence::Resting, 0),
        };

        div()
            .id("voice-call-hud")
            .absolute()
            .top(px(metrics::HEADER_HEIGHT + 10.0))
            .left(px(16.0))
            .right(px(16.0))
            .p(px(metrics::SPACE_4))
            .rounded(px(metrics::RADIUS_CARD))
            .bg(theme.panel.opacity(0.96))
            .border_1()
            .border_color(color.opacity(0.4))
            .flex()
            .flex_col()
            .gap(px(metrics::SPACE_3))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(metrics::SPACE_3))
                            .child(widgets::mascot(mascot, color, presence, 38.0, id + 100_000))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(1.0))
                                    .child(
                                        div()
                                            .text_size(px(typo::MENU_ITEM))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(theme.ink)
                                            .child(format!("{name} Voice Call")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(typo::TINY))
                                            .text_color(color)
                                            .child("Live · Neural audio pipeline active"),
                                    ),
                            ),
                    )
                    // Audio waveform bars
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.0))
                            .child(div().w(px(3.0)).h(px(14.0)).rounded_full().bg(color))
                            .child(div().w(px(3.0)).h(px(24.0)).rounded_full().bg(color))
                            .child(div().w(px(3.0)).h(px(10.0)).rounded_full().bg(color))
                            .child(div().w(px(3.0)).h(px(28.0)).rounded_full().bg(color))
                            .child(div().w(px(3.0)).h(px(18.0)).rounded_full().bg(color))
                            .child(div().w(px(3.0)).h(px(22.0)).rounded_full().bg(color))
                            .child(div().w(px(3.0)).h(px(12.0)).rounded_full().bg(color)),
                    )
                    // End call button
                    .child(
                        div()
                            .id("end-call-btn")
                            .px(px(metrics::SPACE_3))
                            .h(px(metrics::CONTROL_SM))
                            .rounded(px(metrics::RADIUS_CONTROL))
                            .bg(theme.danger)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _event, _window, cx| this.toggle_voice_call(cx)))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(typo::META))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(gpui::white())
                                    .child("End Call"),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn transcript(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let chat = self.workspace.chat();
        if chat.messages.is_empty() {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .px(px(metrics::SPACE_5))
                .child(
                    div()
                        .text_size(px(typo::BODY))
                        .text_color(theme.ink_tertiary)
                        .child(format!(
                            "Nothing here yet. Say what you need and {} starts.",
                            self.chat_name()
                        )),
                )
                .into_any_element();
        }

        let chat_index = self.workspace.selected;
        let mut children: Vec<AnyElement> = Vec::new();
        for (message_index, message) in chat.messages.iter().enumerate() {
            children.push(self.message_view(theme, cx, chat_index, message_index, message));
        }

        div()
            .id("transcript")
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(
                div()
                    .w_full()
                    .max_w(px(metrics::CHAT_COLUMN_WIDTH))
                    .mx_auto()
                    .px(px(metrics::SPACE_4))
                    .py(px(metrics::SPACE_1))
                    .flex()
                    .flex_col()
                    .gap(px(metrics::SPACE_2))
                    .child(
                        div()
                            .w_full()
                            .pb(px(4.0))
                            .pt(px(2.0))
                            .flex()
                            .justify_center()
                            .text_size(px(typo::TINY))
                            .text_color(theme.ink_tertiary)
                            .child("Friday 7:48 AM"),
                    )
                    .children(children),
            )
            .into_any_element()
    }

    fn message_view(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        chat_index: usize,
        message_index: usize,
        message: &Message,
    ) -> AnyElement {
        match message.author {
            Author::You => div()
                .w_full()
                .flex()
                .justify_end()
                .child(
                    div()
                        .max_w(px(560.0))
                        .px(px(12.0))
                        .py(px(9.0))
                        .rounded(px(17.0))
                        .bg(if theme.is_dark() { gpui::hsla(0.0, 0.0, 0.92, 1.0) } else { theme.raised })
                        .text_size(px(typo::BUBBLE))
                        .line_height(px(typo::BUBBLE_LINE))
                        .text_color(if theme.is_dark() { gpui::hsla(0.0, 0.0, 0.12, 1.0) } else { theme.ink })
                        .child(message.text()),
                )
                .into_any_element(),
            Author::System => div()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .text_size(px(typo::META))
                        .text_color(theme.ink_tertiary)
                        .child(message.text()),
                )
                .into_any_element(),
            Author::Bot => {
                let has_approval = message.parts.iter().any(|part| matches!(part, Part::Approval { .. }));
                let mut body: Vec<AnyElement> = Vec::new();
                for (part_index, part) in message.parts.iter().enumerate() {
                    body.push(match part {
                        Part::Text(text) => div()
                            .when(has_approval, |this| {
                                this.max_w(px(560.0))
                                    .px(px(12.0))
                                    .py(px(9.0))
                                    .rounded(px(17.0))
                                    .bg(theme.card)
                            })
                            .text_size(px(typo::BUBBLE))
                            .line_height(px(typo::BUBBLE_LINE))
                            .text_color(theme.ink)
                            .child(text.clone())
                            .into_any_element(),
                        Part::Thought { text, collapsed } => div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_1))
                            .px(px(metrics::SPACE_3))
                            .py(px(metrics::SPACE_2))
                            .rounded(px(metrics::RADIUS_CARD))
                            .bg(theme.ink.opacity(0.035))
                            .border_1()
                            .border_color(theme.hairline)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(metrics::SPACE_2))
                                    .child(widgets::icon("sparkle", 12.0, theme.accent))
                                    .child(
                                        div()
                                            .text_size(px(typo::META))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.ink_secondary)
                                            .child("Reasoning process"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(typo::TINY))
                                            .text_color(theme.ink_tertiary)
                                            .child(if *collapsed { "Show" } else { "Collapse" }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(typo::BODY))
                                    .line_height(px(typo::BUBBLE_LINE))
                                    .text_color(theme.ink_secondary)
                                    .child(text.clone()),
                            )
                            .into_any_element(),
                        Part::Tool {
                            icon,
                            label,
                            detail,
                            done,
                            result,
                        } => div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .h(px(24.0))
                                    .px(px(metrics::SPACE_2))
                                    .rounded(px(metrics::RADIUS_MENU_ROW))
                                    .bg(theme.ink.opacity(0.045))
                                    .border_1()
                                    .border_color(theme.hairline)
                                    .child(widgets::icon(
                                        icon,
                                        13.0,
                                        if *done {
                                            theme.ink_tertiary
                                        } else {
                                            theme.success
                                        },
                                    ))
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .font_family(theme.font_mono.clone())
                                            .text_size(px(typo::META))
                                            .text_color(if *done {
                                                theme.ink_secondary
                                            } else {
                                                theme.ink
                                            })
                                            .child(label.clone()),
                                    )
                                    .when_some(detail.clone(), |this, detail| {
                                        this.child(
                                            div()
                                                .truncate()
                                                .text_size(px(typo::TINY))
                                                .text_color(theme.ink_tertiary)
                                                .child(format!("· {detail}")),
                                        )
                                    }),
                            )
                            .when_some(result.clone(), |this, result| {
                                this.child(
                                    div()
                                        .px(px(metrics::SPACE_1))
                                        .text_size(px(typo::TINY))
                                        .text_color(theme.ink_tertiary)
                                        .child(result),
                                )
                            })
                            .into_any_element(),
                        Part::Files {
                            path,
                            added,
                            removed,
                            undone,
                        } => div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .h(px(26.0))
                            .child(widgets::icon("file", 13.0, theme.ink_tertiary))
                            .child(
                                div()
                                    .flex_1()
                                    .truncate()
                                    .font_family(theme.font_mono.clone())
                                    .text_size(px(typo::META))
                                    .text_color(if *undone {
                                        theme.ink_tertiary
                                    } else {
                                        theme.ink_secondary
                                    })
                                    .child(path.clone()),
                            )
                            .when(!*undone, |this| {
                                this.child(
                                    div()
                                        .text_size(px(typo::TINY))
                                        .text_color(theme.success)
                                        .child(format!("+{added}")),
                                )
                                .child(
                                    div()
                                        .text_size(px(typo::TINY))
                                        .text_color(theme.danger)
                                        .child(format!("-{removed}")),
                                )
                                .child(self.undo_button(
                                    theme,
                                    cx,
                                    chat_index,
                                    message_index,
                                    part_index,
                                ))
                            })
                            .when(*undone, |this| {
                                this.child(
                                    div()
                                        .text_size(px(typo::TINY))
                                        .text_color(theme.ink_tertiary)
                                        .child("Undone"),
                                )
                            })
                            .into_any_element(),
                        Part::Note { text, undone } => div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .h(px(26.0))
                            .child(widgets::icon("sparkle", 13.0, theme.ink_tertiary))
                            .child(
                                div()
                                    .flex_1()
                                    .truncate()
                                    .text_size(px(typo::META))
                                    .text_color(if *undone {
                                        theme.ink_tertiary
                                    } else {
                                        theme.ink_secondary
                                    })
                                    .child(text.clone()),
                            )
                            .when(!*undone, |this| {
                                this.child(self.undo_button(
                                    theme,
                                    cx,
                                    chat_index,
                                    message_index,
                                    part_index,
                                ))
                            })
                            .when(*undone, |this| {
                                this.child(
                                    div()
                                        .text_size(px(typo::TINY))
                                        .text_color(theme.ink_tertiary)
                                        .child("Undone"),
                                )
                            })
                            .into_any_element(),
                        Part::Approval { kicker, title, detail, resolved } => {
                            let pending = resolved.is_none();
                            let resolution = resolved.clone();
                            div()
                                .id(("approval-card", message_index * 32 + part_index))
                                .w(px(422.0))
                                .max_w(px(560.0))
                                .p(px(14.0))
                                .rounded(px(17.0))
                                .bg(theme.card)
                                .border_1()
                                .border_color(theme.hairline)
                                .flex()
                                .flex_col()
                                .gap(px(5.0))
                                .child(div().text_size(px(typo::TINY)).text_color(theme.ink_tertiary).child(kicker.clone()))
                                .child(div().text_size(px(typo::BODY)).text_color(theme.ink).child(title.clone()))
                                .child(div().text_size(px(typo::META)).text_color(theme.ink_tertiary).child(detail.clone()))
                                .child(
                                    div()
                                        .mt(px(5.0))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .when_some(resolution, |this, resolution| {
                                            this.child(div().text_size(px(typo::META)).text_color(theme.success).child(resolution))
                                        })
                                        .when(pending, |this| {
                                            this.child(
                                                div()
                                                    .id(("approval-send", message_index * 32 + part_index))
                                                    .h(px(30.0))
                                                    .px(px(13.0))
                                                    .rounded_full()
                                                    .bg(if theme.is_dark() { gpui::hsla(0.0, 0.0, 0.92, 1.0) } else { theme.ink })
                                                    .cursor_pointer()
                                                    .flex()
                                                    .items_center()
                                                    .text_size(px(typo::META))
                                                    .text_color(if theme.is_dark() { gpui::hsla(0.0, 0.0, 0.12, 1.0) } else { theme.bg })
                                                    .on_click(cx.listener(move |this, _event, _window, cx| {
                                                        this.resolve_approval(chat_index, message_index, part_index, "Sent it", cx)
                                                    }))
                                                    .child("Send it"),
                                            )
                                            .child(
                                                div()
                                                    .id(("approval-review", message_index * 32 + part_index))
                                                    .h(px(30.0))
                                                    .px(px(13.0))
                                                    .rounded_full()
                                                    .border_1()
                                                    .border_color(theme.hairline)
                                                    .cursor_pointer()
                                                    .flex()
                                                    .items_center()
                                                    .text_size(px(typo::META))
                                                    .text_color(theme.ink_secondary)
                                                    .on_click(cx.listener(move |this, _event, _window, cx| {
                                                        this.resolve_approval(chat_index, message_index, part_index, "Review first", cx)
                                                    }))
                                                    .child("Review first"),
                                            )
                                        }),
                                )
                                .into_any_element()
                        }
                    });
                }

                if message.streaming {
                    body.push(
                        div()
                            .mt(px(2.0))
                            .w(px(7.0))
                            .h(px(15.0))
                            .rounded(px(2.0))
                            .bg(theme.ink_secondary.opacity(0.55))
                            .into_any_element(),
                    );
                }
                if message.stopped {
                    body.push(
                        div()
                            .text_size(px(typo::META))
                            .text_color(theme.ink_tertiary)
                            .child("Stopped on request")
                            .into_any_element(),
                    );
                }

                div()
                    .w_full()
                    .flex()
                    .justify_start()
                    .child(
                        div()
                            .max_w(px(560.0))
                            .when(!has_approval, |this| {
                                this.px(px(12.0))
                                    .py(px(9.0))
                                    .rounded(px(17.0))
                                    .bg(theme.card)
                            })
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .children(body),
                    )
                    .into_any_element()
            }
        }
    }

    fn undo_button(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        chat_index: usize,
        message_index: usize,
        part_index: usize,
    ) -> AnyElement {
        let hover = theme.hover_wash();
        let color = theme.ink_secondary;
        div()
            .id(("undo", message_index * 32 + part_index))
            .flex_shrink_0()
            .h(px(20.0))
            .px(px(metrics::SPACE_2))
            .flex()
            .items_center()
            .rounded_full()
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .text_size(px(typo::TINY))
            .text_color(color)
            .on_click(cx.listener(move |this, _event, _window, cx| {
                this.undo_part(chat_index, message_index, part_index, cx)
            }))
            .child("Undo")
            .into_any_element()
    }

    pub(crate) fn resolve_approval(
        &mut self,
        chat_index: usize,
        message_index: usize,
        part_index: usize,
        choice: &'static str,
        cx: &mut Context<Self>,
    ) {
        if let Some(part) = self
            .workspace
            .chats
            .get_mut(chat_index)
            .and_then(|chat| chat.messages.get_mut(message_index))
            .and_then(|message| message.parts.get_mut(part_index))
        {
            if let Part::Approval { resolved, .. } = part {
                *resolved = Some(choice.to_string());
            }
        }
        if self.workspace.selected == chat_index {
            self.workspace.replace_last_preview();
        }
        cx.notify();
    }

    fn status_strip(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let running = self.running();
        let label: SharedString = match (&self.status, running) {
            (Some(status), true) => status.clone(),
            _ => {
                let folder = self
                    .workspace
                    .selected_bot()
                    .and_then(|id| self.workspace.bot(id))
                    .map(|bot| bot.folder.to_string())
                    .unwrap_or_else(|| "A room, shared by everyone in it".to_string());
                SharedString::from(format!("Full access · {folder}"))
            }
        };

        let hover = theme.hover_wash();
        let stop_color = theme.ink_secondary;

        div()
            .h(px(metrics::STATUS_HEIGHT))
            .flex_shrink_0()
            .child(
                div()
                    .w_full()
                    .max_w(px(metrics::CHAT_COLUMN_WIDTH))
                    .mx_auto()
                    .px(px(metrics::SPACE_4))
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(if running {
                        widgets::presence_dot(crate::model::Presence::Working, theme, 6.0)
                    } else {
                        div()
                            .size(px(6.0))
                            .rounded_full()
                            .bg(theme.ink_tertiary.opacity(0.35))
                            .into_any_element()
                    })
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_size(px(typo::TINY))
                            .text_color(theme.ink_tertiary)
                            .child(label),
                    )
                    .when(running, |this| {
                        this.child(
                            div()
                                .id("stop-run")
                                .flex_shrink_0()
                                .h(px(20.0))
                                .px(px(metrics::SPACE_2))
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .rounded_full()
                                .border_1()
                                .border_color(theme.hairline)
                                .cursor_pointer()
                                .hover(move |style| style.bg(hover))
                                .text_size(px(typo::TINY))
                                .text_color(stop_color)
                                .on_click(cx.listener(|this, _event, _window, cx| {
                                    this.stop_run(cx)
                                }))
                                .child(widgets::icon("stop", 9.0, stop_color))
                                .child("Stop"),
                        )
                    })
                    .into_any_element(),
            )
            .into_any_element()
    }
}
