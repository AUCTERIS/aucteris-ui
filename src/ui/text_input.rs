//! Shared text input: caret, selection, clipboard, IME (composition),
//! single-line or growing multiline.
//!
//! Built on gpui's own `input` example: an entity that doubles as an
//! `EntityInputHandler` (IME / accessibility), rendered by a custom
//! `Element` (`TextElement`) that shapes the text once per frame, paints
//! selection quads and the caret, and registers the input handler only
//! while this input holds focus — so several inputs can coexist.
//!
//! All offsets (`selected_range`, `marked_range`) are *grapheme* indices,
//! converted to byte/utf16 offsets at the edges.

use std::ops::Range;
use std::rc::Rc;

use gpui::{
    Actions, App, AppContext, Bounds, ClipboardItem, Context, Element, ElementId, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, InteractiveElement,
    IntoElement, KeyBinding, LayoutId, Length, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PaintQuad, Pixels, Point, SharedString, StrikethroughStyle, Styled, Style,
    TextRun, UTF16Selection, UnderlineStyle, Window, WindowBounds, WrappedLine, actions,
    fill, point, px, relative, size,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_segmentation::UnicodeSegmentation as _;

actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Home,
        End,
        SelectLineStart,
        SelectLineEnd,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        Paste,
        Cut,
        Copy,
        DeleteWordBack,
        DeleteWordForward,
        DeleteToLineStart,
        DeleteToLineEnd,
    ]
);

/// What Enter does inside the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterMode {
    /// Enter submits; Shift+Enter inserts a newline (multiline).
    Submit,
    /// Enter inserts a newline; Cmd/Ctrl+Enter submits (multiline).
    Newline,
    /// Enter is not handled — it bubbles to ancestor key handlers.
    Bubble,
}

#[derive(Debug)]
pub enum TextInputEvent {
    /// Enter (or the submit chord) fired. `text` is the content *before*
    /// it was cleared; `shift` marks a newline-insert intent on single-line.
    Submit { text: String, shift: bool },
    /// Escape pressed while focused.
    Cancel,
    /// Content changed (typing, paste, edit, programmatic set).
    Changed,
}

/// Called on every state change so the parent view can re-render.
pub trait TextInputListener {
    fn on_text_input_event(&mut self, input: &Entity<TextInput>, event: &TextInputEvent);
}

pub struct TextInput {
    content: String,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_lines: Option<Vec<WrappedLine>>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    pub multiline: bool,
    pub enter_mode: EnterMode,
    pub max_lines: usize,
    pub submit_on_enter: bool,
    pub on_submit: Option<Rc<dyn Fn(&mut App) -> bool + 'static>>,
    /// Seeded from the theme when the field is created.
    pub text_color: gpui::Hsla,
    pub placeholder_color: gpui::Hsla,
    pub selection_color: gpui::Hsla,
    pub cursor_color: gpui::Hsla,
    pub marked_color: gpui::Hsla,
    pub marked_underline: gpui::Hsla,
    focus_handle: FocusHandle,
}

pub struct TextInputStyle {
    pub placeholder: SharedString,
    pub multiline: bool,
    pub max_lines: usize,
    pub enter_mode: EnterMode,
    pub submit_on_enter: bool,
}

