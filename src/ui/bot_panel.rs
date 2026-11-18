//! The bot panel: everything about one bot, in one place.
//!
//! Settings that belong to a single bot live here, not in app settings. Rows
//! that look interactive are interactive — each one changes something and the
//! change is visible immediately.

use gpui::{
    AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, px,
};

use crate::model::Computer;
use crate::theme::{Appearance, Theme, metrics, text as typo};
use crate::ui::shell::Shell;
use crate::ui::widgets::{self, compact_count};

impl Shell {
    pub(crate) fn view_bot_panel(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let chat = self.workspace.chat().clone();
        let Some(bot_id) = chat.bot else {
            return self.room_panel(theme, cx, &chat);
        };
        let Some(bot) = self.workspace.bot(bot_id).cloned() else {
            return div().w(px(metrics::PANEL_WIDTH)).into_any_element();
        };

        let color = theme.identity(bot.color);
        let hover = theme.hover_wash();

        let header = div()
            .h(px(metrics::HEADER_HEIGHT))
            .flex_shrink_0()
            .px(px(metrics::SPACE_4))
            .flex()
            .items_center()
            .gap(px(metrics::SPACE_1))
            .child(div().flex_1().child(widgets::section_label("Bot", theme)))
            .child(widgets::icon_button(
                ("edit-bot", 0),
                "edit",
                theme,
                cx.listener(move |this, _event, window, cx| this.open_edit_bot(bot_id, window, cx)),
            ))
            .child(widgets::icon_button(
                ("close-panel", 0),
                "x",
                theme,
                cx.listener(|this, _event, _window, cx| this.toggle_panel(cx)),
            ));

        let identity = div()
            .flex()
            .gap(px(metrics::SPACE_3))
            .items_center()
            .child(widgets::mascot(
                bot.mascot,
                color,
                bot.presence,
                40.0,
                bot.id as usize * 1000 + 5,
            ))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(typo::SHEET_TITLE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(bot.name.to_string()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(typo::META))
                            .text_color(theme.ink_tertiary)
                            .child(format!("{} · {}", bot.role, bot.presence.label())),
                    ),
            );

        // Answers with
        let model_row = self.panel_row(
            theme,
            cx,
            "model-row",
            "Model",
            bot.model.to_string(),
            true,
            move |this, cx| this.cycle_model(bot_id, cx),
        );

