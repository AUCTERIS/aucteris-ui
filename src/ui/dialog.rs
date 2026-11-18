//! Dialogs: creating a bot, and the app-wide settings.
//!
//! A dialog is the only floating surface in the app, so it is the only thing
//! that casts a shadow and the only thing behind a scrim. Both dialogs are
//! keyboard-complete: Tab moves between fields, Enter commits, Escape leaves.

use gpui::{
    AnyElement, Context, FocusHandle, FontWeight, IntoElement, KeyDownEvent, ParentElement, Styled,
    Window, div, prelude::*, px,
};

use crate::model::{self, Bot, Chat, Computer, MascotShape, Message, Mode, Part, Presence};
use crate::theme::{Theme, metrics, text as typo};
use crate::ui::shell::Shell;
use crate::ui::widgets;

/// The four text fields of the new-bot form, in Tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    Role,
    Folder,
    Instructions,
}

impl Field {
    fn index(self) -> usize {
        match self {
            Self::Name => 0,
            Self::Role => 1,
            Self::Folder => 2,
            Self::Instructions => 3,
        }
    }

    fn next(self, backwards: bool) -> Self {
        let order = [Self::Name, Self::Role, Self::Folder, Self::Instructions];
        let current = self.index();
        let next = if backwards {
            (current + order.len() - 1) % order.len()
        } else {
            (current + 1) % order.len()
        };
        order[next]
    }

    fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Role => "Role",
            Self::Folder => "Folder",
            Self::Instructions => "Instructions",
        }
    }
}

/// What the person has typed so far.
///
/// One form, two jobs: with `target` empty it creates a bot, with a bot id it
/// edits that bot in place. Fields, keys and layout are identical either way, so
/// learning one teaches the other.
pub struct BotForm {
    /// `None` creates; `Some(id)` edits that bot.
    pub target: Option<u32>,
    pub name: String,
    pub role: String,
    pub folder: String,
    pub instructions: String,
    pub model: usize,
    pub computer: Computer,
    pub color: usize,
    pub mascot: MascotShape,
    pub active: Field,
    pub focus: [FocusHandle; 4],
}

impl BotForm {
    pub fn new(model: usize, color: usize, focus: [FocusHandle; 4]) -> Self {
        Self {
            target: None,
            name: String::new(),
            role: "General".to_string(),
            folder: "~".to_string(),
            instructions: String::new(),
            model,
            computer: Computer::ThisComputer,
            color,
            mascot: MascotShape::Sparkle,
            active: Field::Name,
            focus,
        }
    }

    /// Open the form on an existing bot: every field starts at its current value.
    pub fn from_bot(bot: &Bot, focus: [FocusHandle; 4]) -> Self {
        let model = Shell::MODELS
            .iter()
            .position(|candidate| *candidate == bot.model.as_ref())
            .unwrap_or(0);
        Self {
            target: Some(bot.id),
            name: bot.name.to_string(),
            role: bot.role.to_string(),
            folder: bot.folder.to_string(),
            instructions: bot.instructions.to_string(),
            model,
            computer: bot.computer,
            color: bot.color,
            mascot: bot.mascot,
            active: Field::Name,
            focus,
        }
    }

    fn editing(&self) -> bool {
        self.target.is_some()
    }

    fn text(&self, field: Field) -> &str {
        match field {
            Field::Name => &self.name,
            Field::Role => &self.role,
            Field::Folder => &self.folder,
            Field::Instructions => &self.instructions,
        }
    }

    fn text_mut(&mut self, field: Field) -> &mut String {
        match field {
            Field::Name => &mut self.name,
            Field::Role => &mut self.role,
            Field::Folder => &mut self.folder,
            Field::Instructions => &mut self.instructions,
        }
    }

    fn can_submit(&self) -> bool {
        !self.name.trim().is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    General,
    Behavior,
    Shortcuts,
    About,
}

impl SettingsPage {
    const ALL: [SettingsPage; 4] = [
        SettingsPage::General,
        SettingsPage::Behavior,
        SettingsPage::Shortcuts,
        SettingsPage::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Behavior => "Behavior",
            Self::Shortcuts => "Shortcuts",
            Self::About => "About",
        }
    }
}