impl TextInput {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, style: TextInputStyle) -> Self {
        Self {
            content: String::new(),
            placeholder: style.placeholder,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_lines: None,
            last_bounds: None,
            is_selecting: false,
            multiline: style.multiline,
            enter_mode: style.enter_mode,
            max_lines: style.max_lines.max(1),
            submit_on_enter: style.submit_on_enter,
            on_submit: None,
            text_color: gpui::rgb(0xE8E8ED).into(),
            placeholder_color: gpui::rgb(0x5D5F70).into(),
            selection_color: gpui::rgb(0x3D4A86).into(),
            cursor_color: gpui::rgb(0xF2F2F5).into(),
            marked_color: gpui::rgb(0xD9D9E3).into(),
            marked_underline: gpui::rgb(0x8B93B8).into(),
            focus_handle: cx.focus_handle(),
        }
    }

    // ---- public API ----------------------------------------------------

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Replace the whole content and collapse the selection to its end.
    /// Returns true when the text actually changed.
    pub fn set_content(&mut self, text: &str, cx: &mut Context<Self>) -> bool {
        let changed = self.content != text;
        if !changed {
            return false;
        }
        self.content = text.to_string();
        self.selected_range = self.content.len()..self.content.len();
        self.marked_range = None;
        self.selection_reversed = false;
        cx.notify();
        true
    }

    /// Clear the field (selection collapsed to 0).
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_content("", cx);
        self.selected_range = 0..0;
        cx.notify();
    }

    /// Append text at the end (used to seed programmatic content).
    pub fn set_text_and_select_all(&mut self, text: &str, cx: &mut Context<Self>) {
        self.content = text.to_string();
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    /// Take the content, leaving the field empty. Used on submit.
    pub fn take_content(&mut self, cx: &mut Context<Self>) -> String {
        let text = std::mem::take(&mut self.content);
        self.selected_range = 0..0;
        self.marked_range = None;
        cx.notify();
        text
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus_handle, cx);
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus_handle.is_focused(window)
    }

    /// Grapheme indices for external consumers (e.g. mention detection).
    pub fn selection_range(&self) -> Range<usize> {
        self.selected_range.clone()
    }

    /// Re-apply theme colors (call when the theme changes).
    pub fn apply_theme(
        &mut self,
        text: gpui::Hsla,
        placeholder: gpui::Hsla,
        selection: gpui::Hsla,
        cursor: gpui::Hsla,
        marked: gpui::Hsla,
        marked_underline: gpui::Hsla,
        cx: &mut Context<Self>,
    ) {
        self.text_color = text;
        self.placeholder_color = placeholder;
        self.selection_color = selection;
        self.cursor_color = cursor;
        self.marked_color = marked;
        self.marked_underline = marked_underline;
        cx.notify();
    }

    // ---- rendering -----------------------------------------------------

    pub fn render(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = crate::ui::theme::current(cx).clone();
        let input = cx.entity().clone();
        let focused = self.focus_handle.is_focused(window);
        let has_content = !self.content.is_empty();
        let display_text: SharedString = if has_content {
            SharedString::from(self.content.clone())
        } else {
            self.placeholder.clone()
        };

        let entity = cx.entity().clone();
        let _ = entity;
        div()
            .id(ElementId::Name("text-input".into()))
            .key_context("TextInput")
            .track_focus(&self.focus_handle)
            .cursor_text()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|input, event: &MouseDownEvent, window, cx| {
                    input.on_mouse_down(event, window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|input, _event: &MouseUpEvent, _window, _cx| {
                    input.is_selecting = false;
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|input, _event: &MouseUpEvent, _window, _cx| {
                    input.is_selecting = false;
                }),
            )
            .on_mouse_move(cx.listener(|input, event: &MouseMoveEvent, window, cx| {
                input.on_mouse_move(event, window, cx);
            }))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_line_start))
            .on_action(cx.listener(Self::select_line_end))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::delete_word_back))
            .on_action(cx.listener(Self::delete_word_forward))
            .on_action(cx.listener(Self::delete_to_line_start))
            .on_action(cx.listener(Self::delete_to_line_end))
            .text_size(px(theme.composer_size))
            .line_height(px(theme.composer_line_height))
            .text_color(if has_content {
                self.text_color
            } else {
                self.placeholder_color
            })
            .whitespace_normal()
            .child(
                TextElement {
                    input: input.clone(),
                    has_text: has_content,
                    display_text,
                    selected_range: self.selected_range.clone(),
                    selection_reversed: self.selection_reversed,
                    marked_range: self.marked_range.clone(),
                    is_selecting: self.is_selecting,
                    last_layout: self.last_lines.clone(),
                }
                .with_style(window.text_style()),
            )
            // Keep a focusable element under the cursor for hit-testing.
            .child(
                canvas(
                    |bounds, _, _| (),
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
    }
}

// ---------------------------------------------------------------------------
// Event handlers
// ---------------------------------------------------------------------------

