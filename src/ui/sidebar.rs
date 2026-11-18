//! The sidebar: the team, the rooms, and how much is left.
//!
//! Rows carry one status mark each. There are no count badges and no "needs
//! you" summaries above the list — a bot's state belongs on its own row.

use gpui::{
    AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, px,
};

use crate::model::Presence;
use crate::theme::{Theme, metrics, text as typo};
use crate::ui::shell::Shell;
use crate::ui::widgets;

impl Shell {
    pub(crate) fn view_sidebar(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .w(px(metrics::SIDEBAR_WIDTH))
            .ml(px(metrics::SPACE_2))
            .mt(px(metrics::SPACE_2))
            .mb(px(metrics::SPACE_2))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .rounded(px(metrics::RADIUS_CARD))
            .border_1()
            .border_color(theme.edge)
            .bg(theme.panel)
            .child(self.sidebar_header(theme, cx))
            .when(self.searching || !self.search.is_empty(), |this| {
                this.child(self.sidebar_search(theme, cx))
            })
            .child(if self.searching || !self.search.is_empty() {
                self.sidebar_list(theme, cx)
            } else {
                self.sidebar_home(theme, cx)
            })
            .child(self.sidebar_footer(theme, cx))
            .into_any_element()
    }

    fn sidebar_header(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .h(px(metrics::HEADER_HEIGHT))
            .flex_shrink_0()
            .border_b_1()
            .border_color(theme.hairline)
            .px(px(metrics::SPACE_2))
            .flex()
            .items_center()
            .justify_end()
            .gap(px(metrics::SPACE_1))
            .child(
                div()
                    .id("sidebar-search-button")
                    .size(px(metrics::CONTROL_LG))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .bg(theme.control)
                    .hover(|style| style.bg(theme.raised_hover))
                    .on_click(cx.listener(|this, _event, window, cx| {
                    this.searching = true;
                    window.focus(&this.search_focus.clone(), cx);
                    cx.notify();
                    }))
                    .child(widgets::icon("search", 17.0, theme.ink)),
            )
            .child(
                div()
                    .id("sidebar-new-bot-button")
                    .size(px(metrics::CONTROL_LG))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .bg(theme.control)
                    .hover(|style| style.bg(theme.raised_hover))
                    .on_click(cx.listener(|this, _event, window, cx| this.open_new_bot(window, cx)))
                    .child(widgets::icon("plus", 19.0, theme.ink)),
            )
            .into_any_element()
    }

