//! Keeps the board current in the background, so the UI thread never waits on
//! bd for it.
//!
//! - Server-mode workspaces get their own `bd serve`; reads and live updates
//!   then go over HTTP (see `serve`). Embedded workspaces, where bd serve is
//!   not available, follow `bd events tail --follow`.
//! - The journal head is read before the full reload and the follower starts
//!   from it, so nothing committed in between is missed (bd's documented
//!   cold-start order).
//! - A follower that stops is restarted from the last seq it delivered. When
//!   bd reports that checkpoint as truncated, or the journal was reset, the
//!   board reloads in full and follows from the new head.
//! - Changes the journal does not record (a `bd dolt pull`, a merge) show up
//!   as a Dolt manifest change with no record; that triggers a full reload.
//! - Blocked state comes from `bd blocked`, then from the records.
//!
//! Full reloads run here and reach the board as a `Snapshot`. Records applied
//! while one was loading are applied again on top (see `App::apply_snapshot`).
//! The board's own reloads (`r`, scope, closed, startup) and its detail pane
//! (`bd show`) are loaded here too, so the UI thread never waits on bd.

use super::events::{last_seq, seq_of, truncated_head, Record};
use super::types::Bead;
use super::{blocked, command, dolt_mode, load, manifests, mark_blocked, run, serve, show};
use crate::model::Scope;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const RESTART_DELAY: Duration = Duration::from_secs(2);
const RESTART_DELAY_MAX: Duration = Duration::from_secs(60);
/// How long a manifest change may go without a record before it counts as an
/// unjournaled change. bd's follower delivers a record within about a second.
const SYNC_QUIET: Duration = Duration::from_secs(4);

pub enum Signal {
    /// Following the journal (true) or not (false).
    Live(bool),
    Record(Box<Record>),
    Snapshot(Box<Snapshot>),
    Blocked(Box<BlockedState>),
    /// A load failed; the board keeps what it has and shows the error.
    LoadFailed {
        scope: Scope,
        error: String,
    },
    /// A bead for the detail pane (`bd show`).
    Detail {
        scope: Scope,
        id: String,
        bead: Option<Box<Bead>>,
    },
}

/// A full reload. Records after `after_seq` may already be on the board and
/// are applied again on top.
pub struct Snapshot {
    pub scope: Scope,
    pub beads: Vec<Bead>,
    pub after_seq: u64,
    /// Whether closed beads were loaded; a snapshot that no longer matches
    /// the board's setting is dropped (the toggle already reloaded).
    pub show_closed: bool,
    /// The journal was reset, so recent records belong to the old journal
    /// and must not be applied again.
    pub reset: bool,
}

pub struct BlockedState {
    pub blocked: HashMap<String, Vec<String>>,
    pub after_seq: u64,
}

pub(super) struct Shared {
    tx: Sender<Signal>,
    show_closed: AtomicBool,
    /// Highest seq delivered to the board.
    last_seq: AtomicU64,
    last_record: Mutex<Instant>,
    serve_url: Mutex<Option<String>>,
    children: Mutex<Vec<Child>>,
    watching_manifests: AtomicBool,
    stop: AtomicBool,
}

impl Shared {
    pub(super) fn send(&self, s: Signal) -> bool {
        !self.stopped() && self.tx.send(s).is_ok()
    }

    pub(super) fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub(super) fn last_seq(&self) -> u64 {
        self.last_seq.load(Ordering::Relaxed)
    }

    pub(super) fn set_head(&self, seq: u64) {
        self.last_seq.store(seq, Ordering::Relaxed);
    }

    /// Hand a record to the board. Returns false once the board is gone.
    pub(super) fn deliver(&self, rec: Record) -> bool {
        if rec.seq <= self.last_seq() {
            return true;
        }
        self.last_seq.store(rec.seq, Ordering::Relaxed);
        if let Ok(mut t) = self.last_record.lock() {
            *t = Instant::now();
        }
        self.send(Signal::Record(Box::new(rec)))
    }

    /// Take one document from a follower: a record is delivered; one that
    /// has a seq but does not parse is reloaded in full instead, so its change
    /// is not lost. Returns false once the board is gone.
    pub(super) fn take(&self, doc: &str) -> bool {
        if let Ok(rec) = serde_json::from_str::<Record>(doc) {
            return self.deliver(rec);
        }
        if let Some(seq) = seq_of(doc) {
            if seq > self.last_seq() {
                self.set_head(seq);
                self.snapshot();
            }
        }
        !self.stopped()
    }

