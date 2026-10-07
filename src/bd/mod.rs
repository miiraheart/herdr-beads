//! The bd bridge: every call shells out to the `bd` CLI with an argv VECTOR
//! (never a shell string) so task titles/notes/reasons can never be injected.
//!
//! herdr launches plugins with a minimal PATH, so `bd` is resolved against the
//! common Homebrew/usr locations before falling back to PATH.

pub mod events;
pub mod live;
pub mod serve;
pub mod types;

use crate::model::Scope;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use types::Bead;

fn resolve_bd() -> String {
    for c in [
        "/opt/homebrew/bin/bd",
        "/usr/local/bin/bd",
        "/usr/bin/bd",
        "/home/linuxbrew/.linuxbrew/bin/bd",
    ] {
        if Path::new(c).exists() {
            return c.to_string();
        }
    }
    "bd".to_string()
}

/// A `bd` command for `scope`, with the launch environment fixed up.
pub(crate) fn command(scope: Scope) -> Command {
    let mut cmd = Command::new(resolve_bd());
    // Ensure Homebrew is on PATH even under herdr's minimal launch environment.
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:{path}"));
    }
    if scope == Scope::Global {
        cmd.arg("--global");
        // bd's --global needs shared-server mode; opt in (harmless if unavailable).
        cmd.env("BEADS_DOLT_SHARED_SERVER", "1");
    }
    cmd
}

/// Run a bd subcommand, returning stdout. Errors carry bd's stderr.
pub fn run(scope: Scope, args: &[&str]) -> Result<String> {
    let mut cmd = command(scope);
    cmd.args(args);
    let out = cmd
        .output()
        .with_context(|| format!("failed to spawn bd {}", args.join(" ")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("bd {}: {}", args.join(" "), err.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn parse_list(s: &str) -> Result<Vec<Bead>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(s).context("parsing bd --json output")
}

// ---------------------------------------------------------------- reads

/// The full board: `bd list`, with closed issues when asked. `--all` gets
/// both in one call, so a failure is reported instead of half a board.
pub fn load(scope: Scope, include_closed: bool) -> Result<Vec<Bead>> {
    let args: &[&str] = if include_closed {
        &["list", "--json", "--all"]
    } else {
        &["list", "--json"]
    };
    parse_list(&run(scope, args)?)
}

pub fn show(scope: Scope, id: &str) -> Result<Option<Bead>> {
    let v = parse_list(&run(scope, &["show", id, "--json"])?)?;
    Ok(v.into_iter().next())
}

/// Blocked beads and what blocks them, from `bd blocked`. `bd list` does not
/// carry the blocked flag, and it cannot be worked out from the edges alone
/// (it also flows down to children of a blocked parent).
pub fn blocked(scope: Scope) -> Result<HashMap<String, Vec<String>>> {
    let beads = parse_list(&run(scope, &["blocked", "--json"])?)?;
    Ok(beads.into_iter().map(|b| (b.id, b.blocked_by)).collect())
}

pub fn mark_blocked(beads: &mut [Bead], blocked: &HashMap<String, Vec<String>>) {
    for b in beads {
        match blocked.get(&b.id) {
            Some(by) => {
                b.is_blocked = true;
                b.blocked_by = by.clone();
            }
            None => {
                b.is_blocked = false;
                b.blocked_by.clear();
            }
        }
    }
}

// ---------------------------------------------------------------- workspace

/// The workspace's `.beads` directory, found like bd finds it: `BEADS_DIR`,
/// else the nearest `.beads` from the working directory up.
fn beads_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("BEADS_DIR") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    let cwd = std::env::current_dir().ok()?;
    cwd.ancestors()
        .map(|d| d.join(".beads"))
        .find(|d| d.is_dir())
}

/// The workspace's Dolt mode (`embedded`, `server` or `proxied-server`), read
/// from `.beads/metadata.json` so no slow bd call is needed. Mirrors bd's own
/// precedence: the server-mode env vars win over the file.
pub fn dolt_mode() -> String {
    let forced = ["BEADS_DOLT_SERVER_MODE", "BEADS_DOLT_SHARED_SERVER"]
        .iter()
        .any(|k| std::env::var(k).is_ok_and(|v| v == "1"));
    if forced {
        return "server".into();
    }
    beads_dir()
        .and_then(|d| std::fs::read_to_string(d.join("metadata.json")).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("dolt_mode")?.as_str().map(String::from))
        .unwrap_or_else(|| "embedded".into())
}

/// The Dolt `manifest` files of the workspace databases. They change on every
/// write, including a `bd dolt pull`, which the events journal does not record.
pub fn manifests() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Some(beads) = beads_dir() else {
        return out;
    };
    for root in ["embeddeddolt", "dolt"] {
        let Ok(dbs) = std::fs::read_dir(beads.join(root)) else {
            continue;
        };
        for db in dbs.flatten() {
            let m = db.path().join(".dolt/noms/manifest");
            if m.exists() {
                out.push(m);
            }
        }
    }
    out
}

// ---------------------------------------------------------------- writes
//
// Free text (titles, notes, reasons, labels...) can start with `-`, which bd
// would read as a flag. Flags are passed as `--name=value`, and note/comment
// text after `--`, so any text reaches bd as data.

fn flag(name: &str, value: &str) -> String {
    format!("--{name}={value}")
}

fn run_args(scope: Scope, args: &[String]) -> Result<String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    run(scope, &args)
}

