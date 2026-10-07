//! `bd serve` for server-mode workspaces (bd 1.3+): reads and live updates go
//! over HTTP on loopback instead of a new bd process per call. bd serve
//! refuses embedded workspaces, which keep using `bd events tail`.
//!
//! bd serve records its address nowhere, so the board starts its own on a
//! free port, reads the address it prints, and stops it on exit. HTTP goes
//! through `curl` with an argv vector, like every bd call. Writes stay on the
//! CLI, because writes over HTTP skip the workspace's `.beads/hooks`.

use super::events::truncated_head;
use super::live::{Backoff, Follow, Shared};
use super::types::Bead;
use super::{command, parse_list};
use crate::model::Scope;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::process::ChildStdout;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// How long `bd serve` may take to report its address before the board gives
/// up on it and falls back to the CLI.
const START_TIMEOUT: Duration = Duration::from_secs(20);
/// Reads run on the UI thread, so a stuck server must not freeze the board.
const READ_TIMEOUT: &str = "10";

/// Start this workspace's `bd serve` and follow its events. Returns false if
/// it could not start or stopped, so the caller falls back to the CLI.
pub(super) fn run(sh: &Arc<Shared>) -> bool {
    let Some((base, pid)) = start(sh) else {
        return false;
    };
    sh.set_serve_url(Some(&base));
    let head = match get(&format!("{base}/v0/beads/events?since=999999999&limit=1")) {
        Ok((200, body)) => serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("head")?.as_u64()),
        // 409 events_journal_disabled: reads still use HTTP, nothing to follow.
        _ => None,
    };
    let Some(head) = head else {
        sh.load_blocked();
        return true;
    };
    sh.rebaseline(head);
    sh.watch_manifests();
    let mut backoff = Backoff::new();
    loop {
        let started = Instant::now();
        match watch(sh, &base, sh.last_seq()) {
            Follow::Closed | Follow::Disabled => return true,
            Follow::Truncated(head) => {
                sh.rebaseline(head);
                if !backoff.wait(started.elapsed(), sh) {
                    return true;
                }
            }
            Follow::Ended => {
                if sh.exited(pid) {
                    sh.untrack(pid);
                    sh.set_serve_url(None);
                    return sh.stopped();
                }
                if !backoff.wait(started.elapsed(), sh) {
                    return true;
                }
            }
        }
    }
}

/// Spawn `bd serve` on a free loopback port and wait for its address.
fn start(sh: &Shared) -> Option<(String, u32)> {
    let mut child = command(Scope::Repo)
        .args(["serve", "--addr", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let pid = sh.track(child);
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(|l| l.ok()) {
            if let Some((_, url)) = line.split_once("listening on ") {
                let _ = tx.send(url.trim().to_string());
            }
            // Keep draining its output so a full pipe never stalls it.
        }
    });
    match rx.recv_timeout(START_TIMEOUT) {
        Ok(base) => Some((base, pid)),
        Err(_) => {
            sh.untrack(pid);
            None
        }
    }
}

/// GET a URL, returning the HTTP status and body.
fn get(url: &str) -> Result<(u16, String)> {
    let out = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            READ_TIMEOUT,
            "-w",
            "\n%{http_code}",
            url,
        ])
        .output()
        .context("failed to run curl")?;
    if !out.status.success() {
        bail!(
            "curl {url}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, code) = text.rsplit_once('\n').unwrap_or(("", &text));
    Ok((code.trim().parse().unwrap_or(0), body.to_string()))
}

fn get_ok(url: &str) -> Result<String> {
    match get(url)? {
        (200, body) => Ok(body),
        (code, body) => bail!("bd serve {code}: {}", body.trim()),
    }
}

#[derive(Deserialize)]
struct Page {
    items: Vec<Bead>,
}

/// The board's beads over HTTP, the same shape as `bd list --json`.
pub fn list(base: &str, include_closed: bool) -> Result<Vec<Bead>> {
    let body = get_ok(&format!("{base}/v0/beads/issues?all=true&limit=0"))?;
    let page: Page = serde_json::from_str(&body).context("parsing bd serve issues")?;
    Ok(page
        .items
        .into_iter()
        .filter(|b| include_closed || !b.is_closed())
        .collect())
}

/// One bead with its dependencies expanded, like `bd show --json`.
pub fn show(base: &str, id: &str) -> Result<Option<Bead>> {
    let body = get_ok(&format!("{base}/v0/beads/issues/{id}"))?;
    Ok(parse_list(&format!("[{body}]"))?.into_iter().next())
}

/// Follow `events:watch` (Server-Sent Events) from `since`.
fn watch(sh: &Shared, base: &str, since: u64) -> Follow {
    let mut cmd = Command::new("curl");
    cmd.args([
        "-sS",
        "-N",
        "-H",
        &format!("Last-Event-ID: {since}"),
        &format!("{base}/v0/beads/events:watch?since={since}"),
    ]);
    sh.follow(cmd, read_stream)
}

fn read_stream(sh: &Shared, stdout: ChildStdout) -> Follow {
    let mut event = String::new();
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        if line.is_empty() {
            event.clear();
        } else if let Some(name) = line.strip_prefix("event:") {
            event = name.trim().to_string();
        } else if let Some(data) = line.strip_prefix("data:") {
            if let Some(head) = truncated_head(data.trim()) {
                return Follow::Truncated(head);
            }
            if event.is_empty() && !sh.take(data.trim()) {
                return Follow::Closed;
            }
        } else if line.starts_with('{') {
            // A refusal before the stream starts comes back as plain JSON.
            if let Some(head) = truncated_head(&line) {
                return Follow::Truncated(head);
            }
            if line.contains("events_journal_disabled") {
                return Follow::Disabled;
            }
        }
    }
    Follow::Ended
}