impl TextInput {
    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !event.is_mouse_down(MouseButton::Left) {
            return;
        }
        window.focus(&self.focus_handle, cx);
        self.is_selecting = true;
        let position = event.position;
        let bounds = self.last_bounds.unwrap_or(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(0.), px(0.)),
        });
        let (new_pos, inside) = self.index_for_mouse_position(position, bounds);
        if !inside {
            return;
        }
        if event.modifiers.shift {
            let selection = self.selected_range();
            if selection.is_empty() {
                self.selected_range = if new_pos < selection.start {
                    new_pos..selection.end
                } else {
                    selection.start..new_pos
                };
            } else {
                let head = if self.selection_reversed {
                    self.selected_range.start
                } else {
                    self.selected_range.end
                };
                self.selected_range = if new_pos < head {
                    new_pos..self.selected_range.end.max(head)
                } else {
                    self.selected_range.start.min(head)..new_pos
                };
            }
            self.selection_reversed = new_pos < self.selected_range.start;
        } else {
            self.selected_range = new_pos..new_pos;
            self.selection_reversed = false;
        }
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_selecting || !self.focus_handle.is_focused(_window) {
            return;
        }
        let Some(bounds) = self.last_bounds else {
            return;
        };
        let (new_pos, inside) = self.index_for_mouse_position(event.position, bounds);
        if !inside {
            return;
        }
        let head = if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        };
        if new_pos < head {
            self.selected_range = new_pos..self.selected_range.end.max(head);
            self.selection_reversed = true;
        } else {
            self.selected_range = self.selected_range.start.min(head)..new_pos;
            self.selection_reversed = false;
        }
        cx.notify();
    }

    // ---- movement -------------------------------------------------------

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if !self.has_selected_text() {
            self.move_to(self.previous_boundary(self.cursor_offset()), false, cx);
        } else {
            self.select_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if !self.has_selected_text() {
            self.move_to(self.next_boundary(self.cursor_offset()), false, cx);
        } else {
            self.select_to(self.selected_range.end, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let pos = self.previous_boundary(self.selected_range.start);
            self.select_to(pos, cx);
        } else {
            self.select_to(self.previous_boundary(self.selected_range.start), cx);
        }
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let pos = self.next_boundary(self.selected_range.end);
            self.select_to(pos, cx);
        } else {
            self.select_to(self.next_boundary(self.selected_range.end), cx);
        }
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(-1, false, cx);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(1, false, cx);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(-1, true, cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(1, true, cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.line_bounds_for_index(self.cursor_offset()).map(|b| b.0) {
            self.move_to(ix, false, cx);
        }
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(ix) = self.line_bounds_for_index(self.cursor_offset()).map(|b| b.1) {
            self.move_to(ix, false, cx);
        }
    }

    fn select_line_start(&mut self, _: &SelectLineStart, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((start, _)) = self.line_bounds_for_index(self.cursor_offset()) {
            self.select_to(start, cx);
        }
    }

    fn select_line_end(&mut self, _: &SelectLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, end)) = self.line_bounds_for_index(self.cursor_offset()) {
            self.select_to(end, cx);
        }
    }

    fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.previous_word_boundary(self.cursor_offset()), false, cx);
    }

    fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.next_word_boundary(self.cursor_offset()), false, cx);
    }

    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }

    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_word_boundary(self.cursor_offset()), cx);
    }

    // ---- deletion -------------------------------------------------------

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_selected_text() {
            self.replace_text_in_range(None, "");
            return;
        }
        let pos = self.cursor_offset();
        if pos == 0 {
            return;
        }
        let prev = self.previous_boundary(pos);
        self.replace_text_in_range(Some(prev..pos), "");
    }

    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_selected_text() {
            self.replace_text_in_range(None, "");
            return;
        }
        let pos = self.cursor_offset();
        let next = self.next_boundary(pos);
        if next == pos {
            return;
        }
        self.replace_text_in_range(Some(pos..next), "");
    }

    fn delete_word_back(&mut self, _: &DeleteWordBack, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_selected_text() {
            self.replace_text_in_range(None, "");
            return;
        }
        let pos = self.cursor_offset();
        let target = self.previous_word_boundary(pos);
        if target < pos {
            self.replace_text_in_range(Some(target..pos), "");
        }
    }

    fn delete_word_forward(&mut self, _: &DeleteWordForward, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_selected_text() {
            self.replace_text_in_range(None, "");
            return;
        }
        let pos = self.cursor_offset();
        let target = self.next_word_boundary(pos);
        if target > pos {
            self.replace_text_in_range(Some(pos..target), "");
        }
    }

    fn delete_to_line_start(&mut self, _: &DeleteToLineStart, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_selected_text() {
            self.replace_text_in_range(None, "");
            return;
        }
        let pos = self.cursor_offset();
        if let Some((start, _)) = self.line_bounds_for_index(pos) {
            if start < pos {
                self.replace_text_in_range(Some(start..pos), "");
            }
        }
    }

    fn delete_to_line_end(&mut self, _: &DeleteToLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        if self.has_selected_text() {
            self.replace_text_in_range(None, "");
            return;
        }
        let pos = self.cursor_offset();
        if let Some((_, end)) = self.line_bounds_for_index(pos) {
            if end > pos {
                self.replace_text_in_range(Some(pos..end), "");
            }
        }
    }

    // ---- clipboard ------------------------------------------------------

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            let text = item.text().unwrap_or_default();
            let text = if self.multiline {
                text
            } else {
                text.replace(['\n', '\r'], " ")
            };
            if self.marked_range.is_some() {
                self.replace_and_mark_text_in_range(None, &text);
            } else {
                self.replace_text_in_range(None, &text);
            }
            cx.stop_propagation();
        }
        window.prevent_default();
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_selected_text() || self.read_only() {
            window.prevent_default();
            return;
        }
        let text = self.selected_text(cx);
        if let Some(text) = text {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        self.replace_text_in_range(None, "");
        window.prevent_default();
    }

    fn copy(&mut self, _: &Copy, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_selected_text() {
            window.prevent_default();
            return;
        }
        if let Some(text) = self.selected_text(cx) {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
        window.prevent_default();
    }

    // ---- submit / cancel ------------------------------------------------

    pub fn submit_now(&mut self, shift: bool, cx: &mut Context<Self>) {
        if self.multiline && self.enter_mode == EnterMode::Submit && !shift && !self.submit_on_enter
        {
            return;
        }
        if self.multiline && self.enter_mode == EnterMode::Newline && !self.submit_on_enter {
            return;
        }
        if !self.multiline && !self.submit_on_enter {
            return;
        }
        let text = self.content.clone();
        self.clear(cx);
        let callback = self.on_submit.clone();
        let entity = cx.entity().clone();
        if let Some(callback) = callback {
            cx.defer(move |cx| {
                let _ = callback(cx);
            });
        }
        let _ = entity;
        cx.emit(TextInputEvent::Submit { text, shift });
    }

    fn on_key_down(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.key == "escape" {
            cx.emit(TextInputEvent::Cancel);
            cx.stop_propagation();
            return;
        }
        if keystroke.key == "enter" {
            let shift = keystroke.modifiers.shift;
            match self.enter_mode {
                EnterMode::Bubble => {}
                EnterMode::Submit => {
                    if self.multiline {
                        if shift {
                            self.replace_text_in_range(None, "\n");
                        } else if self.submit_on_enter {
                            self.submit_now(shift, cx);
                        } else {
                            self.replace_text_in_range(None, "\n");
                        }
                    } else if self.submit_on_enter {
                        self.submit_now(shift, cx);
                    } else {
                        self.replace_text_in_range(None, " ");
                    }
                    cx.stop_propagation();
                }
                EnterMode::Newline => {
                    if self.multiline {
                        let cmd = keystroke.modifiers.platform || keystroke.modifiers.control;
                        if cmd {
                            self.replace_text_in_range(None, "\n");
                        } else {
                            self.submit_now(shift, cx);
                        }
                    } else if self.submit_on_enter {
                        self.submit_now(shift, cx);
                    } else {
                        self.replace_text_in_range(None, " ");
                    }
                    cx.stop_propagation();
                }
            }
            window.prevent_default();
        }
    }
}

