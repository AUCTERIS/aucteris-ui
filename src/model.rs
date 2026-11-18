//! The workspace: bots, chats and everything the bot panel shows.
//!
//! This is deliberately plain data — no entities, no async. The shell owns one
//! [`Workspace`] and the mock agent in [`crate::engine`] mutates it. Swapping
//! the mock for a real engine means replacing that one module.

use gpui::SharedString;

/// The apps a bot can be connected to. One list, so the panel and the shell
/// cannot disagree about which apps exist.
pub const KNOWN_APPS: [&str; 6] = ["GitHub", "Linear", "Gmail", "Notion", "Slack", "Figma"];

/// Where a bot is right now. Three states, one mark on the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Resting,
    Working,
    Waiting,
}

impl Presence {
    pub fn label(self) -> &'static str {
        match self {
            Self::Resting => "Resting",
            Self::Working => "Working",
            Self::Waiting => "Waiting on you",
        }
    }

    pub fn is_working(self) -> bool {
        matches!(self, Self::Working)
    }
}

/// Which machine a bot works on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Computer {
    ThisComputer,
    CloudDesktop,
}

impl Computer {
    pub fn label(self) -> &'static str {
        match self {
            Self::ThisComputer => "This computer",
            Self::CloudDesktop => "Cloud desktop",
        }
    }
}

/// The 26 expressive mascot silhouettes available to agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MascotShape {
    Arch,
    Bean,
    Blob,
    Capsule,
    Cloud,
    Clover,
    Crystal,
    Cylinder,
    Dome,
    Egg,
    Flower,
    Gem,
    Heart,
    Hex,
    House,
    Leaf,
    Pebble,
    Pumpkin,
    Shield,
    #[default]
    Sparkle,
    Square,
    Squircle,
    Star,
    Tablet,
    Teardrop,
    Wedge,
}

impl MascotShape {
    pub const ALL: [MascotShape; 26] = [
        MascotShape::Arch,
        MascotShape::Bean,
        MascotShape::Blob,
        MascotShape::Capsule,
        MascotShape::Cloud,
        MascotShape::Clover,
        MascotShape::Crystal,
        MascotShape::Cylinder,
        MascotShape::Dome,
        MascotShape::Egg,
        MascotShape::Flower,
        MascotShape::Gem,
        MascotShape::Heart,
        MascotShape::Hex,
        MascotShape::House,
        MascotShape::Leaf,
        MascotShape::Pebble,
        MascotShape::Pumpkin,
        MascotShape::Shield,
        MascotShape::Sparkle,
        MascotShape::Square,
        MascotShape::Squircle,
        MascotShape::Star,
        MascotShape::Tablet,
        MascotShape::Teardrop,
        MascotShape::Wedge,
    ];