pub enum Dialog {
    Bot(BotForm),
    Settings(SettingsPage),
}

impl Shell {
    // ---- opening ---------------------------------------------------------

    fn form_focuses(cx: &mut Context<Self>) -> [FocusHandle; 4] {
        [
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
            cx.focus_handle(),
        ]
    }

    pub(crate) fn open_new_bot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = Self::form_focuses(cx);
        let color = self.workspace.bots.len() % 8;
        let model = self.settings.default_model;
        self.dialog = Some(Dialog::Bot(BotForm::new(model, color, focus)));
        window.focus(&self.dialog_focus, cx);
        cx.notify();
    }

    /// Edit a bot from its panel, through the same form that created it.
    pub(crate) fn open_edit_bot(
        &mut self,
        bot_id: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(bot) = self.workspace.bot(bot_id).cloned() else {
            return;
        };
        let focus = Self::form_focuses(cx);
        self.dialog = Some(Dialog::Bot(BotForm::from_bot(&bot, focus)));
        window.focus(&self.dialog_focus, cx);
        cx.notify();
    }

    pub(crate) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = Some(Dialog::Settings(SettingsPage::General));
        window.focus(&self.dialog_focus, cx);
        cx.notify();
    }

    pub(crate) fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(crate) fn focus_field(
        &mut self,
        field: Field,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(Dialog::Bot(form)) = self.dialog.as_mut() {
            form.active = field;
            let handle = form.focus[field.index()].clone();
            window.focus(&handle, cx);
        }
        cx.notify();
    }

    // ---- keys ------------------------------------------------------------

    pub(crate) fn on_dialog_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;

        if key == "escape" {
            self.close_dialog(window, cx);
            return;
        }
        if !matches!(self.dialog, Some(Dialog::Bot(_))) {
            return;
        }

        let mut refocus: Option<Field> = None;
        let mut submit = false;
        if let Some(Dialog::Bot(form)) = self.dialog.as_mut() {
            match key {
                "tab" => {
                    form.active = form.active.next(modifiers.shift);
                    refocus = Some(form.active);
                }
                "enter" => submit = form.can_submit(),
                "backspace" => {
                    form.text_mut(form.active).pop();
                }
                _ => {
                    if modifiers.control || modifiers.platform {
                        return;
                    }
                    let typed = event
                        .keystroke
                        .key_char
                        .clone()
                        .or_else(|| (key == "space").then(|| " ".to_string()));
                    if let Some(typed) = typed.filter(|text| !text.is_empty()) {
                        form.text_mut(form.active).push_str(&typed);
                    }
                }
            }
        }
        if let Some(field) = refocus {
            self.focus_field(field, window, cx);
        }
        if submit {
            self.commit_bot_form(window, cx);
            return;
        }
        cx.notify();
    }

    // ---- committing ------------------------------------------------------

    /// Create the bot the form describes, or write the edits back to it.
    pub(crate) fn commit_bot_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Dialog::Bot(form)) = self.dialog.take() else {
            return;
        };
        if !form.can_submit() {
            self.dialog = Some(Dialog::Bot(form));
            return;
        }

        let name = form.name.trim().to_string();
        let role = if form.role.trim().is_empty() {
            "General".to_string()
        } else {
            form.role.trim().to_string()
        };
        let folder = if form.folder.trim().is_empty() {
            "~".to_string()
        } else {
            form.folder.trim().to_string()
        };
        let instructions = if form.instructions.trim().is_empty() {
            "Ask before touching anything outside the working folder.".to_string()
        } else {
            form.instructions.trim().to_string()
        };
        let model = Shell::MODELS
            .get(form.model)
            .copied()
            .unwrap_or(Shell::MODELS[0]);

        // Editing: everything the form owns changes in place, in the same order
        // the panel shows it. The chat title follows the name.
        if let Some(id) = form.target {
            if let Some(bot) = self.workspace.bot_mut(id) {
                bot.name = name.clone().into();
                bot.role = role.clone().into();
                bot.model = model.into();
                bot.computer = form.computer;
                bot.folder = folder.clone().into();
                bot.instructions = instructions.into();
                bot.color = form.color;
                bot.mascot = form.mascot;
            }
            if let Some(chat) = self
                .workspace
                .chats
                .iter_mut()
                .find(|chat| chat.bot == Some(id))
            {
                chat.title = name.into();
            }
            window.focus(&self.focus, cx);
            cx.notify();
            return;
        }

        let id = self
            .workspace
            .bots
            .iter()
            .map(|bot| bot.id)
            .max()
            .unwrap_or(0)
            + 1;
        let chat_id = self
            .workspace
            .chats
            .iter()
            .map(|chat| chat.id)
            .max()
            .unwrap_or(0)
            + 1;

        self.workspace.bots.push(Bot {
            id,
            name: name.clone().into(),
            role: role.clone().into(),
            color: form.color,
            mascot: form.mascot,
            model: model.into(),
            presence: Presence::Resting,
            activity: "Set itself up".into(),
            computer: form.computer,
            folder: folder.clone().into(),
            instructions: instructions.into(),
            apps: Vec::new(),
            schedule: None,
            tokens: 0,
        });
        self.workspace.chats.push(Chat {
            id: chat_id,
            title: name.clone().into(),
            bot: Some(id),
            members: Vec::new(),
            messages: vec![Message {
                author: model::Author::Bot,
                bot: Some(id),
                parts: vec![Part::Text(format!(
                    "I'm set up.\n\nI answer with {model}, I work in {folder}, and I look after {role}. Tell me the one thing you want off your plate and I'll take it."
                ))],
                time: "now".into(),
                streaming: false,
                stopped: false,
            }],
            preview: "I'm set up. What should I take first?".into(),
            time: "now".into(),
        });
        self.workspace.selected = self.workspace.chats.len() - 1;
        self.draft.clear();
        self.scroll.scroll_to_bottom();
        window.focus(&self.focus, cx);
        cx.notify();
    }

    // ---- rendering -------------------------------------------------------

    pub(crate) fn view_overlay(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let panel = match self.dialog.as_ref()? {
            Dialog::Bot(form) => self.bot_form_dialog(theme, cx, form),
            Dialog::Settings(page) => self.settings_dialog(theme, cx, *page),
        };
        Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.scrim)
                .child(panel)
                .into_any_element(),
        )
    }

    fn dialog_shell(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        width: f32,
        header: AnyElement,
        body: AnyElement,
        footer: AnyElement,
    ) -> AnyElement {
        div()
            .id("dialog")
            .w(px(width))
            .max_h(px(600.0))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(metrics::RADIUS_DIALOG))
            .bg(theme.dialog)
            .border_1()
            .border_color(theme.edge)
            .shadow_xl()
            .track_focus(&self.dialog_focus)
            .on_key_down(cx.listener(Self::on_dialog_key))
            .child(header)
            .child(body)
            .child(footer)
            .into_any_element()
    }

    fn dialog_header(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        title: &'static str,
        subtitle: Option<String>,
    ) -> AnyElement {
        div()
            .flex_shrink_0()
            .px(px(metrics::SPACE_4))
            .pt(px(metrics::SPACE_4))
            .pb(px(metrics::SPACE_3))
            .flex()
            .items_start()
            .gap(px(metrics::SPACE_3))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(typo::SHEET_TITLE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.ink)
                            .child(title),
                    )
                    .when_some(subtitle, |this, subtitle| {
                        this.child(
                            div()
                                .text_size(px(typo::META))
                                .line_height(px(typo::META_LINE))
                                .text_color(theme.ink_tertiary)
                                .child(subtitle),
                        )
                    }),
            )
            .child(widgets::icon_button(
                ("dialog-close", 0),
                "x",
                theme,
                cx.listener(|this, _event, window, cx| this.close_dialog(window, cx)),
            ))
            .into_any_element()
    }

    /// One text field: a plate, the value, and a caret on the active one.
    fn form_field(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        form: &BotForm,
        field: Field,
        mono: bool,
    ) -> AnyElement {
        let active = form.active == field;
        let value = form.text(field).to_string();
        let empty = value.is_empty();
        let placeholder = match field {
            Field::Name => "Ada",
            Field::Role => "Research",
            Field::Folder => "~/code/project",
            Field::Instructions => "How this bot should work",
        };
        let text_color = if empty { theme.ink_tertiary } else { theme.ink };
        let font = if mono {
            theme.font_mono.clone()
        } else {
            theme.font_sans.clone()
        };

        div()
            .id(("field", field.index()))
            .h(px(metrics::CONTROL_LG))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(2.0))
            .overflow_hidden()
            .rounded(px(metrics::RADIUS_CONTROL))
            .bg(theme.inset)
            .border_1()
            .border_color(if active { theme.edge } else { theme.hairline })
            .cursor_text()
            .track_focus(&form.focus[field.index()])
            .on_click(
                cx.listener(move |this, _event, window, cx| this.focus_field(field, window, cx)),
            )
            .child(
                div()
                    .truncate()
                    .font_family(font)
                    .text_size(px(typo::BODY))
                    .text_color(text_color)
                    .child(if empty {
                        placeholder.to_string()
                    } else {
                        value
                    }),
            )
            .when(active, |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .w(px(1.5))
                        .h(px(16.0))
                        .bg(theme.ink_secondary)
                        .into_any_element(),
                )
            })
            .into_any_element()
    }

    fn labelled(
        &self,
        theme: &Theme,
        label: &'static str,
        hint: Option<&'static str>,
        control: AnyElement,
    ) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap(px(metrics::SPACE_1))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(metrics::SPACE_1))
                    .child(
                        div()
                            .text_size(px(typo::META))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.ink_secondary)
                            .child(label),
                    )
                    .when_some(hint, |this, hint| {
                        this.child(
                            div()
                                .text_size(px(typo::TINY))
                                .text_color(theme.ink_tertiary)
                                .child(hint),
                        )
                    }),
            )
            .child(control)
            .into_any_element()
    }

    fn bot_form_dialog(&self, theme: &Theme, cx: &mut Context<Self>, form: &BotForm) -> AnyElement {
        let editing = form.editing();
        let header = self.dialog_header(
            theme,
            cx,
            if editing { "Edit bot" } else { "New bot" },
            Some(if editing {
                "The chat, the sidebar and the panel follow these fields the moment you save."
                    .to_string()
            } else {
                "It answers in this chat. You can change any of this later in its panel."
                    .to_string()
            }),
        );

        let name = self.labelled(
            theme,
            Field::Name.label(),
            None,
            self.form_field(theme, cx, form, Field::Name, false),
        );
        let role = self.labelled(
            theme,
            Field::Role.label(),
            None,
            self.form_field(theme, cx, form, Field::Role, false),
        );

        let folder = self.labelled(
            theme,
            "Working folder",
            None,
            self.form_field(theme, cx, form, Field::Folder, true),
        );
        let instructions = self.labelled(
            theme,
            Field::Instructions.label(),
            Some("Optional"),
            self.form_field(theme, cx, form, Field::Instructions, false),
        );

        let model_name = Shell::MODELS
            .get(form.model)
            .copied()
            .unwrap_or(Shell::MODELS[0]);
        let hover = theme.hover_wash();
        let model_row = div()
            .id("form-model")
            .h(px(metrics::CONTROL_LG))
            .px(px(10.0))
            .flex()
            .items_center()
            .justify_between()
            .rounded(px(metrics::RADIUS_CONTROL))
            .bg(theme.inset)
            .border_1()
            .border_color(theme.hairline)
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_click(cx.listener(|this, _event, _window, cx| {
                if let Some(Dialog::Bot(form)) = this.dialog.as_mut() {
                    form.model = (form.model + 1) % Shell::MODELS.len();
                }
                cx.notify();
            }))
            .child(
                div()
                    .text_size(px(typo::BODY))
                    .text_color(theme.ink)
                    .child(model_name),
            )
            .child(widgets::icon("chevron-down", 12.0, theme.ink_tertiary))
            .into_any_element();

        let computer = div()
            .flex()
            .gap(px(metrics::SPACE_1))
            .child(widgets::segment(
                ("form-machine", 0),
                "This computer",
                form.computer == Computer::ThisComputer,
                theme,
                cx.listener(|this, _event, _window, cx| {
                    if let Some(Dialog::Bot(form)) = this.dialog.as_mut() {
                        form.computer = Computer::ThisComputer;
                    }
                    cx.notify();
                }),
            ))
            .child(widgets::segment(
                ("form-machine", 1),
                "Cloud desktop",
                form.computer == Computer::CloudDesktop,
                theme,
                cx.listener(|this, _event, _window, cx| {
                    if let Some(Dialog::Bot(form)) = this.dialog.as_mut() {
                        form.computer = Computer::CloudDesktop;
                    }
                    cx.notify();
                }),
            ))
            .into_any_element();

        let mut swatches: Vec<AnyElement> = Vec::new();
        for slot in 0..8 {
            let color = theme.identity(slot);
            let selected = form.color == slot;
            swatches.push(
                div()
                    .id(("swatch", slot))
                    .p(px(2.0))
                    .rounded_full()
                    .border_1()
                    .border_color(if selected { theme.ink } else { theme.hairline })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        if let Some(Dialog::Bot(form)) = this.dialog.as_mut() {
                            form.color = slot;
                        }
                        cx.notify();
                    }))
                    .child(widgets::mascot(form.mascot, color, Presence::Resting, 22.0, 100 + slot))
                    .into_any_element(),
            );
        }

        let mut mascot_buttons: Vec<AnyElement> = Vec::new();
        for shape in MascotShape::ALL {
            let selected = form.mascot == shape;
            let shape_index = shape.index();
            mascot_buttons.push(
                div()
                    .id(("mascot-pick", shape_index))
                    .p(px(2.0))
                    .rounded(px(metrics::RADIUS_CONTROL))
                    .border_1()
                    .border_color(if selected { theme.ink } else { theme.hairline })
                    .bg(if selected { theme.hover_wash() } else { gpui::transparent_black() })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        if let Some(Dialog::Bot(form)) = this.dialog.as_mut() {
                            form.mascot = shape;
                        }
                        cx.notify();
                    }))
                    .child(widgets::mascot(
                        shape,
                        theme.identity(form.color),
                        if selected { Presence::Working } else { Presence::Resting },
                        26.0,
                        200 + shape_index,
                    ))
                    .into_any_element(),
            );
        }

        let body = div()
            .id("dialog-body")
            .flex_1()
            .overflow_y_scroll()
            .px(px(metrics::SPACE_4))
            .pb(px(metrics::SPACE_4))
            .flex()
            .flex_col()
            .gap(px(metrics::SPACE_3))
            .child(self.labelled(theme, "Answers with", None, model_row))
            .child(self.labelled(theme, "Works on", None, computer))
            .child(folder)
            .child(instructions)
            .child(self.labelled(
                theme,
                "Mascot Shape",
                Some("Choose from 26 expressive agent silhouettes"),
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(metrics::SPACE_1))
                    .children(mascot_buttons)
                    .into_any_element(),
            ))
            .child(self.labelled(
                theme,
                "Color",
                Some("Its mark, and its name in the transcript"),
                div()
                    .flex()
                    .items_center()
                    .gap(px(metrics::SPACE_2))
                    // Live preview: the pick and the mood read at a glance.
                    .child(widgets::mascot(
                        form.mascot,
                        theme.identity(form.color),
                        Presence::Working,
                        40.0,
                        999,
                    ))
                    .child(
                        div()
                            .flex()
                            .gap(px(metrics::SPACE_1))
                            .children(swatches),
                    )
                    .into_any_element(),
            ))
            // The create flow starts with the model and work context.
            // Keep identity fields available at the end so the first viewport
            // follows the reference while names and roles remain editable.
            .child(
                div()
                    .flex()
                    .gap(px(metrics::SPACE_3))
                    .child(div().flex_1().child(name))
                    .child(div().w(px(150.0)).child(role)),
            )
            .into_any_element();

        let can_submit = form.can_submit();
        let accent = theme.accent;
        let on_accent = theme.on_accent;
        let control = theme.control;
        let footer = div()
            .flex_shrink_0()
            .px(px(metrics::SPACE_4))
            .py(px(metrics::SPACE_3))
            .flex()
            .items_center()
            .gap(px(metrics::SPACE_2))
            .border_t_1()
            .border_color(theme.hairline)
            .child(
                div()
                    .flex_1()
                    .text_size(px(typo::TINY))
                    .text_color(theme.ink_tertiary)
                    .child(if editing {
                        "Enter saves · Tab moves · Esc cancels"
                    } else {
                        "Enter creates · Tab moves · Esc closes"
                    }),
            )
            .child(widgets::segment(
                ("form-cancel", 0),
                "Cancel",
                false,
                theme,
                cx.listener(|this, _event, window, cx| this.close_dialog(window, cx)),
            ))
            .child(
                div()
                    .id("form-create")
                    .h(px(26.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .rounded_full()
                    .bg(if can_submit { accent } else { control })
                    .when(can_submit, |this| this.cursor_pointer())
                    .text_size(px(typo::TINY))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if can_submit {
                        on_accent
                    } else {
                        theme.ink_tertiary
                    })
                    .on_click(
                        cx.listener(|this, _event, window, cx| this.commit_bot_form(window, cx)),
                    )
                    .child(if editing {
                        "Save changes"
                    } else {
                        "Create bot"
                    })
                    .into_any_element(),
            )
            .into_any_element();

        self.dialog_shell(theme, cx, 516.0, header, body, footer)
    }

    fn settings_dialog(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        page: SettingsPage,
    ) -> AnyElement {
        let header = self.dialog_header(
            theme,
            cx,
            "Settings",
            Some("App-wide only. A single bot's settings live in its panel.".into()),
        );

        let mut rail: Vec<AnyElement> = Vec::new();
        for entry in SettingsPage::ALL {
            let selected = entry == page;
            let hover = theme.hover_wash();
            rail.push(
                div()
                    .id(("settings-tab", entry as usize))
                    .h(px(metrics::MENU_ROW))
                    .px(px(metrics::SPACE_2))
                    .mx(px(metrics::SPACE_1))
                    .flex()
                    .items_center()
                    .rounded(px(metrics::RADIUS_MENU_ROW))
                    .when(selected, |this| this.bg(theme.ink.opacity(0.09)))
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .text_size(px(typo::BODY))
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
                    .on_click(cx.listener(move |this, _event, _window, cx| {
                        this.dialog = Some(Dialog::Settings(entry));
                        cx.notify();
                    }))
                    .child(entry.label())
                    .into_any_element(),
            );
        }

        let content = match page {
            SettingsPage::General => self.settings_general(theme, cx),
            SettingsPage::Behavior => self.settings_behavior(theme, cx),
            SettingsPage::Shortcuts => Self::settings_shortcuts(theme),
            SettingsPage::About => Self::settings_about(theme),
        };

        let body = div()
            .flex()
            .flex_1()
            .min_w(px(0.0))
            .overflow_hidden()
            .child(
                div()
                    .w(px(168.0))
                    .h_full()
                    .flex_shrink_0()
                    .pt(px(metrics::SPACE_2))
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .bg(theme.panel)
                    .children(rail),
            )
            .child(widgets::edge(theme.hairline))
            .child(
                div()
                    .id("settings-body")
                    .flex_1()
                    // A flex item in a row keeps `min-width: auto` until it is
                    // allowed to shrink, which is what let long copy escape the
                    // dialog instead of wrapping.
                    .min_w(px(0.0))
                    .h_full()
                    .overflow_y_scroll()
                    .px(px(metrics::SPACE_5))
                    .py(px(metrics::SPACE_4))
                    .flex()
                    .flex_col()
                    .gap(px(metrics::SPACE_5))
                    .child(content),
            )
            .into_any_element();

        let footer = div()
            .flex_shrink_0()
            .px(px(metrics::SPACE_5))
            .py(px(metrics::SPACE_3))
            .flex()
            .items_center()
            .gap(px(metrics::SPACE_2))
            .border_t_1()
            .border_color(theme.hairline)
            .child(
                div()
                    .flex_1()
                    .text_size(px(typo::TINY))
                    .text_color(theme.ink_tertiary)
                    .child("Changes apply immediately · Esc closes"),
            )
            .child(
                div()
                    .id("settings-done")
                    .h(px(26.0))
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .rounded_full()
                    .bg(theme.accent)
                    .cursor_pointer()
                    .text_size(px(typo::TINY))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.on_accent)
                    .on_click(cx.listener(|this, _event, window, cx| this.close_dialog(window, cx)))
                    .child("Done")
                    .into_any_element(),
            )
            .into_any_element();

        self.dialog_shell(theme, cx, 680.0, header, body, footer)
    }

    fn settings_general(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let skin = div()
            .flex()
            .gap(px(metrics::SPACE_1))
            .child(widgets::segment(
                ("settings-skin", 0),
                "Midnight",
                self.appearance == crate::theme::Appearance::Dark,
                theme,
                cx.listener(|this, _event, _window, cx| {
                    if this.appearance != crate::theme::Appearance::Dark {
                        this.toggle_appearance(cx);
                    }
                }),
            ))
            .child(widgets::segment(
                ("settings-skin", 1),
                "Daylight",
                self.appearance == crate::theme::Appearance::Light,
                theme,
                cx.listener(|this, _event, _window, cx| {
                    if this.appearance != crate::theme::Appearance::Light {
                        this.toggle_appearance(cx);
                    }
                }),
            ))
            .into_any_element();

        let mode = div()
            .flex()
            .gap(px(metrics::SPACE_1))
            .child(widgets::segment(
                ("settings-mode", 0),
                Mode::Simple.label(),
                self.settings.mode == Mode::Simple,
                theme,
                cx.listener(|this, _event, _window, cx| this.set_mode(Mode::Simple, cx)),
            ))
            .child(widgets::segment(
                ("settings-mode", 1),
                Mode::Advanced.label(),
                self.settings.mode == Mode::Advanced,
                theme,
                cx.listener(|this, _event, _window, cx| this.set_mode(Mode::Advanced, cx)),
            ))
            .into_any_element();

        div()
            .flex()
            .flex_col()
            .gap(px(metrics::SPACE_5))
            .child(self.labelled(theme, "Skin", None, skin))
            .child(self.labelled(
                theme,
                "Interface",
                Some("Advanced adds detail; it never rearranges the app"),
                mode,
            ))
            .child(
                div()
                    .text_size(px(typo::META))
                    .line_height(px(typo::META_LINE + 2.0))
                    .text_color(theme.ink_tertiary)
                    .child("Simple keeps token counts and usage meters out of the way. Switching never loses anything: the same rows come back in Advanced."),
            )
            .into_any_element()
    }

    fn settings_behavior(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let hover = theme.hover_wash();
        let model = Shell::MODELS
            .get(self.settings.default_model)
            .copied()
            .unwrap_or(Shell::MODELS[0]);
        let model_row = div()
            .id("settings-default-model")
            .h(px(metrics::CONTROL_LG))
            .px(px(10.0))
            .flex()
            .items_center()
            .justify_between()
            .rounded(px(metrics::RADIUS_CONTROL))
            .bg(theme.inset)
            .border_1()
            .border_color(theme.hairline)
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_click(cx.listener(|this, _event, _window, cx| {
                this.settings.default_model =
                    (this.settings.default_model + 1) % Shell::MODELS.len();
                cx.notify();
            }))
            .child(
                div()
                    .text_size(px(typo::BODY))
                    .text_color(theme.ink)
                    .child(model),
            )
            .child(widgets::icon("chevron-down", 12.0, theme.ink_tertiary))
            .into_any_element();

        let send_row = div()
            .flex()
            .items_center()
            .gap(px(metrics::SPACE_3))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(
                        div()
                            .text_size(px(typo::BODY))
                            .text_color(theme.ink)
                            .child("Send on Enter"),
                    )
                    .child(
                        div()
                            .text_size(px(typo::TINY))
                            .text_color(theme.ink_tertiary)
                            .child(if self.settings.send_on_enter {
                                "Off means Enter starts a new line and Ctrl+Enter sends"
                            } else {
                                "Enter starts a new line · Ctrl+Enter sends"
                            }),
                    ),
            )
            .child(widgets::switch(
                ("settings-send", 0),
                self.settings.send_on_enter,
                theme,
                cx.listener(|this, _event, _window, cx| {
                    this.settings.send_on_enter = !this.settings.send_on_enter;
                    cx.notify();
                }),
            ))
            .into_any_element();

        div()
            .flex()
            .flex_col()
            .gap(px(metrics::SPACE_5))
            .child(self.labelled(theme, "Default model for new bots", None, model_row))
            .child(self.labelled(theme, "Composer", None, send_row))
            .into_any_element()
    }

    fn settings_shortcuts(theme: &Theme) -> AnyElement {
        let rows: [(&str, [&str; 2]); 6] = [
            ("Send the message", ["Enter", ""]),
            ("New line", ["Shift", "Enter"]),
            ("New bot", ["Ctrl", "N"]),
            ("Search the sidebar", ["Ctrl", "F"]),
            ("Bot panel", ["Ctrl", "I"]),
            ("Switch skin", ["Ctrl", "J"]),
        ];
        let mut list: Vec<AnyElement> = Vec::new();
        for (action, keys) in rows {
            let mut caps: Vec<AnyElement> = Vec::new();
            for key in keys.iter().filter(|key| !key.is_empty()) {
                caps.push(widgets::keycap(*key, theme));
            }
            list.push(
                div()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(typo::BODY))
                            .text_color(theme.ink_secondary)
                            .child(action),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(metrics::SPACE_1))
                            .children(caps),
                    )
                    .into_any_element(),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(metrics::SPACE_5))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(widgets::section_label("Keyboard", theme))
                    .children(list),
            )
            .child(
                div()
                    .text_size(px(typo::META))
                    .line_height(px(typo::META_LINE + 2.0))
                    .text_color(theme.ink_tertiary)
                    .child("On this computer Ctrl is the secondary modifier; on a Mac it is Cmd."),
            )
            .into_any_element()
    }

    fn settings_about(theme: &Theme) -> AnyElement {
        let line = |label: &'static str, value: String| {
            div()
                .h(px(28.0))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(typo::BODY))
                        .text_color(theme.ink_secondary)
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(typo::META))
                        .text_color(theme.ink)
                        .child(value),
                )
                .into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(metrics::SPACE_5))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(1.0))
                    .child(widgets::section_label("This app", theme))
                    .child(line("Version", env!("CARGO_PKG_VERSION").to_string()))
                    .child(line("Engine", "Simulated (src/engine.rs)".to_string()))
                    .child(line("UI stack", "gpui".to_string())),
            )
            .child(
                div()
                    .text_size(px(typo::META))
                    .line_height(px(typo::META_LINE + 2.0))
                    .text_color(theme.ink_tertiary)
                    .child("Written for this app: no source copied from another bot client, icons drawn here. Preferences and bots reset when the app closes."),
            )
            .into_any_element()
    }

    // ---- settings that do something -------------------------------------

    pub(crate) fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        self.settings.mode = mode;
        cx.notify();
    }
}