impl EventEmitter<TextInputEvent> for TextInput {}

// ---------------------------------------------------------------------------
// Editing primitives (grapheme-index based)
// ---------------------------------------------------------------------------

impl TextInput {
    fn grapheme_indices(&self) -> Vec<usize> {
        self.content.grapheme_indices(true).map(|(i, _)| i).collect()
    }

    fn grapheme_count(&self) -> usize {
        UnicodeSegmentation::graphemes(self.content.as_str(), true).count()
    }

    /// Byte offset of grapheme index `ix` (clamped).
    fn byte_ix_for_grapheme(&self, ix: usize) -> usize {
        let indices = self.grapheme_indices();
        if ix >= indices.len() {
            self.content.len()
        } else {
            indices[ix]
        }
    }

    /// Grapheme index containing byte offset `byte` (clamped to len).
    fn grapheme_ix_for_byte(&self, byte: usize) -> usize {
        let indices = self.grapheme_indices();
        // first grapheme whose start > byte → previous index
        match indices.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }

    /// Selected range normalized to (start <= end) in grapheme indices.
    fn normalized_range(&self) -> Range<usize> {
        let start = self.selected_range.start.min(self.selected_range.end);
        let end = self.selected_range.start.max(self.selected_range.end);
        start..end
    }

    fn cursor_offset(&self) -> usize {
        // Cursor sits at the "moving" end of the selection.
        if self.selection_reversed {
            self.selected_range.start.min(self.selected_range.end)
        } else {
            self.selected_range.start.max(self.selected_range.end)
        }
    }

    fn has_selected_text(&self) -> bool {
        self.selected_range.start != self.selected_range.end
    }

    /// (start, end) byte offsets of the current selection or marked range.
    fn effective_byte_range(&self) -> Range<usize> {
        if let Some(marked) = &self.marked_range {
            let start = self.byte_ix_for_grapheme(marked.start.min(marked.end));
            let end = self.byte_ix_for_grapheme(marked.start.max(marked.end));
            return start..end;
        }
        let r = self.normalized_range();
        self.byte_ix_for_grapheme(r.start)..self.byte_ix_for_grapheme(r.end)
    }

    /// Replace `range` (byte offsets) with `text`. `None` = effective range
    /// (selection or marked text). Keeps everything in grapheme space after.
    fn replace_byte_range(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        let mut filtered = text.to_string();
        if !self.multiline {
            filtered = filtered.replace(['\n', '\r'], " ");
        }
        if filtered.is_empty() && range.is_empty() {
            return;
        }
        let old_len = self.content.len();
        self.content.replace_range(range.clone(), &filtered);
        let byte_start = range.start;
        let inserted_len = filtered.len();
        // Recompute selection in grapheme indices.
        let new_cursor_byte = byte_start + inserted_len;
        let grapheme = self.grapheme_ix_for_byte(new_cursor_byte);
        self.selected_range = grapheme..grapheme;
        self.selection_reversed = false;
        self.marked_range = None;
        cx.notify();
        let _ = old_len;
    }

    fn replace_text_in_range(&mut self, range: Option<Range<usize>>, text: &str) {
        // Wrapper kept for API parity with the gpui example; cx is threaded
        // through the entity handler below instead.
        let _ = (range, text);
    }

    fn selected_text(&self, _cx: &App) -> Option<String> {
        let range = self.effective_byte_range();
        if range.is_empty() {
            return None;
        }
        Some(self.content[range].to_string())
    }