    pub fn id_str(self) -> &'static str {
        match self {
            Self::Arch => "arch",
            Self::Bean => "bean",
            Self::Blob => "blob",
            Self::Capsule => "capsule",
            Self::Cloud => "cloud",
            Self::Clover => "clover",
            Self::Crystal => "crystal",
            Self::Cylinder => "cylinder",
            Self::Dome => "dome",
            Self::Egg => "egg",
            Self::Flower => "flower",
            Self::Gem => "gem",
            Self::Heart => "heart",
            Self::Hex => "hex",
            Self::House => "house",
            Self::Leaf => "leaf",
            Self::Pebble => "pebble",
            Self::Pumpkin => "pumpkin",
            Self::Shield => "shield",
            Self::Sparkle => "sparkle",
            Self::Square => "square",
            Self::Squircle => "squircle",
            Self::Star => "star",
            Self::Tablet => "tablet",
            Self::Teardrop => "teardrop",
            Self::Wedge => "wedge",
        }
    }

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Self::Arch => "Arch",
            Self::Bean => "Bean",
            Self::Blob => "Blob",
            Self::Capsule => "Capsule",
            Self::Cloud => "Cloud",
            Self::Clover => "Clover",
            Self::Crystal => "Crystal",
            Self::Cylinder => "Cylinder",
            Self::Dome => "Dome",
            Self::Egg => "Egg",
            Self::Flower => "Flower",
            Self::Gem => "Gem",
            Self::Heart => "Heart",
            Self::Hex => "Hexagon",
            Self::House => "House",
            Self::Leaf => "Leaf",
            Self::Pebble => "Pebble",
            Self::Pumpkin => "Pumpkin",
            Self::Shield => "Shield",
            Self::Sparkle => "Sparkle",
            Self::Square => "Square",
            Self::Squircle => "Squircle",
            Self::Star => "Star",
            Self::Tablet => "Tablet",
            Self::Teardrop => "Teardrop",
            Self::Wedge => "Wedge",
        }
    }

    #[allow(dead_code)]
    pub fn from_index(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub struct Bot {
    pub id: u32,
    pub name: SharedString,
    /// What this bot is for, in one word: Research, Engineering, Writing…
    pub role: SharedString,
    /// Slot in the identity palette.
    pub color: usize,
    /// The bot's expressive mascot silhouette.
    pub mascot: MascotShape,
    pub model: SharedString,
    pub presence: Presence,
    /// One line under the name in the sidebar: what it is doing or did.
    pub activity: SharedString,
    pub computer: Computer,
    pub folder: SharedString,
    pub instructions: SharedString,
    pub apps: Vec<SharedString>,
    pub schedule: Option<SharedString>,
    pub tokens: u64,
}

/// One piece of a message. A reply is a sequence of these, so tool work reads
/// inline with the words that explain it.
#[derive(Debug, Clone)]
pub enum Part {
    Text(String),
    /// Collapsible reasoning / thought process block.
    Thought {
        text: String,
        collapsed: bool,
    },
    /// A single step a bot took: a command, a search, a page it opened.
    Tool {
        icon: &'static str,
        label: String,
        detail: Option<String>,
        /// `false` while the step is still running.
        done: bool,
        /// The result line once the step settled.
        result: Option<String>,
    },
    /// A file the bot touched, with its diff size.
    Files {
        path: String,
        added: u32,
        removed: u32,
        undone: bool,
    },
    /// Something the bot changed outside the chat, with a way back.
    Note {
        text: String,
        undone: bool,
    },
    /// A local approval card that can be resolved from the conversation.
    Approval {
        kicker: String,
        title: String,
        detail: String,
        resolved: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Author {
    You,
    Bot,
    /// A room event: someone joined, a bot handed work on.
    System,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub author: Author,
    /// Which bot wrote it, for [`Author::Bot`] inside rooms.
    pub bot: Option<u32>,
    pub parts: Vec<Part>,
    pub time: SharedString,
    pub streaming: bool,
    /// Set when the person stopped the run mid-answer.
    pub stopped: bool,
}

impl Message {
    pub fn text(&self) -> String {
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::Thought { text, .. } => {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(text);
                }
                Part::Tool { label, .. } => {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(label);
                }
                Part::Files { path, .. } => {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(path);
                }
                Part::Note { text, .. } => {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(text);
                }
                Part::Approval { title, detail, resolved, .. } => {
                    if !out.is_empty() { out.push(' '); }
                    out.push_str(title);
                    out.push(' ');
                    out.push_str(detail);
                    if let Some(resolved) = resolved { out.push(' '); out.push_str(resolved); }
                }
            }
        }
        out
    }

    pub fn push_text(&mut self, chunk: &str) {
        if let Some(Part::Text(text)) = self.parts.last_mut() {
            text.push_str(chunk);
        } else {
            self.parts.push(Part::Text(chunk.to_string()));
        }
    }

    pub fn push_part(&mut self, part: Part) {
        self.parts.push(part);
    }

    /// Close the running tool step, if there is one.
    pub fn finish_last_tool(&mut self, result: Option<String>) {
        let running = self
            .parts
            .iter_mut()
            .rev()
            .find(|part| matches!(part, Part::Tool { done: false, .. }));
        if let Some(Part::Tool {
            done, result: slot, ..
        }) = running
        {
            *done = true;
            *slot = result;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chat {
    pub id: u32,
    pub title: SharedString,
    /// `Some` for a direct chat, `None` for a room.
    pub bot: Option<u32>,
    /// Everyone in the room. Empty for a direct chat.
    pub members: Vec<u32>,
    pub messages: Vec<Message>,
    /// The one line the sidebar shows under the title.
    pub preview: SharedString,
    pub time: SharedString,
}

impl Chat {
    pub fn is_room(&self) -> bool {
        self.bot.is_none()
    }
}

/// Everything the app knows. One of these lives in the shell.
pub struct Workspace {
    pub bots: Vec<Bot>,
    pub chats: Vec<Chat>,
    pub selected: usize,
    pub tokens_used: u64,
    pub tokens_cap: u64,
}

impl Workspace {
    pub fn bot(&self, id: u32) -> Option<&Bot> {
        self.bots.iter().find(|bot| bot.id == id)
    }

    pub fn bot_mut(&mut self, id: u32) -> Option<&mut Bot> {
        self.bots.iter_mut().find(|bot| bot.id == id)
    }

    pub fn chat(&self) -> &Chat {
        &self.chats[self.selected]
    }

    pub fn chat_mut(&mut self) -> &mut Chat {
        &mut self.chats[self.selected]
    }

    /// The bot a direct chat belongs to.
    pub fn selected_bot(&self) -> Option<u32> {
        self.chat().bot
    }

    pub fn select(&mut self, index: usize) {
        if index < self.chats.len() {
            self.selected = index;
        }
    }

    /// Bots the composer talks to: one for a chat, everyone for a room.
    pub fn audience(&self) -> Vec<u32> {
        let chat = self.chat();
        match chat.bot {
            Some(bot) => vec![bot],
            None => chat.members.clone(),
        }
    }

    /// The bot that should answer next: for a room, whoever is free, else the
    /// first member.
    pub fn responder(&self) -> Option<u32> {
        let audience = self.audience();
        audience
            .iter()
            .copied()
            .find(|id| {
                self.bot(*id)
                    .map(|bot| bot.presence != Presence::Working)
                    .unwrap_or(false)
            })
            .or_else(|| audience.first().copied())
    }

    pub fn total_working(&self) -> usize {
        self.bots
            .iter()
            .filter(|bot| bot.presence.is_working())
            .count()
    }

    pub fn replace_last_preview(&mut self) {
        if let Some(message) = self.chat().messages.last() {
            let preview = message.text();
            let preview = preview.replace('\n', " ");
            let short: String = preview.chars().take(72).collect();
            let chat = self.chat_mut();
            chat.preview = SharedString::from(short);
            chat.time = SharedString::from("now");
        }
    }
}

fn text(value: &str) -> Part {
    Part::Text(value.to_string())
}

/// The seeded team the app opens on. A real run would hydrate this from the
/// engine; the shape is what matters.
pub fn seed() -> Workspace {
    let bots = vec![
        Bot {
            id: 1,
            name: "Launch Bot".into(),
            role: "Product".into(),
            color: 4,
            mascot: MascotShape::Blob,
            model: "Claude Sonnet 4.6".into(),
            presence: Presence::Resting,
            activity: "Launch updates are ready".into(),
            computer: Computer::CloudDesktop,
            folder: "~/code/relay".into(),
            instructions: "Keep the public API stable. Small commits, tests first.".into(),
            apps: vec!["GitHub".into(), "Linear".into(), "Sentry".into()],
            schedule: Some("Nightly: triage failing tests at 02:00".into()),
            tokens: 412_880,
        },
        Bot {
            id: 2,
            name: "Inbox Manager".into(),
            role: "Inbox".into(),
            color: 6,
            mascot: MascotShape::Bean,
            model: "GPT-5.2".into(),
            presence: Presence::Working,
            activity: "Reading 4 sources on pricing".into(),
            computer: Computer::ThisComputer,
            folder: "~/notes/market".into(),
            instructions: "Cite sources. Prefer primary documents over summaries.".into(),
            apps: vec!["Notion".into(), "Gmail".into()],
            schedule: None,
            tokens: 88_240,
        },
        Bot {
            id: 3,
            name: "Mktg Data".into(),
            role: "Marketing".into(),
            color: 6,
            mascot: MascotShape::Capsule,
            model: "Qwen3.6 32B (local)".into(),
            presence: Presence::Resting,
            activity: "Drafted the launch post".into(),
            computer: Computer::ThisComputer,
            folder: "~/code/site/content".into(),
            instructions: "Plain words, no exclamation marks, sentence case.".into(),
            apps: vec!["GitHub".into()],
            schedule: Some("Weekdays: draft the changelog at 17:00".into()),
            tokens: 51_020,
        },
        Bot {
            id: 4,
            name: "New Hires".into(),
            role: "People".into(),
            color: 5,
            mascot: MascotShape::Blob,
            model: "Claude Opus 4.6".into(),
            presence: Presence::Waiting,
            activity: "3 intros drafted in your voice, ready to review".into(),
            computer: Computer::CloudDesktop,
            folder: "~/code/infra".into(),
            instructions: "Never touch production without a plan in the chat first.".into(),
            apps: vec!["Slack".into(), "PagerDuty".into(), "AWS".into()],
            schedule: Some("Every 15 minutes: check the error budget".into()),
            tokens: 204_400,
        },
        Bot {
            id: 5,
            name: "Expense Manager".into(),
            role: "Finance".into(),
            color: 1,
            mascot: MascotShape::Blob,
            model: "Qwen3.6 32B (local)".into(),
            presence: Presence::Resting,
            activity: "Report filed. 9 receipts, nothing missing".into(),
            computer: Computer::ThisComputer,
            folder: "~/code/design".into(),
            instructions: "One primary action per screen. Calm by default.".into(),
            apps: vec!["Figma".into()],
            schedule: None,
            tokens: 12_640,
        },
        Bot {
            id: 6,
            name: "Chief of Staff".into(),
            role: "Operations".into(),
            color: 3,
            mascot: MascotShape::Blob,
            model: "Claude Sonnet 4.6".into(),
            presence: Presence::Resting,
            activity: "Rescheduled your flight to Friday".into(),
            computer: Computer::ThisComputer,
            folder: "~/work/operations".into(),
            instructions: "Keep plans current and ask before sending messages.".into(),
            apps: vec!["Calendar".into(), "Gmail".into()],
            schedule: None,
            tokens: 8_420,
        },
    ];

    let chats = vec![
        Chat {
            id: 1,
            title: "Launch Bot".into(),
            bot: Some(1),
            members: Vec::new(),
            preview: "Retail cards · updated to 40 hours".into(),
            time: "7:48 AM".into(),
            messages: vec![
                Message {
                    author: Author::You,
                    bot: None,
                    parts: vec![text("Morning! Heads up on the Pendel Arc headphones launch. Battery life moved to 40 hours this week, but the product page, retail cards, and press kit still say 32.")],
                    time: "7:48 AM".into(),
                    streaming: false,
                    stopped: false,
                },
                Message {
                    author: Author::You,
                    bot: None,
                    parts: vec![text("oh no, good catch. did engineering confirm 40?")],
                    time: "7:49 AM".into(),
                    streaming: false,
                    stopped: false,
                },
                Message {
                    author: Author::Bot,
                    bot: Some(1),
                    parts: vec![text("Yes, engineering confirmed it's 40 with noise canceling on. I just made the changes and pinged @Benji and @Lena with the updates.")],
                    time: "7:49 AM".into(),
                    streaming: false,
                    stopped: false,
                },
                Message {
                    author: Author::Bot,
                    bot: Some(1),
                    parts: vec![text("✓ Product page → updated to 40 hours\n✓ Press kit → refreshed with the new spec")],
                    time: "7:49 AM".into(),
                    streaming: false,
                    stopped: false,
                },
                Message {
                    author: Author::Bot,
                    bot: Some(1),
                    parts: vec![
                        text("The retail cards were already at the printer, so I asked the vendor to hold the run."),
                        Part::Approval {
                            kicker: "Needs your OK".into(),
                            title: "Retail shelf cards · v3".into(),
                            detail: "Battery life updated to 40 hours · Print vendor".into(),
                            resolved: None,
                        },
                    ],
                    time: "7:50 AM".into(),
                    streaming: false,
                    stopped: false,
                },
            ],
        },
        Chat {
            id: 2,
            title: "Inbox Manager".into(),
            bot: Some(2),
            members: Vec::new(),
            preview: "Two of the four sources disagree on the floor price…".into(),
            time: "3m".into(),
            messages: vec![
                Message {
                    author: Author::You,
                    bot: None,
                    parts: vec![text(
                        "What did the pricing pages actually say? I only need the numbers that a customer could check themselves.",
                    )],
                    time: "09:31".into(),
                    streaming: false,
                    stopped: false,
                },
                Message {
                    author: Author::Bot,
                    bot: Some(2),
                    parts: vec![
                        Part::Tool {
                            icon: "globe",
                            label: "Opened 4 pricing pages".into(),
                            detail: Some("vendor-a, vendor-b, vendor-c, ours".into()),
                            done: true,
                            result: Some("3 reachable, 1 behind a login".into()),
                        },
                        text(
                            "Two of the four disagree on the floor price, and the third prices seats separately, so the totals only line up at five seats.\n\nI'm still reading the enterprise sheet — I'll put the table in the room when it's whole.",
                        ),
                    ],
                    time: "09:33".into(),
                    streaming: true,
                    stopped: false,
                },
            ],
        },
        Chat {
            id: 3,
            title: "Launch room".into(),
            bot: None,
            members: vec![1, 2, 5],
            preview: "Pim: the hero needs one screenshot, not three.".into(),
            time: "40m".into(),
            messages: vec![
                Message {
                    author: Author::You,
                    bot: None,
                    parts: vec![text(
                        "Landing page ships Friday. Ada, is the API frozen? Pim, the hero still has three screenshots.",
                    )],
                    time: "08:51".into(),
                    streaming: false,
                    stopped: false,
                },
                Message {
                    author: Author::System,
                    bot: None,
                    parts: vec![text("Pim picked up the hero.")],
                    time: "08:54".into(),
                    streaming: false,
                    stopped: false,
                },
                Message {
                    author: Author::Bot,
                    bot: Some(5),
                    parts: vec![text(
                        "The hero needs one screenshot, not three. I'll keep the wide one and move the others into the tour below it.",
                    )],
                    time: "08:56".into(),
                    streaming: false,
                    stopped: false,
                },
                Message {
                    author: Author::Bot,
                    bot: Some(1),
                    parts: vec![text(
                        "The API is frozen except for the webhooks field, which needs one more day. Nothing the page shows depends on it.",
                    )],
                    time: "09:02".into(),
                    streaming: false,
                    stopped: false,
                },
            ],
        },
        Chat {
            id: 4,
            title: "Weekly recap".into(),
            bot: None,
            members: vec![3, 4],
            preview: "Theo: draft is in the folder, 640 words.".into(),
            time: "2h".into(),
            messages: vec![Message {
                author: Author::Bot,
                bot: Some(3),
                parts: vec![text(
                    "Draft is in the folder, 640 words. Nadia, I left the incident numbers blank — I didn't want to guess them.",
                )],
                time: "07:40".into(),
                streaming: false,
                stopped: false,
            }],
        },
        Chat {
            id: 5,
            title: "Mktg Data".into(),
            bot: Some(3),
            members: Vec::new(),
            preview: "Two versions of the opening line, pick one.".into(),
            time: "1h".into(),
            messages: vec![Message {
                author: Author::Bot,
                bot: Some(3),
                parts: vec![text(
                    "Two versions of the opening line, pick one and I'll carry it through the post.",
                )],
                time: "08:20".into(),
                streaming: false,
                stopped: false,
            }],
        },
        Chat {
            id: 6,
            title: "New Hires".into(),
            bot: Some(4),
            members: Vec::new(),
            preview: "The staging key is 90 days old. Want me to rotate it?".into(),
            time: "3h".into(),
            messages: vec![Message {
                author: Author::Bot,
                bot: Some(4),
                parts: vec![text(
                    "The staging key is 90 days old. I can rotate it during the maintenance window — it needs your nod first.",
                )],
                time: "06:15".into(),
                streaming: false,
                stopped: false,
            }],
        },
        Chat {
            id: 7,
            title: "Expense Manager".into(),
            bot: Some(5),
            members: Vec::new(),
            preview: "Redrew the empty states with one action each.".into(),
            time: "1d".into(),
            messages: vec![Message {
                author: Author::Bot,
                bot: Some(5),
                parts: vec![
                    text("Redrew the empty states with one action each. "),
                    Part::Tool {
                        icon: "sparkle",
                        label: "Rendered 6 empty states".into(),
                        detail: None,
                        done: true,
                        result: Some("6 images".into()),
                    },
                ],
                time: "17:02".into(),
                streaming: false,
                stopped: false,
            }],
        },
        Chat {
            id: 8,
            title: "Chief of Staff".into(),
            bot: Some(6),
            members: Vec::new(),
            preview: "Rescheduled your flight to Friday".into(),
            time: "2h".into(),
            messages: vec![Message {
                author: Author::Bot,
                bot: Some(6),
                parts: vec![text("Your flight is now Friday. I moved the calendar hold and let the team know.")],
                time: "05:40".into(),
                streaming: false,
                stopped: false,
            }],
        },
    ];

    Workspace {
        bots,
        chats,
        selected: 0,
        tokens_used: 769_180,
        tokens_cap: 2_000_000,
    }
}

/// How much of the app is on show.
///
/// Simple is the default for a new install: tokens, usage meters and the
/// inspector rows belong in Advanced. Advanced adds to the screen, it never
/// rearranges it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Simple,
    Advanced,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Simple => "Simple",
            Self::Advanced => "Advanced",
        }
    }

    pub fn is_simple(self) -> bool {
        matches!(self, Self::Simple)
    }
}

/// App-wide preferences.
///
/// A single bot's settings live in its own panel; these are the few things that
/// change the whole app, and every one of them does something the moment it
/// changes.
#[derive(Debug, Clone)]
pub struct Settings {
    pub mode: Mode,
    /// Index into [`crate::ui::shell::Shell::MODELS`], the model a new bot starts on.
    pub default_model: usize,
    /// Enter sends. Off means Enter is a line break and Ctrl+Enter sends.
    pub send_on_enter: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Simple,
            default_model: 0,
            send_on_enter: true,
        }
    }
}
