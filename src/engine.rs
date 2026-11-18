//! The simulated agent engine.
//!
//! [`respond`] turns a prompt into a scripted run: a sequence of [`Action`]s
//! that the shell plays back with delays, exactly where a real engine would
//! stream tokens and tool events. Each bot answers in its own voice, so the demo
//! reads like a team rather than one assistant with five names.
//!
//! Replacing this module with a real engine (ACP, an HTTP API, a local model)
//! only needs to produce the same `Vec<Action>`: nothing in the UI knows where
//! the actions came from.

use crate::model::Bot;

/// One beat of a run.
#[derive(Debug, Clone)]
pub enum Action {
    /// Publish what the bot is doing, before anything is said.
    Intent(&'static str),
    /// A fragment of the answer, appended to the live message.
    Say(String),
    /// Start a step: icon, label, optional detail, and the activity line.
    Tool {
        icon: &'static str,
        label: &'static str,
        detail: Option<&'static str>,
        activity: &'static str,
    },
    /// Close the running step with its result.
    Settle(&'static str),
    /// A file the run touched.
    Files {
        path: &'static str,
        added: u32,
        removed: u32,
    },
    /// A change outside the chat, with an Undo.
    Note(&'static str),
}

/// How long the shell waits before the next action.
pub fn pace(action: &Action) -> std::time::Duration {
    use std::time::Duration;
    match action {
        Action::Intent(_) => Duration::from_millis(620),
        Action::Say(text) => Duration::from_millis(if text.len() > 40 { 42 } else { 26 }),
        Action::Tool { .. } => Duration::from_millis(780),
        Action::Settle(_) => Duration::from_millis(320),
        Action::Files { .. } => Duration::from_millis(220),
        Action::Note(_) => Duration::from_millis(360),
    }
}

/// Rough token count for the usage meter: ~1.3 per word plus the overhead a
/// real turn carries.
pub fn estimate_tokens(text: &str) -> u64 {
    let words = text.split_whitespace().count() as u64;
    640 + words * 2 + text.len() as u64 / 3
}

/// Split a paragraph into streaming fragments of about three words, keeping the
/// spaces so the assembled text is byte-identical to the input.
fn say(text: &'static str) -> Vec<Action> {
    let mut out = Vec::new();
    let mut buffer = String::new();
    let mut words = 0;
    for word in text.split_inclusive(' ') {
        buffer.push_str(word);
        words += 1;
        if words >= 3 {
            out.push(Action::Say(std::mem::take(&mut buffer)));
            words = 0;
        }
    }
    if !buffer.is_empty() {
        out.push(Action::Say(buffer));
    }
    out
}

fn say_all(texts: &[&'static str]) -> Vec<Action> {
    texts.iter().flat_map(|text| say(text)).collect()
}

fn tool(
    icon: &'static str,
    label: &'static str,
    detail: Option<&'static str>,
    activity: &'static str,
) -> Action {
    Action::Tool {
        icon,
        label,
        detail,
        activity,
    }
}

/// Pick a run for the prompt. The bot's role decides the shape of the work; the
/// words decide the subject.
pub fn respond(prompt: &str, bot: &Bot) -> Vec<Action> {
    let lower = prompt.to_lowercase();
    let asks_test = lower.contains("test") || lower.contains("fix") || lower.contains("bug");
    let asks_research = lower.contains("research")
        || lower.contains("price")
        || lower.contains("compare")
        || lower.contains("source")
        || lower.contains("find");
    let asks_write = lower.contains("write")
        || lower.contains("draft")
        || lower.contains("post")
        || lower.contains("email");
    let asks_ops = lower.contains("deploy")
        || lower.contains("key")
        || lower.contains("staging")
        || lower.contains("rotate")
        || lower.contains("incident");
    let asks_design = lower.contains("design")
        || lower.contains("screen")
        || lower.contains("empty state")
        || lower.contains("hero");

    if asks_test || bot.role.as_ref() == "Engineering" && !asks_research && !asks_write {
        engineering_run(prompt)
    } else if asks_ops || bot.role.as_ref() == "Ops" && !asks_research {
        ops_run(prompt)
    } else if asks_research || bot.role.as_ref() == "Research" && !asks_write && !asks_design {
        research_run(prompt)
    } else if asks_write || bot.role.as_ref() == "Writing" {
        writing_run(prompt)
    } else if asks_design || bot.role.as_ref() == "Design" {
        design_run(prompt)
    } else {
        general_run(prompt)
    }
}

fn engineering_run(prompt: &str) -> Vec<Action> {
    let mut run = vec![
        Action::Intent("Reading the failing test"),
        tool(
            "terminal",
            "git log --oneline -20",
            None,
            "Checking recent changes",
        ),
        Action::Settle("20 commits, 3 touch the sync crate"),
        tool(
            "file",
            "Read 4 files",
            Some("crates/relay/src/room.rs, crates/relay/tests/sync_room.rs"),
            "Reading the code around the failure",
        ),
        Action::Settle("412 lines"),
    ];
    run.extend(say_all(&[
        "The failure is timing, not logic. ",
        "`room_id` hashes `Instant::now()` and two devices started in the same millisecond produce the same room.\n\n",
        "I'm making the clock a parameter of the room and seeding it in the test, so the ordering is explicit instead of lucky.",
    ]));
    run.push(tool(
        "edit",
        "Edit crates/relay/src/room.rs",
        Some("+18 −9"),
        "Changing the room clock",
    ));
    run.push(Action::Files {
        path: "crates/relay/src/room.rs",
        added: 18,
        removed: 9,
    });
    run.push(tool(
        "terminal",
        "cargo test -p relay",
        None,
        "Running the suite",
    ));
    run.push(Action::Settle("41 passed, 0 failed"));
    run.push(Action::Files {
        path: "crates/relay/tests/sync_room.rs",
        added: 24,
        removed: 2,
    });
    run.extend(say_all(&[
        "Green on three consecutive runs. ",
        "I kept the retry for real network blips — it just isn't what was hiding this bug anymore.\n\n",
        if prompt.len() > 140 {
            "That was a long brief, so I only did the part that was failing. Tell me which follow-up matters most and I'll take it next."
        } else {
            "Want me to open the pull request, or would you rather read the diff first?"
        },
    ]));
    run
}

fn research_run(_prompt: &str) -> Vec<Action> {
    let mut run = vec![
        Action::Intent("Opening the sources"),
        tool(
            "globe",
            "Opened 4 pages",
            Some("vendor-a, vendor-b, vendor-c, ours"),
            "Reading pricing pages",
        ),
        Action::Settle("3 readable, 1 behind a login"),
        tool(
            "search",
            "Searched 12 documents",
            Some("pricing, seats, annual terms"),
            "Cross-checking the numbers",
        ),
        Action::Settle("5 sources agree, 2 conflict"),
    ];
    run.extend(say_all(&[
        "Three of the four pages agree on the seat price; the fourth bundles support and only matches at five seats.\n\n",
        "The annual terms are where they split: one vendor quotes net-30, the others net-60. ",
        "I put both in the table instead of picking one, so nobody has to trust my guess.",
    ]));
    run.push(tool(
        "file",
        "Wrote one file",
        Some("notes/pricing.md"),
        "Writing the summary",
    ));
    run.push(Action::Files {
        path: "notes/pricing.md",
        added: 62,
        removed: 0,
    });
    run.extend(say_all(&[
        "The table is in your notes folder. ",
        "Two lines are blank because the vendor keeps enterprise pricing behind a login — I'd rather leave a gap than invent a number.",
    ]));
    run
}

fn writing_run(_prompt: &str) -> Vec<Action> {
    let mut run = vec![
        Action::Intent("Reading the changelog"),
        tool(
            "folder",
            "Read the last 12 entries",
            Some("content/changelog/"),
            "Matching the house voice",
        ),
        Action::Settle("12 entries, 640 words"),
    ];
    run.extend(say_all(&[
        "Here's the draft, in the voice the older entries use — short sentences, no exclamation marks.\n\n",
        "\"Bots can change a setting for you now. Ask for a quieter notification sound and it happens in the chat, with an Undo next to it if you'd rather have the old one back.\"\n\n",
        "I kept it to two sentences and left the version number out: the page prints that itself.",
    ]));
    run.push(Action::Files {
        path: "content/changelog/0.4.md",
        added: 34,
        removed: 0,
    });
    run.extend(say_all(&[
        "Saved as a draft, not published. ",
        "Say the word and I'll open the pull request against the site.",
    ]));
    run
}

fn ops_run(_prompt: &str) -> Vec<Action> {
    let mut run = vec![
        Action::Intent("Checking the maintenance window"),
        tool(
            "terminal",
            "infra status --env staging",
            None,
            "Reading the environment",
        ),
        Action::Settle("3 services healthy, window opens 02:00"),
        tool(
            "clock",
            "Read the rotation policy",
            Some("keys/rotation.md"),
            "Checking the rules",
        ),
        Action::Settle("90 days, staging may rotate unannounced"),
    ];
    run.extend(say_all(&[
        "Staging is allowed to rotate without an announcement, so nothing needs to wait for the window.\n\n",
        "I'll rotate the key, keep the old one valid for one hour, and tell you here when the new one is live. ",
        "Production is a separate key and I won't touch it.",
    ]));
    run.push(tool(
        "refresh",
        "Rotated the staging key",
        Some("old key expires in 1h"),
        "Rotating the staging key",
    ));
    run.push(Action::Note("Rotated the staging key (expires in 1h)"));
    run.push(Action::Settle("Staging accepted the new key"));
    run.extend(say_all(&[
        "Done — the staging key is new, and the previous one keeps working until 03:14 so a deploy in flight doesn't break.\n\n",
        "If anything fails in the next hour, Undo puts the old key back.",
    ]));
    run
}

fn design_run(_prompt: &str) -> Vec<Action> {
    let mut run = vec![
        Action::Intent("Looking at the hero"),
        tool(
            "sparkle",
            "Opened 3 screens",
            Some("home, tour, pricing"),
            "Comparing the screens",
        ),
        Action::Settle("3 screens, 6 images"),
    ];
    run.extend(say_all(&[
        "The hero is doing three jobs at once. ",
        "One screenshot, one sentence about what the app is, one button — the tour below can carry the rest.\n\n",
        "That also fixes the narrow window: three images never fit, so they were shrinking to nothing anyway.",
    ]));
    run.push(tool(
        "edit",
        "Edit hero.tsx",
        Some("+12 −31"),
        "Rewriting the hero",
    ));
    run.push(Action::Files {
        path: "src/components/hero.tsx",
        added: 12,
        removed: 31,
    });
    run.extend(say_all(&[
        "The moved images are in the tour section, unchanged. ",
        "Nothing is published — this is a local edit you can look at first.",
    ]));
    run
}

fn general_run(prompt: &str) -> Vec<Action> {
    let subject: String = prompt
        .split_whitespace()
        .take(9)
        .collect::<Vec<_>>()
        .join(" ");
    let subject = if subject.is_empty() {
        "that".to_string()
    } else {
        subject
    };
    let mut run = vec![
        Action::Intent("Reading what you have"),
        tool(
            "search",
            "Searched your workspace",
            None,
            "Looking for related work",
        ),
        Action::Settle("4 matches in 2 folders"),
    ];
    run.extend(say_all(&[
        "I read what's already here before touching anything. ",
        "There are four related notes, and two of them disagree about which folder this belongs in.\n\n",
    ]));
    let opening = format!("\"{}…\" ", subject);
    run.push(Action::Say(opening));
    run.extend(say_all(&[
        "splits into two jobs: the part I can do now, and the part that needs your call.\n\n",
        "I'll take the first part. For the second, I've written down the two options with what each one costs.",
    ]));
    run.push(tool(
        "edit",
        "Wrote one file",
        Some("notes/plan.md"),
        "Writing the plan",
    ));
    run.push(Action::Files {
        path: "notes/plan.md",
        added: 28,
        removed: 0,
    });
    run.extend(say_all(&[
        "The plan is in your notes folder. ",
        "Pick a line and I'll start there.",
    ]));
    run
}