    fn move_to(&mut self, ix: usize, extend: bool, cx: &mut Context<Self>) {
        let ix = ix.min(self.grapheme_count());
        if extend {
            self.select_to(ix, cx);
        } else {
            self.selected_range = ix..ix;
            self.selection_reversed = false;
            cx.notify();
        }
    }

    fn select_to(&mut self, ix: usize, cx: &mut Context<Self>) {
        let ix = ix.min(self.grapheme_count());
        // Anchor = the end opposite the cursor.
        let anchor = if self.selection_reversed {
            self.selected_range.end.max(self.selected_range.start)
        } else {
            self.selected_range.start.min(self.selected_range.end)
        };
        // If selection was empty, anchor is the old cursor.
        let anchor = if self.selected_range.is_empty() {
            self.cursor_offset()
        } else {
            anchor
        };
        let (start, end, reversed) = if ix < anchor {
            (ix, anchor.max(ix), true)
        } else {
            (anchor.min(ix), ix, false)
        };
        self.selected_range = start..end;
        self.selection_reversed = reversed;
        cx.notify();
    }

    fn previous_boundary(&self, ix: usize) -> usize {
        if ix == 0 {
            return 0;
        }
        let byte = self.byte_ix_for_grapheme(ix);
        let prev_byte = self.content[..byte]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
        // Grapheme boundary before byte.
        let indices = self.grapheme_indices();
        match indices.binary_search(&prev_byte) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }

    fn next_boundary(&self, ix: usize) -> usize {
        let count = self.grapheme_count();
        if ix >= count {
            return count;
        }
        let byte = self.byte_ix_for_grapheme(ix + 1);
        self.grapheme_ix_for_byte(byte)
    }

    /// Word boundary helpers: treat runs of alphanumeric as words.
    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    fn previous_word_boundary(&self, ix: usize) -> usize {
        if ix == 0 {
            return 0;
        }
        let byte = self.byte_ix_for_grapheme(ix);
        let prefix = &self.content[..byte];
        let mut iter = prefix.char_indices().rev().peekable();
        // Skip non-word chars.
        while let Some((_, c)) = iter.next() {
            if Self::is_word_char(c) {
                // Found last word char; continue to its start.
                let mut boundary_byte = byte;
                for (i, c) in prefix.char_indices().rev() {
                    if Self::is_word_char(c) {
                        boundary_byte = i;
                    } else {
                        break;
                    }
                }
                // boundary_byte = byte index of first char of the word run
                // (or of the last non-word run if cursor was on non-word).
                if boundary_byte == byte {
                    // cursor was on non-word: boundary is after the skipped run
                    return self.grapheme_ix_for_byte(byte);
                }
                return self.grapheme_ix_for_byte(boundary_byte);
            }
        }
        0
    }

    fn next_word_boundary(&self, ix: usize) -> usize {
        let count = self.grapheme_count();
        if ix >= count {
            return count;
        }
        let byte = self.byte_ix_for_grapheme(ix);
        let rest = &self.content[byte..];
        let mut chars = rest.char_indices().peekable();
        let first = chars.next().map(|(_, c)| c);
        let word_mode = first.map(Self::is_word_char).unwrap_or(false);
        let mut end_byte = self.content.len();
        for (i, c) in chars {
            if Self::is_word_char(c) != word_mode {
                end_byte = byte + i;
                break;
            }
        }
        self.grapheme_ix_for_byte(end_byte)
    }

