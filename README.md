# Grok Bot

A calm desktop home for a team of AI bots: each one has its own model, its own
working folder, its own computer and its own conversation. You talk to them like
contacts, watch them work, and take back anything you did not want.

The desktop interface is a Rust/GPUI recreation guided by the supplied Grok Bot
screenshots and the extracted `grok` reference tree. Bot mascots use the
original Grok Bot transparent PNG shape masks, tinted with Grok's identity
palette and embedded in the app.

## Run it

```bash
cargo run
```

The first build compiles gpui and its renderer, which takes a while on a cold
`target/`; every run after that is quick.

The window opens with the Launch Bot conversation selected.

| Key | Does |
| --- | --- |
| `Enter` | Send |
| `Shift+Enter` | New line in the composer (kept, not sent) |
| `Ctrl+J` | Switch Midnight / Daylight |
| `Ctrl+I` | Show or hide the bot panel |
| `Ctrl+N` | New bot (opens the form) |
| `Ctrl+F` | Search the sidebar |
| `Esc` | Close a dialog, or clear what you typed |

In the new-bot form: `Tab` moves between fields, `Enter` creates, `Esc` closes.
The sidebar's gear opens Settings.

## Creating a bot

The `+` in the sidebar (or `Ctrl+N`) opens a real form, not a prompt: name, role,
model, computer, working folder, instructions and the colour of its mark. Nothing
is created until Create, and the button stays disabled until the bot has a name.
The bot's first message describes the setup it just received, so the chat starts
with the facts rather than the person's guesses.

## Editing a bot

The pencil in the bot panel's header reopens the same form on that bot, filled in
with what it is now. Saving writes the fields back in place: the sidebar row, the
chat title, the composer's placeholder and the panel all follow the new name, and
the mark takes the new colour. Nothing is appended to the transcript — an edit is
not an event the bot needs to narrate.

## Settings

Settings are app-wide; a single bot's settings live in its own panel. Each page
changes something real, immediately:

- **General** — skin (Midnight / Daylight) and interface mode.
- **Behaviour** — the model a new bot starts on, and whether `Enter` sends or
  starts a new line (`Ctrl+Enter` then sends).
- **Shortcuts** — the keyboard map, as reference.
- **About** — version, the fact that the engine is simulated, and the provenance
  note.

**Simple** (the default) keeps token counts, usage meters and inspector rows out
of the way. **Advanced** adds them back: the team usage meter in the sidebar and
the Usage section in the bot panel. Switching never loses anything — it is the
same rows, shown or hidden.

## What is real, what is simulated

The shell is fully wired: selection, search, composer input, streaming, stop,
undo, copy, the new-bot form, every settings page, the bot panel rows, the usage
meters and both skins all change real state.

The **engine is simulated** (`src/engine.rs`). A prompt is turned into a scripted
run — intents, tool steps, file edits, out-of-chat changes — played back with the
pacing a live model would produce. Nothing leaves the machine and no API key is
needed. Swapping in a real engine means producing the same `Vec<Action>`; no view
code needs to change.

## Layout

```
src/
  main.rs        boot: assets, font choice, skin, window
  theme.rs       tokens (two skins), layout metrics, type scale
  model.rs       bots, chats, messages, seeded team
  engine.rs      the simulated agent: prompts -> action scripts
  assets.rs      embedded icon set (SVG masks)
  ui/
    shell.rs     the three places, run lifecycle, keys
    sidebar.rs   team + rooms
    chat.rs      header, transcript, status strip
    composer.rs  the plate you type into
    bot_panel.rs everything about one bot
    dialog.rs    new-bot form + settings
    widgets.rs   shared components
```

## Design rules the code follows

- **Three ink levels, one accent.** Hierarchy comes from ink and size, never from
  a fourth grey or a decorative color.
- **Bot colors belong to bots.** `identity-*` in the token set is only used for a
  bot's own mark and name.
- **One status mark per row.** No count badges, no "needs you" summaries.
- **Every state is designed.** Empty, working, stopped, undone, no matches,
  narrow window, both skins.
- **Motion explains, it never decorates.** Only the live run animates, and the
  streaming caret is static on purpose.

## Provenance

The interface structure, mascot shape assets and color palette follow the
reference screenshot and extracted Grok Bot source assets. The conversation
engine remains simulated and local, as described above.
