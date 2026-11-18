//! Views. Each module adds methods to [`shell::Shell`] for one region of the
//! window, so state stays in one place and a click never travels through an
//! event bus.

pub mod bot_panel;
pub mod chat;
pub mod composer;
pub mod dialog;
pub mod shell;
pub mod sidebar;
pub mod widgets;