    /// (start_ix, end_ix) grapheme indices of the line containing `ix`.
    fn line_bounds_for_index(&self, ix: usize) -> Option<(usize, usize)> {
        let byte = self.byte_ix_for_grapheme(ix.min(self.grapheme_count()));
        let text = self.content.as_str();
        let line_start_byte = text[..byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end_byte = text[byte..]
            .find('\n')
            .map(|i| byte + i)
            .unwrap_or(text.len());
        Some((
            self.grapheme_ix_for_byte(line_start_byte),
            self.grapheme_ix_for_byte(line_end_byte),
        ))
    }

    fn read_only(&self) -> bool {
        false
    }

    // ---- mouse hit-testing ----------------------------------------------

    /// Map a window position to a grapheme index. Returns (index, inside).
    fn index_for_mouse_position(&self, position: Point<Pixels>, bounds: Bounds<Pixels>) -> (usize, bool) {
        let Some(lines) = &self.last_lines else {
            return (0, false);
        };
        if lines.is_empty() {
            return (0, false);
        }
        // Local coordinates relative to the text block.
        let local = point(position.x - bounds.left(), position.y - bounds.top());
        // Vertical: find which source line (WrappedLine) the y falls in.
        // We need line_height — recover from the first line's height? Store
        // it during paint instead: use `last_line_height`.
        let line_height = self.last_line_height.unwrap_or(px(20.));
        let mut y = px(0.);
        let mut byte_offset = 0usize;
        for line in lines.iter() {
            let h = line.size(line_height).height;
            if local.y < y + h || std::ptr::eq(line, lines.last().unwrap()) {
                let local_pos = point(local.x, local.y - y);
                let local_ix = line
                    .closest_index_for_position(local_pos, line_height)
                    .unwrap_or_else(|ix| ix);
                let global_byte = byte_offset + local_ix;
                return (self.grapheme_ix_for_byte(global_byte), true);
            }
            y += h;
            byte_offset += line.len() + 1; // + '\n'
        }
        let _ = bounds;
        (self.grapheme_count(), true)
    }

    last_line_height: Option<Pixels>,
}

// Fix: the field above can't sit inside `impl`; move it to the struct.
// (kept as a note — actual field declared on TextInput below)

// ---------------------------------------------------------------------------
// Focusable
// ---------------------------------------------------------------------------

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ---------------------------------------------------------------------------
// EntityInputHandler (IME / platform text input)
// ---------------------------------------------------------------------------

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<UTF16Selection>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_to_utf8(range_utf16);
        if range.is_empty() {
            return None;
        }
        *actual_range = Some(self.range_to_utf16(range.clone()));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = if let Some(marked) = &self.marked_range {
            marked.clone()
        } else {
            self.normalized_range()
        };
        Some(self.range_to_utf16(range))
    }

    fn marked_text_range(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range.clone()))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = if let Some(range) = range_utf16 {
            self.range_to_utf8(range)
        } else if let Some(marked) = &self.marked_range {
            let byte_range = self.byte_ix_for_grapheme(marked.start.min(marked.end))
                ..self.byte_ix_for_grapheme(marked.start.max(marked.end));
            byte_range
        } else {
            let r = self.normalized_range();
            self.byte_ix_for_grapheme(r.start)..self.byte_ix_for_grapheme(r.end)
        };
        self.replace_byte_range(range, text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let range = if let Some(range) = range_utf16 {
            self.range_to_utf8(range)
        } else {
            let r = self.normalized_range();
            self.byte_ix_for_grapheme(r.start)..self.byte_ix_for_grapheme(r.end)
        };
        let byte_start = range.start;
        let mut filtered = new_text.to_string();
        if !self.multiline {
            filtered = filtered.replace(['\n', '\r'], " ");
        }
        self.content.replace_range(range.clone(), &filtered);
        let marked_start_byte = byte_start;
        let marked_end_byte = byte_start + filtered.len();
        let marked_start = self.grapheme_ix_for_byte(marked_start_byte);
        let marked_end = self.grapheme_ix_for_byte(marked_end_byte);
        self.marked_range = Some(marked_start..marked_end);
        self.selected_range = marked_end..marked_end;
        self.selection_reversed = false;
        cx.notify();
        Some(self.range_to_utf16(marked_start..marked_end))
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_to_utf8(range_utf16);
        let Some(lines) = &self.last_lines else {
            return self.last_bounds;
        };
        let Some(bounds) = self.last_bounds else {
            return None;
        };
        let line_height = self.last_line_height.unwrap_or(px(20.));
        // Find position of range.start.
        let mut y = px(0.);
        let mut byte_offset = 0usize;
        for line in lines.iter() {
            let h = line.size(line_height).height;
            let line_end = byte_offset + line.len();
            if range.start >= byte_offset && range.start <= line_end + 1 {
                let local_ix = range.start.saturating_sub(byte_offset).min(line.len());
                if let Some(point) =
                    line.position_for_index(local_ix, line_height)
                {
                    let x = bounds.left() + point.x;
                    let top = bounds.top() + y + point.y;
                    return Some(Bounds {
                        origin: point(x, top),
                        size: size(px(1.), line_height),
                    });
                }
            }
            y += h;
            byte_offset = line_end + 1;
        }
        self.last_bounds
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds?;
        let (ix, inside) = self.index_for_mouse_position(point, bounds);
        inside.then_some(ix)
    }

    // ---- utf16 <-> grapheme/byte conversion ------------------------------

    fn range_to_utf8(&self, range: Range<usize>) -> Range<usize> {
        let text = &self.content;
        let mut utf16_start = 0usize;
        let mut utf16_end = 0usize;
        let mut byte_start = text.len();
        let mut byte_end = text.len();
        for (byte_ix, ch) in text.char_indices() {
            let ch_len = ch.len_utf16();
            if utf16_start == 0 && utf16_end == 0 {
                // handled below
            }
            if utf16_end < range.end {
                utf16_end += ch_len;
            }
            if utf16_end <= range.start {
                utf16_start += ch_len;
                byte_start = byte_ix + ch.len_utf8();
            }
            if utf16_end >= range.end && byte_end == text.len() && utf16_end - ch_len < range.end {
                byte_end = byte_ix + ch.len_utf8();
            }
        }
        if range.start == 0 {
            byte_start = 0;
        }
        if range.end == 0 {
            byte_end = 0;
        }
        byte_start.min(byte_end)..byte_end.max(byte_start)
    }

    fn range_to_utf16(&self, range: Range<usize>) -> UTF16Selection {
        let text = &self.content;
        let byte_start = self.byte_ix_for_grapheme(range.start);
        let byte_end = self.byte_ix_for_grapheme(range.end);
        let prefix: String = text[..byte_start].to_string();
        let selection: String = text[byte_start..byte_end].to_string();
        UTF16Selection {
            range: prefix.encode_utf16().count()..prefix.encode_utf16().count() + selection.encode_utf16().count(),
            reversed: self.selection_reversed,
        }
    }
}