pub fn set_status(scope: Scope, id: &str, status: &str) -> Result<()> {
    run(scope, &["update", id, &flag("status", status)]).map(|_| ())
}

pub fn claim(scope: Scope, id: &str) -> Result<()> {
    run(scope, &["update", id, "--claim"]).map(|_| ())
}

pub fn close(scope: Scope, id: &str, reason: &str) -> Result<()> {
    run(scope, &["close", id, &flag("reason", reason)]).map(|_| ())
}

pub fn set_priority(scope: Scope, id: &str, priority: u8) -> Result<()> {
    let p = priority.to_string();
    run(scope, &["priority", id, &p]).map(|_| ())
}

pub fn add_note(scope: Scope, id: &str, note: &str) -> Result<()> {
    run(scope, &["note", id, "--", note]).map(|_| ())
}

pub fn add_comment(scope: Scope, id: &str, text: &str) -> Result<()> {
    run(scope, &["comment", id, "--", text]).map(|_| ())
}

pub struct NewBead {
    pub title: String,
    pub issue_type: String,
    pub priority: u8,
    pub description: String,
    pub assignee: String,
    pub parent: String,
    pub labels: String,
    pub deferred: bool,
}

fn create_args(nb: &NewBead) -> Vec<String> {
    // --description is mandatory by convention; seed from title if blank.
    let desc = if nb.description.is_empty() {
        &nb.title
    } else {
        &nb.description
    };
    let mut args = vec![
        "create".to_string(),
        flag("title", &nb.title),
        flag("type", &nb.issue_type),
        flag("priority", &nb.priority.to_string()),
        flag("description", desc),
        "--silent".to_string(),
    ];
    if nb.deferred {
        args.push(flag("status", "deferred"));
    }
    if !nb.assignee.is_empty() {
        args.push(flag("assignee", &nb.assignee));
    }
    if !nb.parent.is_empty() {
        args.push(flag("parent", &nb.parent));
    }
    if !nb.labels.is_empty() {
        args.push(flag("labels", &nb.labels));
    }
    args
}

/// Create a bead from a fully-specified form; returns the new id. The
/// backlog toggle sets its status in the same call.
pub fn create(scope: Scope, nb: &NewBead) -> Result<String> {
    Ok(run_args(scope, &create_args(nb))?.trim().to_string())
}

fn update_args(id: &str, nb: &NewBead) -> Vec<String> {
    let mut args = vec![
        "update".to_string(),
        id.to_string(),
        flag("title", &nb.title),
        flag("type", &nb.issue_type),
        flag("priority", &nb.priority.to_string()),
    ];
    if nb.deferred {
        args.push(flag("status", "deferred"));
    }
    if !nb.description.is_empty() {
        args.push(flag("description", &nb.description));
    }
    if !nb.assignee.is_empty() {
        args.push(flag("assignee", &nb.assignee));
    }
    if !nb.parent.is_empty() {
        args.push(flag("parent", &nb.parent));
    }
    if !nb.labels.is_empty() {
        args.push(flag("set-labels", &nb.labels));
    }
    args
}