        // Works on
        let computer_row = self.panel_row(
            theme,
            cx,
            "computer-row",
            "Computer",
            bot.computer.label().to_string(),
            true,
            move |this, cx| this.toggle_computer(bot_id, cx),
        );
        let folder_row = div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(metrics::MENU_ROW))
            .child(
                div()
                    .text_size(px(typo::BODY))
                    .text_color(theme.ink_secondary)
                    .child("Folder"),
            )
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .text_right()
                    .font_family(theme.font_mono.clone())
                    .text_size(px(typo::TINY))
                    .text_color(theme.ink_tertiary)
                    .child(bot.folder.to_string()),
            );

        let instructions = div()
            .p(px(metrics::SPACE_3))
            .rounded(px(metrics::RADIUS_CARD))
            .bg(theme.inset)
            .text_size(px(typo::META))
            .line_height(px(typo::META_LINE + 2.0))
            .text_color(theme.ink_secondary)
            .child(bot.instructions.to_string());

        let mut apps: Vec<AnyElement> = Vec::new();
        for (index, app) in crate::model::KNOWN_APPS.iter().enumerate() {
            let connected = bot.apps.iter().any(|name| name.as_ref() == *app);
            let chip_hover = hover;
            let fg = if connected {
                theme.ink
            } else {
                theme.ink_tertiary
            };
            let border = if connected {
                theme.edge
            } else {
                theme.hairline
            };
            apps.push(
                div()
                    .id(("app-chip", index))
                    .h(px(24.0))
                    .px(px(metrics::SPACE_2))
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .rounded_full()
                    .border_1()
                    .border_color(border)
                    .when(connected, |this| this.bg(theme.ink.opacity(0.05)))
                    .cursor_pointer()
                    .hover(move |style| style.bg(chip_hover))
                    .text_size(px(typo::TINY))
                    .text_color(fg)
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.toggle_app(bot_id, index, cx)
                    }))
                    .when(connected, |this| {
                        this.child(widgets::icon("check", 10.0, theme.success))
                    })
                    .child(*app)
                    .into_any_element(),
            );
        }

        let schedule = bot.schedule.clone();
        let schedule_row = div()
            .id("schedule-row")
            .h(px(metrics::MENU_ROW))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(metrics::SPACE_2))
            .rounded(px(metrics::RADIUS_MENU_ROW))
            .px(px(metrics::SPACE_1))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_click(
                cx.listener(move |this, _event, _window, cx| this.toggle_schedule(bot_id, cx)),
            )
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .text_size(px(typo::META))
                    .text_color(if schedule.is_some() {
                        theme.ink_secondary
                    } else {
                        theme.ink_tertiary
                    })
                    .child(schedule.unwrap_or_else(|| "Off".into()).to_string()),
            )
            .child(widgets::icon(
                "clock",
                13.0,
                if bot.schedule.is_some() {
                    theme.ink_secondary
                } else {
                    theme.ink_tertiary
                },
            ));

        let usage_fraction = (bot.tokens as f32 / 500_000.0).clamp(0.0, 1.0);
        let usage = div()
            .flex()
            .flex_col()
            .gap(px(metrics::SPACE_2))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(typo::META))
                            .text_color(theme.ink_secondary)
                            .child(format!("{} tokens this month", compact_count(bot.tokens))),
                    )
                    .child(
                        div()
                            .id("reset-usage")
                            .h(px(20.0))
                            .px(px(metrics::SPACE_2))
                            .flex()
                            .items_center()
                            .rounded_full()
                            .cursor_pointer()
                            .hover(move |style| style.bg(hover))
                            .text_size(px(typo::TINY))
                            .text_color(theme.ink_tertiary)
                            .on_click(cx.listener(move |this, _event, _window, cx| {
                                this.reset_usage(bot_id, cx)
                            }))
                            .child("Reset"),
                    ),
            )
            .child(widgets::meter(usage_fraction, theme));

        let skin_row = div()
            .flex()
            .gap(px(metrics::SPACE_1))
            .child(self.skin_button(theme, cx, Appearance::Dark))
            .child(self.skin_button(theme, cx, Appearance::Light));

        div()
            .w(px(metrics::PANEL_WIDTH))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(theme.panel)
            .child(header)
            .child(
                div()
                    .id("bot-panel-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .px(px(metrics::SPACE_4))
                    .pb(px(metrics::SPACE_4))
                    .flex()
                    .flex_col()
                    .gap(px(metrics::SPACE_5))
                    .child(identity)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_1))
                            .child(widgets::section_label("Answers with", theme))
                            .child(model_row),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_1))
                            .child(widgets::section_label("Works on", theme))
                            .child(computer_row)
                            .child(folder_row),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_2))
                            .child(widgets::section_label("Instructions", theme))
                            .child(instructions),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_2))
                            .child(widgets::section_label("Connected apps", theme))
                            .child(div().flex().flex_wrap().gap(px(6.0)).children(apps)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_1))
                            .child(widgets::section_label("Schedule", theme))
                            .child(schedule_row),
                    )
                    // Token counts are an Advanced detail.
                    .when(!self.settings.mode.is_simple(), |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(metrics::SPACE_2))
                                .child(widgets::section_label("Usage", theme))
                                .child(usage),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_2))
                            .child(widgets::section_label("Skin", theme))
                            .child(skin_row),
                    ),
            )
            .into_any_element()
    }

    fn room_panel(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        chat: &crate::model::Chat,
    ) -> AnyElement {
        let room_id = chat.id;
        let mut members: Vec<AnyElement> = Vec::new();
        for id in &chat.members {
            let Some(bot) = self.workspace.bot(*id) else {
                continue;
            };
            let bot_id = bot.id;
            let color = theme.identity(bot.color);
            members.push(
                div()
                    .h(px(40.0))
                    .flex()
                    .items_center()
                    .gap(px(metrics::SPACE_2))
                    .child(widgets::mascot(
                        bot.mascot,
                        color,
                        bot.presence,
                        26.0,
                        bot.id as usize * 1000 + 6,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_size(px(typo::BODY))
                            .text_color(theme.ink)
                            .child(bot.name.to_string()),
                    )
                    .child(widgets::presence_dot(bot.presence, theme, 6.0))
                    .child(widgets::icon_button(
                        ("remove-room-member", bot.id as usize),
                        "x",
                        theme,
                        cx.listener(move |this, _event, _window, cx| {
                            this.remove_bot_from_room(room_id, bot_id, cx)
                        }),
                    ))
                    .into_any_element(),
            );
        }

        let mut available_bots: Vec<AnyElement> = Vec::new();
        for bot in self
            .workspace
            .bots
            .iter()
            .filter(|bot| !chat.members.contains(&bot.id))
        {
            let bot_id = bot.id;
            available_bots.push(
                div()
                    .h(px(36.0))
                    .flex()
                    .items_center()
                    .gap(px(metrics::SPACE_2))
                    .child(widgets::mascot(
                        bot.mascot,
                        theme.identity(bot.color),
                        bot.presence,
                        24.0,
                        bot.id as usize * 1000 + 7,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_size(px(typo::META))
                            .text_color(theme.ink_secondary)
                            .child(bot.name.to_string()),
                    )
                    .child(widgets::icon_button(
                        ("add-room-member", bot_id as usize),
                        "plus",
                        theme,
                        cx.listener(move |this, _event, _window, cx| {
                            this.add_bot_to_room(room_id, bot_id, cx)
                        }),
                    ))
                    .into_any_element(),
            );
        }

        let header = div()
            .h(px(metrics::HEADER_HEIGHT))
            .flex_shrink_0()
            .px(px(metrics::SPACE_4))
            .flex()
            .items_center()
            .justify_between()
            .child(widgets::section_label("Team", theme))
            .child(widgets::icon_button(
                ("close-panel", 1),
                "x",
                theme,
                cx.listener(|this, _event, _window, cx| this.toggle_panel(cx)),
            ));

        div()
            .w(px(metrics::PANEL_WIDTH))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(theme.panel)
            .child(header)
            .child(
                div()
                    .flex_1()
                    .px(px(metrics::SPACE_4))
                    .flex()
                    .flex_col()
                    .gap(px(metrics::SPACE_4))
                    .child(
                        div()
                            .text_size(px(typo::SHEET_TITLE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(chat.title.to_string()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_1))
                            .child(widgets::section_label("Bots in this team", theme))
                            .children(members)
                            .when(chat.members.is_empty(), |this| {
                                this.child(
                                    div()
                                        .py(px(metrics::SPACE_1))
                                        .text_size(px(typo::META))
                                        .text_color(theme.ink_tertiary)
                                        .child("Add bots below to build this team."),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(metrics::SPACE_1))
                            .child(widgets::section_label("Add bots", theme))
                            .children(available_bots),
                    )
                    .child(
                        div()
                            .text_size(px(typo::META))
                            .line_height(px(typo::META_LINE + 2.0))
                            .text_color(theme.ink_tertiary)
                            .child("Everyone in a team shares one chat. Add or remove bots as the work changes."),
                    ),
            )
            .into_any_element()
    }

    fn skin_button(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        appearance: Appearance,
    ) -> AnyElement {
        let selected = self.appearance == appearance;
        let hover = theme.hover_wash();
        div()
            .id(("skin", if appearance.is_dark() { 0usize } else { 1usize }))
            .h(px(metrics::CONTROL_MD))
            .px(px(metrics::SPACE_3))
            .flex()
            .items_center()
            .gap(px(5.0))
            .rounded_full()
            .border_1()
            .border_color(if selected { theme.edge } else { theme.hairline })
            .when(selected, |this| this.bg(theme.ink.opacity(0.06)))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .text_size(px(typo::TINY))
            .text_color(if selected {
                theme.ink
            } else {
                theme.ink_tertiary
            })
            .on_click(cx.listener(move |this, _event, _window, cx| {
                if this.appearance != appearance {
                    this.toggle_appearance(cx);
                }
            }))
            .child(widgets::icon(
                if appearance.is_dark() { "moon" } else { "sun" },
                11.0,
                if selected {
                    theme.ink
                } else {
                    theme.ink_tertiary
                },
            ))
            .child(appearance.label())
            .into_any_element()
    }

    fn panel_row<F>(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        id: &'static str,
        label: &'static str,
        value: String,
        interactive: bool,
        on_click: F,
    ) -> AnyElement
    where
        F: Fn(&mut Shell, &mut Context<Shell>) + 'static,
    {
        let hover = theme.hover_wash();
        div()
            .id(id)
            .h(px(metrics::MENU_ROW))
            .px(px(metrics::SPACE_1))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(metrics::SPACE_2))
            .rounded(px(metrics::RADIUS_MENU_ROW))
            .when(interactive, |this| {
                this.cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .on_click(cx.listener(move |this, _event, _window, cx| on_click(this, cx)))
            })
            .child(
                div()
                    .text_size(px(typo::BODY))
                    .text_color(theme.ink_secondary)
                    .child(label),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(metrics::SPACE_1))
                    .child(
                        div()
                            .max_w(px(190.0))
                            .truncate()
                            .text_size(px(typo::META))
                            .text_color(theme.ink)
                            .child(value),
                    )
                    .child(widgets::icon("chevron-right", 12.0, theme.ink_tertiary)),
            )
            .into_any_element()
    }

    /// Kept for the room panel's member list ordering.
    #[allow(dead_code)]
    fn computer_label(computer: Computer) -> &'static str {
        computer.label()
    }
}