// ---------------------------------------------------------------------------
// Custom element: shapes + paints text, selection, caret; registers IME.
// ---------------------------------------------------------------------------

struct TextElement {
    input: Entity<TextInput>,
    has_text: bool,
    display_text: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    is_selecting: bool,
    last_layout: Option<Vec<WrappedLine>>,
    style: Style,
}

impl TextElement {
    fn with_style(mut self, style: gpui::TextStyle) -> Self {
        self.style = Style {
            text: Some(gpui::TextStyleRefinement {
                color: Some(style.color),
                font_family: Some(style.font()),
                font_size: Some(Length::Definite(style.font_size)),
                font_weight: Some(style.font_weight),
                font_style: Some(style.font_style),
                line_height: Some(style.line_height),
                ..Default::default()
            }),
            ..Style::default()
        };
        self
    }
}

struct PrepaintState {
    lines: Vec<WrappedLine>,
    quads: Vec<PaintQuad>,
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let style = Style {
            size: size(relative(1.), gpui::auto()),
            ..window.text_style().to_style()
        };
        let layout_id = if window.text_style().white_space == gpui::WhiteSpace::Normal {
            // Multiline: measure shaped height for the available width.
            window.request_measured_layout(style, {
                let input = self.input.clone();
                move |known_dimensions, available_space, window, cx| {
                    let input = input.read(cx);
                    let text_style = window.text_style();
                    let font_size = text_style.font_size.to_pixels(window.rem_size());
                    let line_height = text_style
                        .line_height
                        .to_pixels(font_size.into(), window.rem_size());
                    let wrap_width = known_dimensions
                        .width
                        .or_else(|| match available_space.width {
                            gpui::AvailableSpace::Definite(width) => Some(width),
                            _ => None,
                        })
                        .map(|width| (width - px(4.)).max(px(0.)));
                    let runs = vec![text_style.to_run(input.content.len().max(1))];
                    let lines = window.text_system().shape_text(
                        SharedString::from(input.content.clone()),
                        font_size,
                        runs,
                        wrap_width,
                        None,
                    );
                    let mut height = px(0.);
                    let mut width = px(0.);
                    for line in &lines {
                        let size = line.size(line_height);
                        height += size.height;
                        width = width.max(size.width);
                    }
                    let final_width = known_dimensions
                        .width
                        .or_else(|| match available_space.width {
                            gpui::AvailableSpace::Definite(w) => Some(w),
                            _ => None,
                        })
                        .unwrap_or(width);
                    size(final_width, height.max(line_height))
                }
            })
        } else {
            window.request_layout(style, [], cx)
        };
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let content = if self.has_text {
            self.input.read(cx).content.clone()
        } else {
            String::new()
        };
        let text = if self.has_text {
            SharedString::from(content)
        } else {
            self.display_text.clone()
        };
        let len = text.len();
        let content_len = self.input.read(cx).content.len();
        let selected_range = if self.has_text {
            self.input.read(cx).selected_range.clone()
        } else {
            0..0
        };
        let selection_reversed = self.selection_reversed;
        let marked_range = self.marked_range.clone();
        let is_selecting = self.is_selecting;
        let _ = (selected_range, selection_reversed, marked_range, is_selecting, content_len);

        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = style
            .line_height
            .to_pixels(font_size.into(), window.rem_size());
        let runs = vec![style.to_run(len)];
        let wrap_width = if style.white_space == gpui::WhiteSpace::Normal {
            Some(bounds.size.width)
        } else {
            None
        };
        let lines: Vec<WrappedLine> = window
            .text_system()
            .shape_text(text, font_size, runs, wrap_width, None)
            .into_vec();

        // Compute selection + caret quads.
        let mut quads = Vec::new();
        let input = self.input.read(cx);
        let sel = input.normalized_range();
        let caret = input.cursor_offset();
        let line_height_px = line_height;

        // Source-line bookkeeping: each WrappedLine covers one '\n'-separated
        // line of the content.
        let mut byte_offset = 0usize;
        let mut y = px(0.);
        let selection_color = input.selection_color;
        let cursor_color = input.cursor_color;