    fn sidebar_home(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let mut bot_rows = Vec::new();
        for bot in &self.workspace.bots {
            let Some(index) = self.chat_index_for_bot(bot.id) else { continue };
            bot_rows.push(self.sidebar_row(
                theme,
                cx,
                index,
                self.workspace.selected == index,
                bot.name.to_string(),
                bot.activity.to_string(),
                widgets::mascot(
                    bot.mascot,
                    theme.identity(bot.color),
                    bot.presence,
                    28.0,
                    bot.id as usize * 1000 + 1,
                ),
                Some(bot.presence),
                self.workspace.chats[index].time.to_string(),
            ));
        }

        let room_indices: Vec<usize> = self
            .workspace
            .chats
            .iter()
            .enumerate()
            .filter_map(|(index, chat)| chat.is_room().then_some(index))
            .collect();
        let mut room_rows = Vec::new();
        for index in room_indices {
            let chat = &self.workspace.chats[index];
            let color = chat
                .members
                .first()
                .and_then(|id| self.workspace.bot(*id))
                .map(|bot| theme.identity(bot.color))
                .unwrap_or(theme.ink_secondary);
            room_rows.push(self.sidebar_row(
                theme,
                cx,
                index,
                self.workspace.selected == index,
                chat.title.to_string(),
                if chat.members.is_empty() {
                    "Add bots to this team".to_string()
                } else {
                    format!("{} bots · {}", chat.members.len(), chat.preview)
                },
                widgets::room_mark(color, 28.0, theme.is_dark()),
                None,
                chat.time.to_string(),
            ));
        }

        let room_heading = div()
            .px(px(metrics::GUTTER))
            .pt(px(metrics::SPACE_3))
            .pb(px(metrics::SPACE_1))
            .flex()
            .items_center()
            .justify_between()
            .child(widgets::section_label("Rooms", theme))
            .child(
                div()
                    .id("create-room")
                    .size(px(metrics::CONTROL_SM))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(metrics::RADIUS_CONTROL))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.raised_hover))
                    .on_click(cx.listener(|this, _event, _window, cx| this.create_room(cx)))
                    .child(widgets::icon("plus", 14.0, theme.ink_secondary)),
            );

        div()
            .id("sidebar-home")
            .flex_1()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .child(
                div()
                    .px(px(metrics::GUTTER))
                    .pt(px(metrics::SPACE_2))
                    .pb(px(metrics::SPACE_1))
                    .child(widgets::section_label("Bots", theme)),
            )
            .children(bot_rows)
            .child(room_heading)
            .when(room_rows.is_empty(), |this| {
                this.child(
                    div()
                        .px(px(metrics::GUTTER))
                        .py(px(metrics::SPACE_2))
                        .text_size(px(typo::META))
                        .text_color(theme.ink_tertiary)
                        .child("Create a room to build a bot team."),
                )
            })
            .children(room_rows)
            .into_any_element()
    }

    fn sidebar_search(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let empty = self.search.is_empty();
        let color = if empty { theme.ink_tertiary } else { theme.ink };
        let focused = self.searching;
        div()
            .px(px(metrics::GUTTER))
            .pb(px(metrics::SPACE_2))
            .child(
                div()
                    .id("search-field")
                    .h(px(metrics::CONTROL_MD))
                    .px(px(metrics::SPACE_2))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(metrics::RADIUS_CONTROL))
                    .bg(theme.inset)
                    .cursor_text()
                    .border_1()
                    .border_color(if focused { theme.focus } else { theme.hairline })
                    .track_focus(&self.search_focus)
                    .on_key_down(cx.listener(Self::on_search_key))
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.searching = true;
                        window.focus(&this.search_focus.clone(), cx);
                        cx.notify();
                    }))
                    .child(widgets::icon(
                        "search",
                        13.0,
                        if focused {
                            theme.ink_secondary
                        } else {
                            theme.ink_tertiary
                        },
                    ))
                    .child(
                        div()
                            .flex_1()
                            .truncate()
                            .text_size(px(typo::META))
                            .text_color(color)
                            .child(if empty {
                                "Search".to_string()
                            } else {
                                self.search.clone()
                            }),
                    ),
            )
            .into_any_element()
    }

    fn sidebar_list(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let query = self.search.to_lowercase();
        let matches = |title: &str| query.is_empty() || title.to_lowercase().contains(&query);

        let mut rows: Vec<AnyElement> = Vec::new();

        rows.push(
            div()
                .px(px(metrics::GUTTER))
                .pt(px(metrics::SPACE_2))
                .pb(px(metrics::SPACE_1))
                .child(widgets::section_label("Bots", theme))
                .into_any_element(),
        );

        for bot in &self.workspace.bots {
            if !matches(bot.name.as_ref()) {
                continue;
            }
            let Some(index) = self.chat_index_for_bot(bot.id) else {
                continue;
            };
            let selected = index == self.workspace.selected;
            rows.push(self.sidebar_row(
                theme,
                cx,
                index,
                selected,
                bot.name.to_string(),
                bot.activity.to_string(),
                widgets::mascot(
                    bot.mascot,
                    theme.identity(bot.color),
                    bot.presence,
                    28.0,
                    bot.id as usize * 1000 + 3,
                ),
                Some(bot.presence),
                self.workspace.chats[index].time.to_string(),
            ));
        }

        let room_indices: Vec<usize> = self
            .workspace
            .chats
            .iter()
            .enumerate()
            .filter(|(_, chat)| chat.is_room())
            .filter(|(_, chat)| matches(chat.title.as_ref()))
            .map(|(index, _)| index)
            .collect();

        if !room_indices.is_empty() {
            rows.push(
                div()
                    .px(px(metrics::GUTTER))
                    .pt(px(metrics::SPACE_3))
                    .pb(px(metrics::SPACE_1))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(widgets::section_label("Rooms", theme))
                    .child(
                        div()
                            .id("search-create-room")
                            .size(px(metrics::CONTROL_SM))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(metrics::RADIUS_CONTROL))
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.raised_hover))
                            .on_click(cx.listener(|this, _event, _window, cx| this.create_room(cx)))
                            .child(widgets::icon("plus", 14.0, theme.ink_secondary)),
                    )
                    .into_any_element(),
            );
            for index in room_indices {
                let chat = &self.workspace.chats[index];
                let selected = index == self.workspace.selected;
                let color_slot = chat
                    .members
                    .first()
                    .and_then(|id| self.workspace.bot(*id))
                    .map(|bot| bot.color)
                    .unwrap_or(0);
                let title = chat.title.to_string();
                let preview = chat.preview.to_string();
                let time = chat.time.to_string();
                rows.push(self.sidebar_row(
                    theme,
                    cx,
                    index,
                    selected,
                    title,
                    preview,
                    widgets::room_mark(theme.identity(color_slot), 28.0, theme.is_dark()),
                    None,
                    time,
                ));
            }
        }

        if rows.len() <= 1 {
            rows.push(
                div()
                    .px(px(metrics::GUTTER))
                    .py(px(metrics::SPACE_3))
                    .text_size(px(typo::META))
                    .text_color(theme.ink_tertiary)
                    .child("Nothing matches that. Try a shorter word.")
                    .into_any_element(),
            );
        }

        div()
            .id("chat-list")
            .flex_1()
            .overflow_y_scroll()
            .pb(px(metrics::SPACE_2))
            .children(rows)
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn sidebar_row(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        index: usize,
        selected: bool,
        title: String,
        preview: String,
        mark: AnyElement,
        presence: Option<Presence>,
        time: String,
    ) -> AnyElement {
        let hover = theme.hover_wash();
        let background = if selected {
            theme.raised
        } else {
            theme.ink.opacity(0.0)
        };
        let title_color = if selected { theme.ink } else { theme.ink };
        let preview_color = if selected {
            theme.ink_secondary
        } else {
            theme.ink_tertiary
        };

        div()
            .id(("chat-row", index))
            .mx(px(metrics::SPACE_1))
            .px(px(metrics::SPACE_2))
            .h(px(50.0))
            .flex()
            .items_center()
            .gap(px(metrics::SPACE_2))
            .rounded(px(metrics::RADIUS_CONTROL))
            .bg(background)
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_click(cx.listener(move |this, _event, _window, cx| this.select_chat(index, cx)))
            .child(mark)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(typo::BODY))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(title_color)
                            .child(title),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(typo::META))
                            .text_color(preview_color)
                            .child(preview),
                    ),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(typo::TINY))
                            .text_color(theme.ink_tertiary)
                            .child(time),
                    )
                    .when_some(presence, |this, presence| {
                        this.child(widgets::presence_dot(presence, theme, 6.0))
                    }),
            )
            .into_any_element()
    }

    fn sidebar_footer(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("sidebar-actions")
            .flex_shrink_0()
            .px(px(metrics::SPACE_2))
            .py(px(metrics::SPACE_2))
            .flex()
            .items_center()
            .justify_end()
            .gap(px(metrics::SPACE_1))
            .child(
                div()
                    .id("sidebar-settings")
                    .size(px(metrics::CONTROL_LG))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(metrics::RADIUS_CONTROL))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.raised_hover))
                    .on_click(cx.listener(|this, _event, window, cx| {
                        this.open_settings(window, cx)
                    }))
                    .child(widgets::icon("sliders", 16.0, theme.ink_secondary)),
            )
            .child(
                div()
                    .id("sidebar-theme")
                    .size(px(metrics::CONTROL_LG))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(metrics::RADIUS_CONTROL))
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.raised_hover))
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.toggle_appearance(cx)
                    }))
                    .child(widgets::icon(
                        if theme.is_dark() { "sun" } else { "moon" },
                        16.0,
                        theme.ink_secondary,
                    )),
            )
            .into_any_element()
    }
}