    /// Keep a child so it is stopped with the board. Children share the
    /// board's process group, so closing the pane hangs them up too.
    pub(super) fn track(&self, child: Child) -> u32 {
        let pid = child.id();
        if self.stopped() {
            stop_child(child);
            return pid;
        }
        if let Ok(mut c) = self.children.lock() {
            c.push(child);
        }
        pid
    }

    /// Whether a tracked child has exited.
    pub(super) fn exited(&self, pid: u32) -> bool {
        self.children
            .lock()
            .ok()
            .and_then(|mut c| {
                let child = c.iter_mut().find(|x| x.id() == pid)?;
                Some(!matches!(child.try_wait(), Ok(None)))
            })
            .unwrap_or(true)
    }

    pub(super) fn untrack(&self, pid: u32) {
        let child = self.children.lock().ok().and_then(|mut c| {
            let i = c.iter().position(|x| x.id() == pid)?;
            Some(c.remove(i))
        });
        if let Some(c) = child {
            stop_child(c);
        }
    }

    /// Reload in full and hand the result to the board.
    pub(super) fn snapshot(&self) {
        self.load_snapshot(false);
    }

    /// Follow from `head` after a truncation or a journal reset, reloading in
    /// full first.
    pub(super) fn rebaseline(&self, head: u64) {
        let reset = head < self.last_seq();
        self.set_head(head);
        self.load_snapshot(reset);
    }

    fn load_snapshot(&self, reset: bool) {
        let after_seq = self.last_seq();
        let show_closed = self.show_closed.load(Ordering::Relaxed);
        let url = self.serve_url.lock().ok().and_then(|u| u.clone());
        let beads = match &url {
            Some(base) => serve::list(base, show_closed),
            None => load(Scope::Repo, show_closed),
        };
        let mut beads = match beads {
            Ok(b) => b,
            Err(e) => {
                self.send(Signal::LoadFailed {
                    scope: Scope::Repo,
                    error: e.to_string(),
                });
                return;
            }
        };
        if let Ok(b) = blocked(Scope::Repo) {
            mark_blocked(&mut beads, &b);
        }
        self.send(Signal::Snapshot(Box::new(Snapshot {
            scope: Scope::Repo,
            beads,
            after_seq,
            show_closed,
            reset,
        })));
    }

    pub(super) fn load_blocked(&self) {
        let after_seq = self.last_seq();
        if let Ok(blocked) = blocked(Scope::Repo) {
            self.send(Signal::Blocked(Box::new(BlockedState {
                blocked,
                after_seq,
            })));
        }
    }

    /// Run a follower: start `cmd`, show the board as live while `read`
    /// consumes its output, then stop it.
    pub(super) fn follow(
        &self,
        mut cmd: Command,
        read: impl FnOnce(&Self, ChildStdout) -> Follow,
    ) -> Follow {
        let spawned = cmd.stdout(Stdio::piped()).stderr(Stdio::null()).spawn();
        let Ok(mut child) = spawned else {
            return Follow::Ended;
        };
        let Some(stdout) = child.stdout.take() else {
            return Follow::Ended;
        };
        let pid = self.track(child);
        let result = if self.send(Signal::Live(true)) {
            read(self, stdout)
        } else {
            Follow::Closed
        };
        self.untrack(pid);
        if self.stopped() || !self.send(Signal::Live(false)) {
            return Follow::Closed;
        }
        result
    }

    pub(super) fn set_serve_url(&self, base: Option<&str>) {
        if let Ok(mut u) = self.serve_url.lock() {
            *u = base.map(String::from);
        }
    }

    /// Start watching for unjournaled changes, once.
    pub(super) fn watch_manifests(self: &Arc<Self>) {
        if !self.watching_manifests.swap(true, Ordering::Relaxed) {
            let sh = Arc::clone(self);
            thread::spawn(move || watch_manifests(&sh));
        }
    }
}

/// Wait before restarting a follower: short after a long run, doubling while
/// it keeps failing right away.
pub(super) struct Backoff {
    delay: Duration,
}

