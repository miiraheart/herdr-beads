//! herdr-beads - a beads (bd) board for herdr: List / Table / Kanban over your
//! bd issues, docked as a side panel or floating as a popup.

mod app;
mod bd;
mod form;
mod input;
mod keys;
mod model;
mod selftest;
mod ui;
mod views;

use crate::app::App;
use crate::model::{Mode, Scope};
use anyhow::Result;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
};
use ratatui::Terminal;
use std::io;

fn parse_args() -> (Mode, Scope, bool) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut mode = Mode::Popup;
    let mut scope = Scope::Repo;
    let mut selftest = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                if let Some(v) = args.get(i + 1) {
                    mode = if v == "dock" { Mode::Dock } else { Mode::Popup };
                    i += 1;
                }
            }
            "--dock" => mode = Mode::Dock,
            "--popup" | "--board" => mode = Mode::Popup,
            "--global" => scope = Scope::Global,
            "--selftest" => selftest = true,
            _ => {}
        }
        i += 1;
    }
    (mode, scope, selftest)
}

/// Resolve the repo the board should scope to. herdr runs the pane in the
/// plugin dir (no `.beads`), but injects HERDR_SOCKET_PATH / HERDR_WORKSPACE_ID
/// / HERDR_PANE_ID / HERDR_BIN_PATH - so we ask `herdr pane list` for the
/// focused pane's cwd in our workspace (same trick herdr-flist uses).
fn resolve_repo_cwd() -> Option<String> {
    let me = std::env::var("HERDR_PANE_ID").unwrap_or_default();
    let ws = std::env::var("HERDR_WORKSPACE_ID").unwrap_or_default();
    let bin = std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
    let out = std::process::Command::new(&bin)
        .arg("pane")
        .arg("list")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let panes = v.get("result")?.get("panes")?.as_array()?;

    let others: Vec<&serde_json::Value> = panes
        .iter()
        .filter(|p| p.get("pane_id").and_then(|x| x.as_str()).unwrap_or("") != me)
        .collect();
    let cwd = |p: &serde_json::Value| p.get("cwd").and_then(|x| x.as_str()).map(String::from);
    let in_ws = |p: &serde_json::Value| {
        !ws.is_empty() && p.get("workspace_id").and_then(|x| x.as_str()) == Some(ws.as_str())
    };
    let focused = |p: &serde_json::Value| p.get("focused").and_then(|x| x.as_bool()) == Some(true);
    let has_beads = |c: &str| std::path::Path::new(c).join(".beads").exists();

    // Preference order: a pane with a real .beads dir wins (focused first, my
    // workspace next), then any focused pane, then anything.
    let mut candidates: Vec<String> = Vec::new();
    for p in &others {
        if let Some(c) = cwd(p) {
            let score = (has_beads(&c) as u8) * 4 + (focused(p) as u8) * 2 + (in_ws(p) as u8);
            candidates.push(format!("{score:02}\u{1}{c}"));
        }
    }
    candidates.sort();
    candidates
        .last()
        .and_then(|s| s.split('\u{1}').nth(1))
        .map(String::from)
}

fn apply_working_dir() {
    // 1) explicit override from the launcher
    if let Ok(dir) = std::env::var("HERDR_BEADS_CWD") {
        if !dir.is_empty() {
            let _ = std::env::set_current_dir(&dir);
        }
    }
    // 2) if still no .beads here, ask herdr for the workspace's repo
    if !std::path::Path::new(".beads").exists() {
        if let Some(dir) = resolve_repo_cwd() {
            let _ = std::env::set_current_dir(&dir);
        }
    }
}

fn main() -> Result<()> {
    let (mode, scope, selftest) = parse_args();

    apply_working_dir();

    if selftest {
        return selftest::run(scope);
    }

    // Tag the pane so the launcher can find (and toggle) it via `herdr pane list`.
    let pane_title = match mode {
        Mode::Dock => "herdr-beads-dock",
        Mode::Popup => "herdr-beads-board",
    };
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(
        out,
        EnterAlternateScreen,
        EnableMouseCapture,
        SetTitle(pane_title)
    )?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;

    let res = run_app(&mut terminal, App::new(mode, scope));

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;
        if app.should_quit {
            break;
        }
        match event::read()? {
            Event::Key(k) if k.kind == KeyEventKind::Press => keys::handle_key(&mut app, k),
            Event::Mouse(m) => keys::handle_mouse(&mut app, m),
            _ => {}
        }
        if app.should_quit {
            break;
        }
    }
    Ok(())
}
