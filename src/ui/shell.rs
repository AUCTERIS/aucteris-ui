//! The app shell: sidebar, chat, bot panel — the three places everything lives.
//!
//! The shell owns the workspace, the composer draft and the run that is in
//! flight. Views are plain methods on it (sidebar.rs, chat.rs, composer.rs,
//! bot_panel.rs) so a click can mutate the one source of truth without an
//! event bus.

use gpui::{
    App, Context, FocusHandle, Focusable, KeyDownEvent, Render, ScrollHandle, SharedString, Window,
    div, prelude::*, px,
};

use crate::engine::{self, Action};
use crate::model::{self, Author, Computer, Message, Part, Presence, Settings, Workspace};
use crate::theme::{Appearance, Theme, metrics, text as typo};
use crate::ui::dialog::Dialog;

pub struct Shell {
    pub(crate) workspace: Workspace,
    pub(crate) appearance: Appearance,

    /// What the person is typing in the composer.
    pub(crate) draft: String,
    /// What they are typing in the sidebar search.
    pub(crate) search: String,
    pub(crate) searching: bool,
    pub(crate) panel_open: bool,

    pub(crate) focus: FocusHandle,
    pub(crate) search_focus: FocusHandle,
    pub(crate) dialog_focus: FocusHandle,
    pub(crate) scroll: ScrollHandle,

    /// App-wide preferences. One bot's settings live in its own panel.
    pub(crate) settings: Settings,
    /// The open dialog, if any. Only one can be open at a time.
    pub(crate) dialog: Option<Dialog>,

    /// Bumped to abandon a run in flight. The loop compares it after every
    /// sleep, so Stop needs no cancellation plumbing.
    generation: u64,
    pub(crate) run_bot: Option<u32>,
    /// The line the status strip shows while a run is live.
    pub(crate) status: Option<SharedString>,
    focus_pending: bool,
    /// Whether the voice-call HUD is active.
    pub(crate) voice_call_active: bool,
}

impl Shell {
    pub fn new(appearance: Appearance, cx: &mut Context<Self>) -> Self {
        Self {
            workspace: model::seed(),
            appearance,
            draft: String::new(),
            search: String::new(),
            searching: false,
            panel_open: false,
            focus: cx.focus_handle(),
            search_focus: cx.focus_handle(),
            dialog_focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            settings: Settings::default(),
            dialog: None,
            generation: 0,
            run_bot: None,
            status: None,
            focus_pending: true,
            voice_call_active: false,
        }
    }

    pub(crate) fn toggle_voice_call(&mut self, cx: &mut Context<Self>) {
        self.voice_call_active = !self.voice_call_active;
        cx.notify();
    }

    pub(crate) fn create_room(&mut self, cx: &mut Context<Self>) {
        let room_count = self.workspace.chats.iter().filter(|chat| chat.is_room()).count();
        let title = if room_count == 0 {
            "New team".to_string()
        } else {
            format!("New team {}", room_count + 1)
        };
        let id = self.workspace.chats.iter().map(|chat| chat.id).max().unwrap_or(0) + 1;
        self.workspace.chats.push(model::Chat {
            id,
            title: title.clone().into(),
            bot: None,
            members: Vec::new(),
            messages: Vec::new(),
            preview: "Add bots to start this team".into(),
            time: "now".into(),
        });
        self.workspace.selected = self.workspace.chats.len() - 1;
        self.panel_open = true;
        self.scroll.scroll_to_bottom();
        cx.notify();
    }

    pub(crate) fn add_bot_to_room(&mut self, room_id: u32, bot_id: u32, cx: &mut Context<Self>) {
        let Some(room) = self
            .workspace
            .chats
            .iter_mut()
            .find(|chat| chat.id == room_id && chat.is_room())
        else {
            return;
        };
        if !room.members.contains(&bot_id) {
            room.members.push(bot_id);
            room.preview = format!("{} bots in this team", room.members.len()).into();
            cx.notify();
        }
    }

    pub(crate) fn remove_bot_from_room(
        &mut self,
        room_id: u32,
        bot_id: u32,
        cx: &mut Context<Self>,
    ) {
        let Some(room) = self
            .workspace
            .chats
            .iter_mut()
            .find(|chat| chat.id == room_id && chat.is_room())
        else {
            return;
        };
        room.members.retain(|id| *id != bot_id);
        room.preview = if room.members.is_empty() {
            "Add bots to start this team".into()
        } else {
            format!("{} bots in this team", room.members.len()).into()
        };
        cx.notify();
    }