impl Backoff {
    pub(super) fn new() -> Self {
        Backoff {
            delay: RESTART_DELAY,
        }
    }

    /// Sleep, then report whether to go on (false once the board is gone).
    pub(super) fn wait(&mut self, ran_for: Duration, sh: &Shared) -> bool {
        self.delay = if ran_for > Duration::from_secs(30) {
            RESTART_DELAY
        } else {
            (self.delay * 2).min(RESTART_DELAY_MAX)
        };
        thread::sleep(self.delay);
        !sh.stopped()
    }
}

pub struct Watcher {
    rx: Receiver<Signal>,
    shared: Arc<Shared>,
}

impl Watcher {
    pub fn start(show_closed: bool) -> Self {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            tx,
            show_closed: AtomicBool::new(show_closed),
            last_seq: AtomicU64::new(0),
            last_record: Mutex::new(Instant::now()),
            serve_url: Mutex::new(None),
            children: Mutex::new(Vec::new()),
            watching_manifests: AtomicBool::new(false),
            stop: AtomicBool::new(false),
        });
        let sh = Arc::clone(&shared);
        thread::spawn(move || {
            if dolt_mode() == "embedded" || !serve::run(&sh) {
                follow_embedded(&sh);
            }
        });
        Watcher { rx, shared }
    }

    /// Drain pending signals without blocking.
    pub fn poll(&self) -> Vec<Signal> {
        self.rx.try_iter().collect()
    }

    pub fn set_show_closed(&self, on: bool) {
        self.shared.show_closed.store(on, Ordering::Relaxed);
    }

    /// Reload the board for `scope` in the background. Repo loads go through
    /// the snapshot path, which replays records that arrive meanwhile; the
    /// global scope has no journal to follow.
    pub fn request_load(&self, scope: Scope) {
        let sh = Arc::clone(&self.shared);
        thread::spawn(move || {
            if scope == Scope::Repo {
                return sh.snapshot();
            }
            let show_closed = sh.show_closed.load(Ordering::Relaxed);
            let signal = match load(scope, show_closed) {
                Ok(beads) => Signal::Snapshot(Box::new(Snapshot {
                    scope,
                    beads,
                    after_seq: u64::MAX,
                    show_closed,
                    reset: false,
                })),
                Err(e) => Signal::LoadFailed {
                    scope,
                    error: e.to_string(),
                },
            };
            sh.send(signal);
        });
    }

    /// Load one bead for the detail pane in the background.
    pub fn request_show(&self, scope: Scope, id: String) {
        let sh = Arc::clone(&self.shared);
        thread::spawn(move || {
            let url = sh.serve_url.lock().ok().and_then(|u| u.clone());
            let shown = match (&url, scope) {
                (Some(base), Scope::Repo) => serve::show(base, &id),
                _ => show(scope, &id),
            };
            let bead = shown.ok().flatten().map(Box::new);
            sh.send(Signal::Detail { scope, id, bead });
        });
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Ok(mut c) = self.shared.children.lock() {
            for child in c.drain(..) {
                stop_child(child);
            }
        }
    }
}