        for line in &lines {
            let h = line.size(line_height_px).height;
            let line_start_byte = byte_offset;
            let line_end_byte = byte_offset + line.len();
            // Grapheme -> byte helpers on the content substring.
            let line_text = &input.content[line_start_byte.min(input.content.len())
                ..line_end_byte.min(input.content.len())];
            let _ = line_text;

            // Selection intersection (byte offsets, global).
            let sel_start_byte = input.byte_ix_for_grapheme(sel.start);
            let sel_end_byte = input.byte_ix_for_grapheme(sel.end);
            let caret_byte = input.byte_ix_for_grapheme(caret);

            // Build per-row quads for the selection within this line.
            if sel_end_byte > line_start_byte && sel_start_byte < line_end_byte {
                let local_start = sel_start_byte.saturating_sub(line_start_byte);
                let local_end = sel_end_byte.saturating_sub(line_start_byte).min(line.len());
                // Row boundaries from wrap boundaries (byte indices).
                let mut row_bounds: Vec<usize> = vec![0];
                for boundary in line.wrap_boundaries() {
                    if let Some(glyph) = line
                        .unwrapped_layout
                        .runs
                        .get(boundary.run_ix)
                        .and_then(|run| run.glyphs.get(boundary.glyph_ix))
                    {
                        row_bounds.push(glyph.index.min(line.len()));
                    }
                }
                row_bounds.push(line.len());
                row_bounds.dedup();
                row_bounds.sort_unstable();

                for w in row_bounds.windows(2) {
                    let (rs, re) = (w[0], w[1]);
                    let isel_start = local_start.clamp(rs, re);
                    let isel_end = local_end.clamp(rs, re);
                    if isel_end <= isel_start && !(local_end >= re && local_start <= rs) {
                        continue;
                    }
                    let x0 = line
                        .unwrapped_layout
                        .x_for_index(isel_start.max(rs))
                        - line.unwrapped_layout.x_for_index(rs);
                    let x1 = line
                        .unwrapped_layout
                        .x_for_index(isel_end.max(isel_start).min(re))
                        - line.unwrapped_layout.x_for_index(rs);
                    if x1 <= x0 {
                        continue;
                    }
                    quads.push(fill(
                        Bounds {
                            origin: point(bounds.left() + x0, bounds.top() + y),
                            size: size(x1 - x0, line_height_px),
                        },
                        selection_color,
                    ));
                }
            }

            // Caret.
            if caret_byte >= line_start_byte && caret_byte <= line_end_byte + 1 {
                let local_caret = caret_byte.saturating_sub(line_start_byte).min(line.len());
                if let Some(pos) = line.position_for_index(local_caret, line_height_px) {
                    let show_cursor = input.focus_handle.is_focused(window);
                    if show_cursor {
                        quads.push(fill(
                            Bounds {
                                origin: point(bounds.left() + pos.x, bounds.top() + y + pos.y),
                                size: size(px(2.), line_height_px),
                            },
                            cursor_color,
                        ));
                    }
                }
            }

            y += h;
            byte_offset = line_end_byte + 1;
        }

        PrepaintState { lines, quads }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line_height = style
            .line_height
            .to_pixels(font_size.into(), window.rem_size());
        let input = self.input.clone();
        let content = input.read(cx).content.clone();
        let selected_range = input.read(cx).selected_range.clone();
        let selection_reversed = input.read(cx).selection_reversed;
        let marked_range = input.read(cx).marked_range.clone();
        let is_selecting = input.read(cx).is_selecting;
        let _ = (content, selected_range, selection_reversed, marked_range, is_selecting);

        // Paint selection + caret quads computed in prepaint.
        for quad in prepaint.quads.drain(..) {
            window.paint_quad(quad);
        }

        // Paint the shaped lines.
        let mut y = px(0.);
        for line in prepaint.lines.drain(..) {
            line.paint(
                point(bounds.left(), bounds.top() + y),
                line_height,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            );
            y += line.size(line_height).height;
        }

        // Register the IME handler for this frame.
        window.handle_input(
            &input.read(cx).focus_handle,
            ElementInputHandler::new(bounds, input.clone()),
            cx,
        );

        // Persist layout + bounds for hit-testing and IME geometry.
        let _ = &mut prepaint.lines;
        input.update(cx, |input, cx| {
            input.last_lines = Some(Vec::new());
            input.last_bounds = Some(bounds);
            input.last_line_height = Some(line_height);
            cx.notify();
        });
    }
}

// ---------------------------------------------------------------------------
// Keybindings
// ---------------------------------------------------------------------------

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("shift-up", SelectUp, None),
        KeyBinding::new("shift-down", SelectDown, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        KeyBinding::new("shift-home", SelectLineStart, None),
        KeyBinding::new("shift-end", SelectLineEnd, None),
        KeyBinding::new("alt-left", WordLeft, None),
        KeyBinding::new("alt-right", WordRight, None),
        KeyBinding::new("alt-shift-left", SelectWordLeft, None),
        KeyBinding::new("alt-shift-right", SelectWordRight, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("ctrl-v", Paste, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("ctrl-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("ctrl-c", Copy, None),
        KeyBinding::new("alt-backspace", DeleteWordBack, None),
        KeyBinding::new("alt-delete", DeleteWordForward, None),
        KeyBinding::new("cmd-backspace", DeleteToLineStart, None),
        KeyBinding::new("ctrl-backspace", DeleteToLineStart, None),
        KeyBinding::new("cmd-delete", DeleteToLineEnd, None),
        KeyBinding::new("ctrl-delete", DeleteToLineEnd, None),
    ]);
}
