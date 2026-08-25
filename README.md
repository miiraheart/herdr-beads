# herdr-beads

[![ci](https://github.com/miiraheart/herdr-beads/actions/workflows/ci.yml/badge.svg)](https://github.com/miiraheart/herdr-beads/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-edition%202021-orange.svg)
![herdr 0.7+](https://img.shields.io/badge/herdr-0.7%2B-8a2be2)
![platforms: linux, macOS](https://img.shields.io/badge/platforms-linux%2C%20macOS-informational)
![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)

<p align="center">
  <a href="#install">install</a> ·
  <a href="#features">features</a> ·
  <a href="#keys">keys</a> ·
  <a href="#keybindings">keybindings</a> ·
  <a href="#how-it-works">how it works</a> ·
  <a href="#configuration">configuration</a>
</p>

A [beads](https://github.com/steveyegge/beads) (`bd`) task board for [herdr](https://herdr.dev). Your `bd` issues as a **List**, a **Table**, or a **Kanban**, docked as a narrow sidebar beside your agents or floating as a full-screen popup. One small Rust/ratatui binary: keyboard-first, mouse-optional, and transparent so your terminal shows through.

<img width="1500" height="914" alt="image" src="https://github.com/user-attachments/assets/e0d48710-3c9d-40c9-892e-58a743caaa98" />

`bd` is where the work lives; herdr-beads is the window onto it. Cards are `bd` issues, columns are `bd` statuses, moving a card runs `bd update --status`, and creating one runs `bd create`. The board never invents its own store.

## Features

- **Three views, one keystroke apart** (`K` or `Tab`), switched in process (instant, no respawn, no lost state):
  - **List**: grouped by status, collapsible, one bead per line, with an inline detail pane.
  - **Table**: flat, aligned columns, the only re-sortable view (`o` cycles status, priority, changed).
  - **Kanban**: one column per status. `v` then arrows retag a card across columns.
- **Two surfaces from the same binary**:
  - **Docked sidebar**: pinned to the left of the tab, narrowed to a sidebar column. One key toggles it open and closed, per tab.
  - **Floating popup**: a wide, full board over your layout. A key opens it; `q` closes it from inside, since a herdr popup is session-modal and no outside key can reach it.
- **Auto-dock** (`A`, off by default): every new tab opens with the dock already in place, docked left and narrowed, without pulling your focus out of the tab you are working in.
- **Real edits, straight to `bd`**: claim, close (with a required reason), set priority, add a note (`bd note`) or a comment (`bd comment`), and a full create form.
- **Rich create form** (`a`): Type, Priority, Title, Description, Assignee, Parent epic, Labels, and a start-in-backlog toggle. Every field `bd create` supports.
- **Types at a glance**: each row carries a type tag (epic, bug `B`, feature `F`, chore `C`, spike `S`, story `Y`, decision `D`, task), colored so epics stand out.
- **Detail everywhere** (`d` or `Enter`): description, dependencies, notes, and counts, enriched by `bd show`. It renders below the list in a narrow sidebar, beside it when wide, or as a modal.
- **Repo-scoped automatically**: the board reads the `bd` database of the herdr window's project. `g` flips to `--global` if you run a shared-server bd database.
- **Transparent**: no solid background. Your terminal theme, image, or blur shows through. Selection and the active tab keep an accent for legibility.
- **Mouse-optional**: click the view tabs, click a card or row, scroll. Everything also has a key.

## Keys

Mouse works everywhere; every action also has a key.

| Key | Action |
| --- | --- |
| `K`, `Tab` | cycle view (List, Table, Kanban) |
| `Shift`+`Tab` | cycle view backwards |
| `j` `k` up/down | move selection |
| `Home`, `G` / `End` | jump to the first or last card |
| `h` `l` left/right | kanban: change column. list: collapse or expand a group |
| `1`..`9` | jump to the Nth status group |
| `[` `]` | jump to the previous or next status group |
| `Enter` | open the detail modal |
| `d` | toggle the detail pane (side when wide, below when narrow) |
| `v` | move mode, then arrows retag the card's status |
| `c` | claim (sets in_progress and assigns you) |
| `x` | close (prompts for a reason) |
| `p` | set priority (0 to 4) |
| `n` | add a note (`bd note`) |
| `m` | add a comment (`bd comment`) |
| `a` | open the new-bead form |
| `e` | edit the selected bead (reopens the form) |
| `s` | set status, then pick 1 to 9 |
| `F` | focus: show only this status group |
| `o` | table: cycle sort (status, priority, changed) |
| `/` | filter. `Esc` clears it |
| `g` | scope: repo or global |
| `C` | show or hide closed |
| `A` | auto-open the dock in new tabs (toggle) |
| `r` | refresh from `bd` |
| `f` | zoom the pane fullscreen (toggle) |
| `?` | key help overlay |
| `q` | quit. `Esc` only backs out a layer, it never quits |

**Create form** (`a`): `Tab` and arrows-up/down move between fields, arrows-left/right cycle Type, Priority, and Parent epic, `Space` toggles start-deferred, `Enter` creates, `Esc` cancels.

## Install

Requires **herdr 0.7.0 or newer** (0.7.5+ is recommended so the plugin loads in every session automatically), a **Rust toolchain** (1.78 or newer, since the committed `Cargo.lock` uses lockfile v4; the plugin builds from source on install), and **`bd`** ([beads](https://github.com/steveyegge/beads)) on your `PATH`.

```bash
herdr plugin install miiraheart/herdr-beads
```

Plugins load at server start, so after installing, restart your herdr server (a `herdr server live-handoff`, then reattach) to activate it. On 0.7.5+ it is then available in every session.

Developing locally? Clone and link it instead:

```bash
git clone https://github.com/miiraheart/herdr-beads
cd herdr-beads
herdr plugin link .
```

## Keybindings

Bind the two surfaces in `~/.config/herdr/config.toml` (the herdr prefix is `ctrl+b`):

```toml
[[keys.command]]
key = "prefix+shift+b"          # toggle the docked beads sidebar
type = "plugin_action"
command = "herdr-beads.open-dock"

[[keys.command]]
key = "prefix+shift+k"          # toggle the floating beads board
type = "plugin_action"
command = "herdr-beads.open-board"
```

The dock key toggles: press to open, press again to close. The floating board is a herdr popup, which is session-modal and grabs all keys while focused. Close it with `q`. `Esc` only backs out a layer, and the toggle key cannot reach it either, because herdr popups have no pane ID and do not appear in the pane API.

The plugin also exposes its actions in herdr's command palette: **Beads: toggle dock**, **Beads: open floating board**, and **Beads: redeploy panes** (closes every beads pane so they respawn on a fresh build).

## How it works

- **bd bridge.** Every read (`bd list`, `bd ready`, `bd show`) and write (`bd update --status`, `--claim`, `bd close -r`, `bd priority`, `bd note`, `bd comment`, `bd create`) is a subprocess call built as an argv vector, never a shell string, so titles, notes, and reasons cannot be shell-injected. Writes are local; syncing (`bd dolt push`) stays your deliberate step.
- **Repo resolution.** herdr runs the pane in the plugin's directory, so the binary asks herdr (`HERDR_WORKSPACE_ID` plus `herdr pane list`) for the focused pane's working directory and runs `bd` there. The board scopes to the project the window is on, with no configuration.
- **Columns are statuses.** The board's groups and columns are `bd`'s statuses (`open`, `in_progress`, `blocked`, `deferred`, `closed`, `pinned`, `hooked`, plus any custom ones). Closed is hidden until you press `C`.
- **In-process views.** List, Table, and Kanban are three renderers over one selection, kept by bead id, so the selection survives view switches and reloads.

### Data model

| Board | bd |
| --- | --- |
| Column or group | issue **status** |
| Card | a `bd` issue (`bd list`, `bd ready`) |
| Move a card | `bd update <id> --status <s>` |
| Claim, close, priority | `bd update --claim`, `bd close -r`, `bd priority` |
| Note, comment | `bd note`, `bd comment` |
| New bead (epic parent, labels, and more) | `bd create -t ... -p ... --parent ... -l ...` |

## Configuration

- **Keybindings** live in `~/.config/herdr/config.toml` (see above).
- **Auto-open the dock in new tabs** (off by default): press `A` in the board. The activity bar shows `+auto` while it is on, and a `tab.created` hook then opens the dock in each new tab, docked left and narrowed, without moving your focus out of the tab you are in. Press `A` again to stop. The setting lives in the plugin config directory, so it survives restarts and applies to the next tab with no reload. Only the dock does this: the floating board is a herdr popup, and a session has just one popup, not one per tab.
- **Scope**: `g` toggles between the window's repo `.beads` and `bd --global`. Global requires a bd shared-server database (`BEADS_DOLT_SHARED_SERVER=1`); without one the board says so and stays on repo scope.
- **Theme**: a fixed Catppuccin-Macchiato accent palette on a transparent background, so it blends with any terminal theme.

## Verify without herdr

The board's TUI needs a real terminal, but the bd bridge is fully testable headless:

```bash
cargo test                                            # parser unit tests
cd /path/to/a/repo/with/beads
/path/to/herdr-beads/target/release/herdr-beads --selftest            # repo scope
/path/to/herdr-beads/target/release/herdr-beads --selftest --global   # global scope
```

`--selftest` prints the grouped board as plain text, the same data the TUI renders.

## Uninstall

```bash
herdr plugin uninstall herdr-beads
```

## License

[MIT](LICENSE).