/// Stop a child: ask politely so `bd serve` and Dolt can shut down cleanly,
/// then force it after a second.
fn stop_child(mut child: Child) {
    let _ = std::process::Command::new("/bin/kill")
        .args(["-TERM", &child.id().to_string()])
        .stderr(Stdio::null())
        .status();
    for _ in 0..20 {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn journal_enabled() -> bool {
    run(Scope::Repo, &["config", "get", "events-journal"])
        .map(|s| s.trim() == "true")
        .unwrap_or(false)
}

/// The journal head. bd 1.3 has no cheap way to ask in embedded mode, so this
/// reads the retained journal (bounded by bd's retention) and takes the last
/// seq, or the head bd reports when the start of it was pruned.
fn journal_head() -> Option<u64> {
    let out = command(Scope::Repo)
        .args(["events", "tail", "--since", "0", "--json"])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if let Some(head) = truncated_head(stdout.trim()) {
        return Some(head);
    }
    out.status.success().then(|| last_seq(&stdout))
}

pub(super) enum Follow {
    /// The checkpoint was below what the journal keeps; this is the head.
    Truncated(u64),
    /// The journal is off (bd serve refused to stream).
    Disabled,
    /// The follower exited for another reason.
    Ended,
    /// The board is gone.
    Closed,
}

fn follow_embedded(sh: &Arc<Shared>) {
    if !journal_enabled() {
        sh.load_blocked();
        return;
    }
    let Some(head) = journal_head() else {
        sh.load_blocked();
        return;
    };
    sh.rebaseline(head);
    sh.watch_manifests();
    let mut backoff = Backoff::new();
    loop {
        let started = Instant::now();
        match tail(sh, sh.last_seq()) {
            Follow::Closed | Follow::Disabled => return,
            Follow::Truncated(head) => {
                sh.rebaseline(head);
                if !backoff.wait(started.elapsed(), sh) {
                    return;
                }
            }
            Follow::Ended => {
                if !backoff.wait(started.elapsed(), sh) {
                    return;
                }
                // A head below our checkpoint means the journal was reset.
                match journal_head() {
                    Some(head) if head < sh.last_seq() => sh.rebaseline(head),
                    _ => {}
                }
            }
        }
    }
}

fn tail(sh: &Shared, since: u64) -> Follow {
    let mut cmd = command(Scope::Repo);
    cmd.args([
        "events",
        "tail",
        "--since",
        &since.to_string(),
        "--follow",
        "--json",
    ]);
    sh.follow(cmd, |sh, stdout| {
        let mut json = JsonLines::default();
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let Some(doc) = json.push(&line) else {
                continue;
            };
            if let Some(head) = truncated_head(&doc) {
                return Follow::Truncated(head);
            }
            if !sh.take(&doc) {
                return Follow::Closed;
            }
        }
        Follow::Ended
    })
}

/// Reassembles `bd events tail --json` output: records come one per line,
/// but an error such as `events_journal_truncated` is printed indented over
/// several lines.
#[derive(Default)]
struct JsonLines {
    buf: String,
}

impl JsonLines {
    /// Feed a line; returns a complete JSON document once one is assembled.
    fn push(&mut self, line: &str) -> Option<String> {
        if !line.trim_start().starts_with('{') && self.buf.is_empty() {
            return None;
        }
        // A one-line record always starts a new document; drop any leftover.
        if line.starts_with("{\"") {
            self.buf.clear();
        }
        if self.buf.len() > 1 << 20 {
            self.buf.clear();
        }
        self.buf.push_str(line);
        self.buf.push('\n');
        if serde_json::from_str::<serde::de::IgnoredAny>(&self.buf).is_ok() {
            Some(std::mem::take(&mut self.buf))
        } else {
            None
        }
    }
}

/// The manifests' contents. Only writes change them: reads such as `bd list`
/// touch the files' modification time, so that cannot be used.
fn manifest_state() -> Vec<Vec<u8>> {
    manifests()
        .iter()
        .filter_map(|m| std::fs::read(m).ok())
        .collect()
}

/// Reload when the database changed but no record arrived for it: a sync,
/// a merge, or anything else the journal does not record.
fn watch_manifests(sh: &Shared) {
    let mut seen = manifest_state();
    while !sh.stopped() {
        thread::sleep(Duration::from_secs(1));
        if manifest_state() == seen {
            continue;
        }
        thread::sleep(SYNC_QUIET / 2);
        let quiet = sh
            .last_record
            .lock()
            .map(|t| t.elapsed() >= SYNC_QUIET)
            .unwrap_or(false);
        if quiet {
            sh.snapshot();
        }
        seen = manifest_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_lines_handles_records_and_indented_errors() {
        let mut j = JsonLines::default();
        let rec = r#"{"seq":1,"op":"create","issue_id":"a-1","issue":{"id":"a-1","title":"has } brace"}}"#;
        assert_eq!(j.push(rec).as_deref().map(str::trim), Some(rec));
        let error = [
            "{",
            r#"  "code": "events_journal_truncated","#,
            r#"  "floor": 25,"#,
            r#"  "head": 24"#,
            "}",
        ];
        let mut doc = None;
        for line in error {
            doc = j.push(line);
        }
        assert_eq!(doc.as_deref().and_then(truncated_head), Some(24));
        assert!(j.push("note: the events journal is disabled").is_none());
        assert!(j.push(rec).is_some());
    }
}
