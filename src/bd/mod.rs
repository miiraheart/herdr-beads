//! The bd bridge: every call shells out to the `bd` CLI with an argv VECTOR
//! (never a shell string) so task titles/notes/reasons can never be injected.
//!
//! herdr launches plugins with a minimal PATH, so `bd` is resolved against the
//! common Homebrew/usr locations before falling back to PATH.

pub mod types;

use crate::model::Scope;
use anyhow::{bail, Context, Result};
use std::path::Path;
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

/// Run a bd subcommand, returning stdout. Errors carry bd's stderr.
pub fn run(scope: Scope, args: &[&str]) -> Result<String> {
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

/// The full board: `bd list`, augmented with closed issues when asked (bd's
/// default list omits some done states depending on config).
pub fn load(scope: Scope, include_closed: bool) -> Result<Vec<Bead>> {
    let mut beads = parse_list(&run(scope, &["list", "--json"])?)?;
    if include_closed {
        // Best-effort: merge in closed issues. Ignore if the flag is rejected.
        if let Ok(s) = run(scope, &["list", "--status", "closed", "--json"]) {
            if let Ok(extra) = parse_list(&s) {
                let have: std::collections::HashSet<_> =
                    beads.iter().map(|b| b.id.clone()).collect();
                beads.extend(extra.into_iter().filter(|b| !have.contains(&b.id)));
            }
        }
    }
    Ok(beads)
}

pub fn show(scope: Scope, id: &str) -> Result<Option<Bead>> {
    let v = parse_list(&run(scope, &["show", id, "--json"])?)?;
    Ok(v.into_iter().next())
}

// ---------------------------------------------------------------- writes

pub fn set_status(scope: Scope, id: &str, status: &str) -> Result<()> {
    run(scope, &["update", id, "-s", status]).map(|_| ())
}

pub fn claim(scope: Scope, id: &str) -> Result<()> {
    run(scope, &["update", id, "--claim"]).map(|_| ())
}

pub fn close(scope: Scope, id: &str, reason: &str) -> Result<()> {
    run(scope, &["close", id, "-r", reason]).map(|_| ())
}

pub fn set_priority(scope: Scope, id: &str, priority: u8) -> Result<()> {
    let p = priority.to_string();
    run(scope, &["priority", id, &p]).map(|_| ())
}

pub fn add_note(scope: Scope, id: &str, note: &str) -> Result<()> {
    run(scope, &["note", id, note]).map(|_| ())
}

pub fn add_comment(scope: Scope, id: &str, text: &str) -> Result<()> {
    run(scope, &["comment", id, text]).map(|_| ())
}

pub struct NewBead<'a> {
    pub title: &'a str,
    pub issue_type: &'a str,
    pub priority: u8,
    pub description: &'a str,
    pub assignee: &'a str,
    pub parent: &'a str,
    pub labels: &'a str,
    pub deferred: bool,
}

/// Create a bead from a fully-specified form; returns the new id.
pub fn create(scope: Scope, nb: &NewBead) -> Result<String> {
    let p = nb.priority.to_string();
    // --description is mandatory by convention; seed from title if blank.
    let desc = if nb.description.is_empty() {
        nb.title
    } else {
        nb.description
    };
    let mut args: Vec<&str> = vec![
        "create",
        nb.title,
        "-t",
        nb.issue_type,
        "-p",
        &p,
        "--description",
        desc,
        "--silent",
    ];
    if !nb.assignee.is_empty() {
        args.push("-a");
        args.push(nb.assignee);
    }
    if !nb.parent.is_empty() {
        args.push("--parent");
        args.push(nb.parent);
    }
    if !nb.labels.is_empty() {
        args.push("-l");
        args.push(nb.labels);
    }
    let id = run(scope, &args)?.trim().to_string();
    if nb.deferred && !id.is_empty() {
        let _ = set_status(scope, &id, "deferred"); // best-effort → backlog
    }
    Ok(id)
}

/// Update an existing bead's core fields from an edited form. Optional fields
/// (description/assignee/parent/labels) are only written when non-empty so an
/// untouched field never wipes existing data. Status is left alone unless the
/// backlog toggle is on.
pub fn update_bead(scope: Scope, id: &str, nb: &NewBead) -> Result<()> {
    let p = nb.priority.to_string();
    let mut args: Vec<&str> = vec![
        "update",
        id,
        "--title",
        nb.title,
        "-t",
        nb.issue_type,
        "-p",
        &p,
    ];
    if !nb.description.is_empty() {
        args.push("--description");
        args.push(nb.description);
    }
    if !nb.assignee.is_empty() {
        args.push("-a");
        args.push(nb.assignee);
    }
    if !nb.parent.is_empty() {
        args.push("--parent");
        args.push(nb.parent);
    }
    if !nb.labels.is_empty() {
        args.push("--set-labels");
        args.push(nb.labels);
    }
    run(scope, &args)?;
    if nb.deferred {
        let _ = set_status(scope, id, "deferred");
    }
    Ok(())
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
    fn tolerates_unknown_fields() {
        let s = r#"[{"id":"x-1","title":"t","status":"open","priority":2,"issue_type":"task","surprise_field":123}]"#;
        let beads = parse_list(s).unwrap();
        assert_eq!(beads[0].id, "x-1");
        assert_eq!(beads[0].assignee(), "-");
    }
}