    pub(crate) fn running(&self) -> bool {
        self.status.is_some() && self.run_bot.is_some()
    }

    // ---- selection -------------------------------------------------------

    pub(crate) fn select_chat(&mut self, index: usize, cx: &mut Context<Self>) {
        self.workspace.select(index);
        self.searching = false;
        self.search.clear();
        self.scroll.scroll_to_bottom();
        cx.notify();
    }

    /// The sidebar row for a bot, or a room, by title.
    pub(crate) fn chat_index_for_bot(&self, bot: u32) -> Option<usize> {
        self.workspace
            .chats
            .iter()
            .position(|chat| chat.bot == Some(bot))
    }

    // ---- the run ---------------------------------------------------------

    /// Send the composer's contents to whoever should answer.
    pub(crate) fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running() {
            return;
        }
        let prompt = self.draft.trim().to_string();
        if prompt.is_empty() {
            return;
        }
        let Some(bot_id) = self.workspace.responder() else {
            return;
        };
        let Some(bot) = self.workspace.bot(bot_id) else {
            return;
        };
        let plan = engine::respond(&prompt, bot);

        self.draft.clear();
        self.workspace.tokens_used += engine::estimate_tokens(&prompt);
        let chat = self.workspace.chat_mut();
        chat.messages.push(Message {
            author: Author::You,
            bot: None,
            parts: vec![Part::Text(prompt)],
            time: "now".into(),
            streaming: false,
            stopped: false,
        });
        chat.messages.push(Message {
            author: Author::Bot,
            bot: Some(bot_id),
            parts: Vec::new(),
            time: "now".into(),
            streaming: true,
            stopped: false,
        });
        if let Some(bot) = self.workspace.bot_mut(bot_id) {
            bot.presence = Presence::Working;
            bot.activity = "Starting…".into();
        }
        self.workspace.replace_last_preview();
        self.status = Some("Starting…".into());
        self.run_bot = Some(bot_id);
        self.generation += 1;
        let generation = self.generation;
        self.scroll.scroll_to_bottom();

