# Aucteris UI

**A focused desktop workspace for a team of AI agents.** Give each agent a role,
a model profile and a working context; keep conversations, teams and agent
details together in one quiet, considered interface.

Aucteris UI is a native Rust application built with GPUI. Its visual direction
centres on a compact agent sidebar, a spacious conversation canvas and a
separate rounded details panel. Midnight and Daylight themes share the same
clear hierarchy, restrained surfaces and expressive agent identities.

> **Prototype status:** this build is an interactive UI prototype. The chat
> engine is local and scripted; it does not call an AI provider, run tools or
> send data over the network. Model names and conversation content are sample
> data.

## Run locally

```sh
cargo run
```

The first build compiles GPUI and its renderer. A Rust toolchain with Edition
2024 support and the Linux graphics dependencies required by GPUI are needed.

The app opens the Launch Bot conversation. Try sending a message to see the
scripted response and agent activity play through the interface.

## Interface

- **Agents:** create, edit, search and switch between role-based agents.
- **Rooms:** group agents into teams and open a shared conversation.
- **Conversation:** inspect messages, activity, approvals and the composer in a
  centred reading column.
- **Agent details:** open the right-hand panel for model, work location, folder,
  instructions, connected apps, schedule and appearance.
- **Appearance:** switch between Midnight and Daylight; choose Simple or
  Advanced detail in Settings.

### Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Enter` | Send a message |
| `Shift+Enter` | Add a line in the composer |
| `Ctrl+J` | Switch Midnight / Daylight |
| `Ctrl+I` | Show or hide agent details |
| `Ctrl+N` | Create an agent |
| `Ctrl+F` | Search agents and rooms |
| `Esc` | Close a dialog or clear the search field |

In the create-agent form, `Tab` moves between fields, `Enter` creates the agent
and `Esc` closes the form.

## What works in this prototype

Navigation, search, message composition, scripted streaming, stop and undo,
agent creation and editing, room membership, settings, the details panel, usage
indicators and both themes are wired to application state. The scripted engine
in `src/engine.rs` produces the same `Action` sequence a future live engine can
consume, keeping the interface separate from its eventual provider integration.

## Project structure

```text
src/
  main.rs        application startup and window configuration
  model.rs       agents, rooms, messages and sample workspace
  engine.rs      local scripted conversation engine
  theme.rs       Midnight / Daylight design tokens and layout metrics
  assets.rs      embedded icons and mascot rendering
  ui/
    shell.rs     application layout and interaction lifecycle
    sidebar.rs   agent and room navigation
    chat.rs      conversation, transcript and status
    composer.rs  message input and send controls
    bot_panel.rs agent and room details
    dialog.rs    agent form and settings
    widgets.rs   shared visual components
```

## Design principles

- Put the conversation first; keep navigation and details close at hand.
- Use spacing, type and surface contrast to establish hierarchy.
- Give every agent a distinct, configurable color and mascot silhouette.
- Make controls interactive and keep settings changes immediately visible.
- Use motion to communicate live activity, with restrained ambient animation.

## Asset attribution

The current mascot silhouettes and several connector icons are adapted from
assets extracted from the Grok Bot desktop application for this prototype.
They are not original Aucteris artwork. Review and replace these assets with
properly licensed Aucteris artwork before distributing the application.
