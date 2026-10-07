//! The board's side of live updates: applying journal records, background
//! reloads and blocked state from `bd::live`.

use crate::app::{App, LOADING};
use crate::bd::events::{self, Record};
use crate::bd::live::{BlockedState, Snapshot};
use crate::bd::mark_blocked;
use crate::bd::types::Bead;
use crate::model::Scope;

/// Enough records to cover a background reload on a large workspace.
const RECENT_MAX: usize = 1000;

impl App {
    /// Apply one events-journal record. One that cannot be applied asks for
    /// a full reload in the background (see `events::apply`).
    pub fn apply_record(&mut self, rec: Record) {
        let was_count = self.status_msg == self.count_msg();
        let pos = self.selected_pos();
        let applied = events::apply(&mut self.beads, &rec, self.show_closed, false);
        if applied {
            events::apply_to_cache(&mut self.detail_cache, &rec);
        }
        self.remember(rec);
        if !applied {
            self.reload_wanted = true;
            return;
        }
        if was_count {
            self.status_msg = self.count_msg();
        }
        self.reselect_near(pos);
    }

    /// Take a full reload made in the background, then re-apply the records
    /// that arrived while it was loading, which it may not include.
    pub fn apply_snapshot(&mut self, snap: Snapshot) {
        if snap.scope != self.scope || snap.show_closed != self.show_closed {
            return;
        }
        if snap.reset {
            self.recent.clear();
        }
        let was_count = self.status_msg.is_empty()
            || self.status_msg == LOADING
            || self.status_msg == self.count_msg();
        let pos = self.selected_pos();
        self.beads = snap.beads;
        for rec in self.recent.iter().filter(|r| r.seq > snap.after_seq) {
            events::apply(&mut self.beads, rec, self.show_closed, true);
        }
        self.refresh_cache();
        if was_count {
            self.status_msg = self.count_msg();
        }
        self.reselect_near(pos);
        self.refresh_detail();
    }

    /// Take blocked state loaded in the background. Records that arrived
    /// while it was loading carry newer blocked flags, so they win.
    pub fn apply_blocked(&mut self, state: BlockedState) {
        mark_blocked(&mut self.beads, &state.blocked);
        for rec in self.recent.iter().filter(|r| r.seq > state.after_seq) {
            let Some(issue) = &rec.issue else { continue };
            if let Some(b) = self.beads.iter_mut().find(|b| b.id == rec.issue_id) {
                b.is_blocked = issue.is_blocked;
                if !b.is_blocked {
                    b.blocked_by.clear();
                }
            }
        }
    }

    pub fn take_reload_wanted(&mut self) -> bool {
        std::mem::take(&mut self.reload_wanted)
    }

    pub fn load_failed(&mut self, scope: Scope, error: &str) {
        if scope == self.scope {
            self.status_msg = App::load_error(scope, error);
        }
    }

    pub fn apply_detail(&mut self, scope: Scope, id: String, bead: Option<Bead>) {
        self.detail_loading.remove(&id);
        if let (true, Some(b)) = (scope == self.scope, bead) {
            self.detail_cache.insert(id, b);
        }
    }

    pub(crate) fn count_msg(&self) -> String {
        format!("{} beads · {}", self.beads.len(), self.scope.label())
    }

    fn remember(&mut self, rec: Record) {
        if self.recent.len() >= RECENT_MAX {
            self.recent.pop_front();
        }
        self.recent.push_back(rec);
    }

    /// Bring cached `bd show` beads in line with the board after a reload,
    /// keeping their expanded dependencies; drop the ones no longer there.
    fn refresh_cache(&mut self) {
        let beads = &self.beads;
        self.detail_cache.retain(|id, cached| {
            let Some(b) = beads.iter().find(|b| &b.id == id) else {
                return false;
            };
            let deps = std::mem::take(&mut cached.dependencies);
            *cached = b.clone();
            cached.dependencies = deps;
            true
        });
    }
}