        cx.spawn_in(window, async move |this, cx| {
            for action in plan {
                let pace = engine::pace(&action);
                cx.background_executor().timer(pace).await;
                match this.update(cx, |shell, _cx| shell.generation == generation) {
                    Ok(true) => {}
                    _ => return,
                }
                if this
                    .update(cx, |shell, cx| shell.apply(action, cx))
                    .is_err()
                {
                    return;
                }
            }
            let _ = this.update(cx, |shell, cx| shell.finish_run(cx));
        })
        .detach();
        cx.notify();
    }

    fn apply(&mut self, action: Action, cx: &mut Context<Self>) {
        let bot_id = self.run_bot;
        match action {
            Action::Intent(activity) => {
                self.status = Some(activity.into());
                if let Some(bot) = bot_id.and_then(|id| self.workspace.bot_mut(id)) {
                    bot.activity = activity.into();
                }
            }
            Action::Say(text) => {
                if let Some(message) = self.workspace.chat_mut().messages.last_mut() {
                    message.push_text(&text);
                }
            }
            Action::Tool {
                icon,
                label,
                detail,
                activity,
            } => {
                self.status = Some(activity.into());
                if let Some(bot) = bot_id.and_then(|id| self.workspace.bot_mut(id)) {
                    bot.activity = activity.into();
                }
                if let Some(message) = self.workspace.chat_mut().messages.last_mut() {
                    message.push_part(Part::Tool {
                        icon,
                        label: label.to_string(),
                        detail: detail.map(str::to_string),
                        done: false,
                        result: None,
                    });
                }
            }
            Action::Settle(result) => {
                if let Some(message) = self.workspace.chat_mut().messages.last_mut() {
                    message.finish_last_tool(Some(result.to_string()));
                }
            }
            Action::Files {
                path,
                added,
                removed,
            } => {
                if let Some(message) = self.workspace.chat_mut().messages.last_mut() {
                    message.push_part(Part::Files {
                        path: path.to_string(),
                        added,
                        removed,
                        undone: false,
                    });
                }
            }
            Action::Note(text) => {
                if let Some(message) = self.workspace.chat_mut().messages.last_mut() {
                    message.push_part(Part::Note {
                        text: text.to_string(),
                        undone: false,
                    });
                }
            }
        }
        self.workspace.replace_last_preview();
        self.scroll.scroll_to_bottom();
        cx.notify();
    }

    fn finish_run(&mut self, cx: &mut Context<Self>) {
        if let Some(message) = self.workspace.chat_mut().messages.last_mut() {
            message.streaming = false;
            message.finish_last_tool(None);
        }
        if let Some(bot_id) = self.run_bot {
            if let Some(bot) = self.workspace.bot_mut(bot_id) {
                bot.presence = Presence::Resting;
                bot.activity = "Replied".into();
                bot.tokens += 900;
            }
            self.workspace.tokens_used += 900;
        }
        self.run_bot = None;
        self.status = None;
        self.workspace.replace_last_preview();
        cx.notify();
    }

    /// Stop the run in flight. The answer keeps whatever arrived, marked as
    /// stopped, so nothing already useful disappears.
    pub(crate) fn stop_run(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        if let Some(message) = self.workspace.chat_mut().messages.last_mut() {
            if message.streaming {
                message.streaming = false;
                message.stopped = true;
                message.finish_last_tool(Some("Stopped".into()));
            }
        }
        if let Some(bot_id) = self.run_bot.take() {
            if let Some(bot) = self.workspace.bot_mut(bot_id) {
                bot.presence = Presence::Resting;
                bot.activity = "Stopped on request".into();
            }
        }
        self.status = None;
        self.workspace.replace_last_preview();
        cx.notify();
    }

    // ---- bot panel actions ----------------------------------------------

    pub(crate) const MODELS: [&'static str; 4] = [
        "Claude Sonnet 4.6",
        "Claude Opus 4.6",
        "GPT-5.2",
        "Qwen3.6 32B (local)",
    ];

    pub(crate) fn cycle_model(&mut self, bot_id: u32, cx: &mut Context<Self>) {
        if let Some(bot) = self.workspace.bot_mut(bot_id) {
            let current = Self::MODELS
                .iter()
                .position(|model| *model == bot.model.as_ref())
                .unwrap_or(0);
            let next = Self::MODELS[(current + 1) % Self::MODELS.len()];
            bot.model = next.into();
            bot.tokens += 120;
        }
        cx.notify();
    }

    pub(crate) fn toggle_computer(&mut self, bot_id: u32, cx: &mut Context<Self>) {
        if let Some(bot) = self.workspace.bot_mut(bot_id) {
            bot.computer = match bot.computer {
                Computer::ThisComputer => Computer::CloudDesktop,
                Computer::CloudDesktop => Computer::ThisComputer,
            };
        }
        cx.notify();
    }

    pub(crate) fn toggle_app(&mut self, bot_id: u32, index: usize, cx: &mut Context<Self>) {
        let Some(app) = crate::model::KNOWN_APPS.get(index) else {
            return;
        };
        if let Some(bot) = self.workspace.bot_mut(bot_id) {
            if bot.apps.iter().any(|existing| existing.as_ref() == *app) {
                bot.apps.retain(|existing| existing.as_ref() != *app);
            } else {
                bot.apps.push(SharedString::from(*app));
            }
        }
        cx.notify();
    }

    pub(crate) fn toggle_schedule(&mut self, bot_id: u32, cx: &mut Context<Self>) {
        if let Some(bot) = self.workspace.bot_mut(bot_id) {
            if bot.schedule.is_some() {
                bot.schedule = None;
            } else {
                bot.schedule = Some("Daily: check in at 09:00".into());
            }
        }
        cx.notify();
    }

    pub(crate) fn reset_usage(&mut self, bot_id: u32, cx: &mut Context<Self>) {
        if let Some(bot) = self.workspace.bot_mut(bot_id) {
            bot.tokens = 0;
        }
        cx.notify();
    }

    /// Take back a file edit or an out-of-chat change, in place.
    pub(crate) fn undo_part(
        &mut self,
        chat_index: usize,
        message_index: usize,
        part_index: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(chat) = self.workspace.chats.get_mut(chat_index) {
            if let Some(message) = chat.messages.get_mut(message_index) {
                match message.parts.get_mut(part_index) {
                    Some(Part::Files { undone, .. }) | Some(Part::Note { undone, .. }) => {
                        *undone = true;
                    }
                    _ => {}
                }
            }
        }
        cx.notify();
    }

    // ---- chrome ----------------------------------------------------------

    pub(crate) fn toggle_panel(&mut self, cx: &mut Context<Self>) {
        self.panel_open = !self.panel_open;
        cx.notify();
    }

    pub(crate) fn toggle_appearance(&mut self, cx: &mut Context<Self>) {
        self.appearance = self.appearance.toggled();
        Theme::install(self.appearance, cx);
        cx.notify();
    }

    // ---- keys ------------------------------------------------------------

    fn shortcuts(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // A dialog owns the keyboard while it is open.
        if self.dialog.is_some() {
            return;
        }
        let key = event.keystroke.key.as_str();
        match key {
            "j" => self.toggle_appearance(cx),
            "i" => self.toggle_panel(cx),
            "n" => self.open_new_bot(window, cx),
            "f" => {
                self.searching = true;
                window.focus(&self.search_focus, cx);
                cx.notify();
            }
            _ => {}
        }
    }

    /// The composer's key handler: type, delete, send.
    pub(crate) fn on_composer_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        // Ctrl+Enter is the send chord when Enter is a line break.
        if modifiers.control || modifiers.platform {
            if event.keystroke.key == "enter" && !self.settings.send_on_enter {
                self.submit(window, cx);
            }
            return;
        }
        match event.keystroke.key.as_str() {
            // Shift+Enter always keeps the line break.
            "enter" if modifiers.shift => {
                self.draft.push('\n');
                cx.notify();
            }
            "enter" if self.settings.send_on_enter => self.submit(window, cx),
            "enter" => {
                self.draft.push('\n');
                cx.notify();
            }
            "backspace" => {
                self.draft.pop();
                cx.notify();
            }
            "escape" => {
                self.draft.clear();
                cx.notify();
            }
            key => {
                let typed = event
                    .keystroke
                    .key_char
                    .clone()
                    .or_else(|| (key == "space").then(|| " ".to_string()));
                if let Some(typed) = typed.filter(|text| !text.is_empty()) {
                    self.draft.push_str(&typed);
                    cx.notify();
                }
            }
        }
    }

    /// The sidebar search's key handler.
    pub(crate) fn on_search_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform {
            return;
        }
        match event.keystroke.key.as_str() {
            "enter" => {
                self.searching = false;
                window.focus(&self.focus, cx);
                cx.notify();
            }
            "escape" => {
                self.search.clear();
                self.searching = false;
                window.focus(&self.focus, cx);
                cx.notify();
            }
            "backspace" => {
                self.search.pop();
                cx.notify();
            }
            key => {
                let typed = event
                    .keystroke
                    .key_char
                    .clone()
                    .or_else(|| (key == "space").then(|| " ".to_string()));
                if let Some(typed) = typed.filter(|text| !text.is_empty()) {
                    self.search.push_str(&typed);
                    cx.notify();
                }
            }
        }
    }

    /// Shortcuts work from anywhere, including an empty window.
    fn on_root_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.control || event.keystroke.modifiers.platform {
            self.shortcuts(event, window, cx);
        }
    }
}

