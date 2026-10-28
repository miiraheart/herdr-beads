//! herdr-beads - a beads (bd) board for herdr: List / Table / Kanban over your
//! bd issues, docked as a side panel or floating as a popup.

mod app;
mod app_live;
mod bd;
mod form;
mod input;
mod keys;
mod model;
mod selftest;
mod ui;
mod views;
mod writer;

use crate::app::App;
use crate::bd::live::{Signal, Watcher};
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
use std::time::Duration;

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
    pick_repo(panes, &me, &ws, |dir| {
        bd::workspace_at(std::path::Path::new(dir)).is_some()
    })
}

/// The pane cwd to scope to, from `herdr pane list`'s panes. Only panes in
/// workspace `ws` are candidates, or the focused pane when `ws` is unknown: a
/// project in another workspace is never the one the user means. Among those,
/// a bd workspace wins, then the focused pane, then the first listed.
/// `has_beads` is asked once per distinct directory.
fn pick_repo(
    panes: &[serde_json::Value],
    me: &str,
    ws: &str,
    has_beads: impl Fn(&str) -> bool,
) -> Option<String> {
    fn field<'a>(p: &'a serde_json::Value, k: &str) -> &'a str {
        p.get(k).and_then(|x| x.as_str()).unwrap_or("")
    }
    let focused = |p: &serde_json::Value| p.get("focused").and_then(|x| x.as_bool()) == Some(true);

    let mut seen: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    let mut best: Option<((bool, bool), &str)> = None;
    for p in panes {
        let cwd = field(p, "cwd");
        let ours = if ws.is_empty() {
            focused(p)
        } else {
            field(p, "workspace_id") == ws
        };
        if cwd.is_empty() || field(p, "pane_id") == me || !ours {
            continue;
        }
        let beads = *seen.entry(cwd).or_insert_with(|| has_beads(cwd));
        let rank = (beads, focused(p));
        if !matches!(best, Some((r, _)) if r >= rank) {
            best = Some((rank, cwd));
        }
    }
    best.map(|(_, cwd)| cwd.to_string())
}

fn apply_working_dir() {
    // 1) the launcher's hint: the focused pane's directory. It is trusted even
    //    when bd finds no workspace there, so the board says so rather than
    //    showing some other project's beads.
    if let Ok(dir) = std::env::var("HERDR_BEADS_CWD") {
        if !dir.is_empty() && std::env::set_current_dir(&dir).is_ok() {
            return;
        }
    }
    // 2) no hint: unless bd finds a workspace here, ask herdr for the
    //    workspace's repo
    let here = std::env::current_dir().ok();
    if here.and_then(|d| bd::workspace_at(&d)).is_none() {
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
    // The journal is per workspace, so live updates follow the repo scope only.
    let watcher = Watcher::start(app.show_closed);
    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;
        if app.should_quit {
            break;
        }
        // Wake up regularly so changes made elsewhere show without a keypress.
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => keys::handle_key(&mut app, k),
                Event::Mouse(m) => keys::handle_mouse(&mut app, m),
                _ => {}
            }
        }
        app.finish_writes();
        watcher.set_show_closed(app.show_closed);
        let repo = app.scope == Scope::Repo;
        for signal in watcher.poll() {
            match signal {
                Signal::Live(on) => app.live = on,
                Signal::Record(rec) if repo => app.apply_record(*rec),
                Signal::Blocked(state) if repo => app.apply_blocked(*state),
                Signal::Snapshot(snap) => app.apply_snapshot(*snap),
                Signal::LoadFailed { scope, error } => app.load_failed(scope, &error),
                Signal::Detail { scope, id, bead } => app.apply_detail(scope, id, bead.map(|b| *b)),
                _ => {}
            }
        }
        if app.take_reload_wanted() {
            watcher.request_load(app.scope);
        }
        if let Some(id) = app.detail_wanted.take() {
            watcher.request_show(app.scope, id);
        }
        if app.should_quit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::pick_repo;
    use serde_json::{json, Value};

    fn pane(id: &str, ws: &str, cwd: &str, focused: bool) -> Value {
        json!({"pane_id": id, "workspace_id": ws, "cwd": cwd, "focused": focused})
    }

    /// Two workspaces: ours (w2) holds a worktree whose `.beads` sits in the
    /// shared git dir, plus a sibling repo; the next one (w3) holds another
    /// project. Only the listed directories count as bd workspaces.
    fn layout() -> Vec<Value> {
        vec![
            pane("w1:p1", "w1", "/code/aaa", false),
            pane("w2:p1", "w2", "/code/sibling", false),
            pane("w2:p2", "w2", "/code/proj/dev", true),
            pane("w3:p1", "w3", "/code/zzz", false),
        ]
    }

    fn beads_in(dirs: &'static [&'static str]) -> impl Fn(&str) -> bool {
        move |d| dirs.contains(&d)
    }

    #[test]
    fn never_picks_another_workspace() {
        let panes = layout();
        let all = beads_in(&["/code/aaa", "/code/sibling", "/code/zzz"]);
        assert_eq!(
            pick_repo(&panes, "w2:dock", "w2", &all).as_deref(),
            Some("/code/sibling")
        );
        let none = beads_in(&["/code/aaa", "/code/zzz"]);
        assert_eq!(
            pick_repo(&panes, "w2:dock", "w2", &none).as_deref(),
            Some("/code/proj/dev")
        );
    }

    #[test]
    fn a_focused_bd_workspace_beats_an_earlier_one() {
        let panes = layout();
        let both = beads_in(&["/code/sibling", "/code/proj/dev"]);
        assert_eq!(
            pick_repo(&panes, "w2:dock", "w2", &both).as_deref(),
            Some("/code/proj/dev")
        );
    }

    #[test]
    fn unknown_workspace_follows_the_focused_pane() {
        let panes = layout();
        let others = beads_in(&["/code/aaa", "/code/sibling", "/code/zzz"]);
        assert_eq!(
            pick_repo(&panes, "", "", &others).as_deref(),
            Some("/code/proj/dev")
        );
    }

    #[test]
    fn skips_itself_and_asks_once_per_directory() {
        let mut panes = layout();
        panes.push(pane("w2:p3", "w2", "/code/sibling", false));
        let asked = std::cell::RefCell::new(Vec::new());
        let pick = pick_repo(&panes, "w2:p2", "w2", |d: &str| {
            asked.borrow_mut().push(d.to_string());
            false
        });
        assert_eq!(pick.as_deref(), Some("/code/sibling"));
        assert_eq!(*asked.borrow(), vec!["/code/sibling".to_string()]);
    }
}