/// Update an existing bead's core fields from an edited form. Optional fields
/// (description/assignee/parent/labels) are only written when non-empty so an
/// untouched field never wipes existing data. Status is left alone unless the
/// backlog toggle is on.
pub fn update_bead(scope: Scope, id: &str, nb: &NewBead) -> Result<()> {
    run_args(scope, &update_args(id, nb)).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list_fixture() {
        let s = include_str!("../../tests/fixtures/list.json");
        let beads = parse_list(s).expect("list.json parses");
        assert!(!beads.is_empty(), "fixture should have beads");
        let b = &beads[0];
        assert!(!b.id.is_empty());
        assert!(!b.status.is_empty());
        assert!(b.priority <= 4);
    }

    #[test]
    fn parses_show_fixture_with_dependencies() {
        let s = include_str!("../../tests/fixtures/show.json");
        let beads = parse_list(s).expect("show.json parses");
        assert_eq!(beads.len().min(1), 1);
        for d in &beads[0].dependencies {
            // In `show`, deps are expanded issues (id/title); either identifies them.
            assert!(d.other_id().is_some() || d.title.is_some());
        }
    }

    #[test]
    fn empty_and_whitespace_parse_to_empty() {
        assert!(parse_list("").unwrap().is_empty());
        assert!(parse_list("   \n  ").unwrap().is_empty());
    }

    #[test]
    fn marks_blocked_beads_and_clears_the_rest() {
        let mut beads = parse_list(include_str!("../../tests/fixtures/list.json")).unwrap();
        beads[1].is_blocked = true;
        let blocked = HashMap::from([("demo-1".to_string(), vec!["demo-2".to_string()])]);
        mark_blocked(&mut beads, &blocked);
        assert!(beads[0].is_blocked);
        assert_eq!(beads[0].blocked_by, vec!["demo-2".to_string()]);
        assert!(!beads[1].is_blocked);
    }

    #[test]
    fn parses_bd_serve_issue_with_expanded_deps() {
        let s = r#"{"id":"sv-918","title":"alpha","status":"open","priority":1,"issue_type":"task","dependencies":[{"id":"sv-6sc","title":"beta","status":"open","dependency_type":"blocks"}],"dependency_count":1}"#;
        let b = parse_list(&format!("[{s}]")).unwrap().remove(0);
        assert_eq!(b.dependencies[0].other_id(), Some("sv-6sc"));
        assert_eq!(b.dependencies[0].dep_type.as_deref(), Some("blocks"));
    }

    fn dashed() -> NewBead {
        NewBead {
            title: "-starts with dash".into(),
            issue_type: "task".into(),
            priority: 2,
            description: "-desc".into(),
            assignee: "-bob".into(),
            parent: String::new(),
            labels: "-lab".into(),
            deferred: false,
        }
    }

    #[test]
    fn text_starting_with_a_dash_is_passed_as_a_value() {
        let create = create_args(&dashed());
        assert!(create.contains(&"--title=-starts with dash".to_string()));
        assert!(create.contains(&"--description=-desc".to_string()));
        assert!(create.contains(&"--assignee=-bob".to_string()));
        assert!(create.contains(&"--labels=-lab".to_string()));
        // Every argument after the subcommand is a `--name=value` flag.
        assert!(create[1..].iter().all(|a| a.starts_with("--")));
        assert!(!create.contains(&"--status=deferred".to_string()));
        let update = update_args("x-1", &dashed());
        assert!(update.contains(&"--title=-starts with dash".to_string()));
        assert!(update.contains(&"--set-labels=-lab".to_string()));
    }

    #[test]
    fn backlog_is_set_in_the_same_call() {
        let nb = NewBead {
            deferred: true,
            ..dashed()
        };
        assert!(create_args(&nb).contains(&"--status=deferred".to_string()));
        assert!(update_args("x-1", &nb).contains(&"--status=deferred".to_string()));
    }

    #[test]
    fn tolerates_unknown_fields() {
        let s = r#"[{"id":"x-1","title":"t","status":"open","priority":2,"issue_type":"task","surprise_field":123}]"#;
        let beads = parse_list(s).unwrap();
        assert_eq!(beads[0].id, "x-1");
        assert_eq!(beads[0].assignee(), "-");
    }
}