impl Focusable for Shell {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.focus_pending {
            self.focus_pending = false;
            window.focus(&self.focus, cx);
        }
        let theme = Theme::of(cx).clone();

        let sidebar = self.view_sidebar(&theme, cx);
        let chat = self.view_chat(&theme, window, cx);
        let overlay = self.view_overlay(&theme, cx);
        let main_pane = div()
            .flex_1()
            .min_w_0()
            .mt(px(metrics::SPACE_2))
            .mb(px(metrics::SPACE_2))
            .ml(px(metrics::SPACE_2))
            .when(!self.panel_open, |this| this.mr(px(metrics::SPACE_2)))
            .flex()
            .overflow_hidden()
            .rounded(px(metrics::RADIUS_CARD))
            .border_1()
            .border_color(theme.edge)
            .bg(theme.bg)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .overflow_hidden()
                    .child(chat),
            );

        let right_panel = self.panel_open.then(|| {
            div()
                .w(px(metrics::PANEL_WIDTH))
                .mt(px(metrics::SPACE_2))
                .mr(px(metrics::SPACE_2))
                .mb(px(metrics::SPACE_2))
                .ml(px(metrics::SPACE_2))
                .flex_shrink_0()
                .flex()
                .overflow_hidden()
                .rounded(px(metrics::RADIUS_CARD))
                .border_1()
                .border_color(theme.edge)
                .bg(theme.panel)
                .child(self.view_bot_panel(&theme, cx))
        });

        div()
            .id("root")
            .relative()
            .size_full()
            .flex()
            .overflow_hidden()
            .bg(theme.bg)
            .font_family(theme.font_sans.clone())
            .text_size(px(typo::BODY))
            .text_color(theme.ink)
            .on_key_down(cx.listener(Self::on_root_key))
            .child(sidebar)
            .child(main_pane)
            .children(right_panel)
            .children(overlay)
            .into_any_element()
    }
}
