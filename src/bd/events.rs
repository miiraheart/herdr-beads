//! bd's events journal (bd 1.3+): the record format and how a record is
//! applied to the board. Following the journal lives in `live`.
//!
//! Each record carries the bead's full state after the change, so the board
//! applies it without asking bd again. The state lacks the dependency list
//! and the counts that `bd list` returns, so those are kept from the board and
//! adjusted from `dep_add`, `dep_remove` and `comment` records.

use super::types::{Bead, Dependency};
use serde::Deserialize;
use std::collections::HashMap;

/// One journal record, from `bd events tail` or `bd serve`.
#[derive(Debug, Clone, Deserialize)]
pub struct Record {
    pub seq: u64,
    pub op: String,
    pub issue_id: String,
    /// The bead after the change; absent on delete.
    #[serde(default)]
    pub issue: Option<Bead>,
    #[serde(default)]
    pub dep: Option<DepRef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DepRef {
    pub kind: String,
    pub target: String,
}

/// bd's answer when a checkpoint is older than the journal keeps:
/// `{"code":"events_journal_truncated",...,"head":N}`. The board then reloads
/// in full and follows from `head`.
pub fn truncated_head(line: &str) -> Option<u64> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("code")?.as_str()? != "events_journal_truncated" {
        return None;
    }
    v.get("head")?.as_u64()
}

/// Apply a record to the board's beads. A bead the board never loaded is
/// added from the record, unless it is closed while closed beads are hidden.
/// An op this version does not know is applied like an update when it carries
/// the bead. Returns false when the record cannot be applied (no bead state),
/// in which case the caller reloads in full.
///
/// `replay` is set when a record is applied again on top of a reload, which
/// already counts its comment.
pub fn apply(beads: &mut Vec<Bead>, rec: &Record, show_closed: bool, replay: bool) -> bool {
    if rec.op == "delete" {
        beads.retain(|b| b.id != rec.issue_id);
        return true;
    }
    if rec.issue.is_none() {
        return false;
    }
    let added = match beads.iter_mut().find(|b| b.id == rec.issue_id) {
        Some(b) => merge(b, rec, replay),
        None => {
            let mut b = Bead::default();
            let added = merge(&mut b, rec, replay);
            if show_closed || !b.is_closed() {
                beads.push(b);
            }
            added
        }
    };
    if let Some(t) = target_of(rec) {
        if let Some(b) = beads.iter_mut().find(|b| b.id == t) {
            adjust_dependents(b, rec, added);
        }
    }
    true
}

/// Apply a record to the detail pane's cached beads, so a change never needs
/// another `bd show`.
pub fn apply_to_cache(cache: &mut HashMap<String, Bead>, rec: &Record) {
    if rec.op == "delete" {
        cache.remove(&rec.issue_id);
        return;
    }
    let mut added = false;
    if let Some(b) = cache.get_mut(&rec.issue_id) {
        added = merge(b, rec, false);
    }
    if let Some(t) = target_of(rec) {
        if let Some(b) = cache.get_mut(t) {
            adjust_dependents(b, rec, added);
        }
    }
}

fn target_of(rec: &Record) -> Option<&str> {
    rec.dep.as_ref().map(|d| d.target.as_str())
}

/// Take the record's state, keeping what the record does not carry. Returns
/// whether a dependency edge changed, so the other end is only adjusted once
/// even when a record is applied twice (after a reload, see `App`).
fn merge(b: &mut Bead, rec: &Record, replay: bool) -> bool {
    if let Some(new) = &rec.issue {
        let old = std::mem::replace(b, new.clone());
        b.dependencies = old.dependencies;
        b.dependency_count = old.dependency_count;
        b.dependent_count = old.dependent_count;
        b.comment_count = old.comment_count;
        if b.is_blocked {
            b.blocked_by = old.blocked_by;
        }
    }
    match (rec.op.as_str(), &rec.dep) {
        ("dep_add", Some(d)) => {
            let known = b
                .dependencies
                .iter()
                .any(|x| x.other_id() == Some(d.target.as_str()));
            if !known {
                b.dependencies.push(Dependency {
                    issue_id: Some(b.id.clone()),
                    depends_on_id: Some(d.target.clone()),
                    dep_type: Some(d.kind.clone()),
                    ..Default::default()
                });
                b.dependency_count += 1;
            }
            !known
        }
        ("dep_remove", Some(d)) => {
            let before = b.dependencies.len();
            b.dependencies
                .retain(|x| x.other_id() != Some(d.target.as_str()));
            let removed = b.dependencies.len() < before;
            if removed {
                b.dependency_count = b.dependency_count.saturating_sub(1);
            }
            removed
        }
        ("comment", _) => {
            if !replay {
                b.comment_count += 1;
            }
            false
        }
        _ => false,
    }
}

fn adjust_dependents(b: &mut Bead, rec: &Record, edge_changed: bool) {
    if !edge_changed {
        return;
    }
    match rec.op.as_str() {
        "dep_add" => b.dependent_count += 1,
        "dep_remove" => b.dependent_count = b.dependent_count.saturating_sub(1),
        _ => {}
    }
}

pub fn seq_of(line: &str) -> Option<u64> {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()?
        .get("seq")?
        .as_u64()
}

/// Highest `seq` in a JSON Lines dump (0 when the journal is empty).
pub fn last_seq(jsonl: &str) -> u64 {
    jsonl.lines().filter_map(seq_of).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_seq_of_journal_dump() {
        let s = include_str!("../../tests/fixtures/events.jsonl");
        assert_eq!(last_seq(s), 3);
    }

    /// `apply` with closed beads hidden, the board's default.
    fn apply(beads: &mut Vec<Bead>, rec: &Record) -> bool {
        super::apply(beads, rec, false, false)
    }

    fn rec(line: &str) -> Record {
        serde_json::from_str(line).expect("record parses")
    }

    fn board() -> Vec<Bead> {
        let s = include_str!("../../tests/fixtures/list.json");
        crate::bd::parse_list(s).unwrap()
    }

    #[test]
    fn real_journal_records_parse() {
        let s = include_str!("../../tests/fixtures/events.jsonl");
        let recs: Vec<Record> = s.lines().map(rec).collect();
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0].op, "create");
        assert_eq!(recs[0].issue.as_ref().unwrap().title, "a");
    }

    #[test]
    fn create_adds_a_bead() {
        let mut beads = board();
        let r = rec(
            r#"{"seq":9,"op":"create","issue_id":"demo-9","issue":{"id":"demo-9","title":"new","status":"open","priority":1,"issue_type":"task"}}"#,
        );
        assert!(apply(&mut beads, &r));
        assert_eq!(
            beads.iter().find(|b| b.id == "demo-9").unwrap().title,
            "new"
        );
    }

    #[test]
    fn update_replaces_fields_and_keeps_deps_and_counts() {
        let mut beads = board();
        let r = rec(
            r#"{"seq":9,"op":"close","issue_id":"demo-1","issue":{"id":"demo-1","title":"renamed","status":"closed","priority":0,"issue_type":"feature"}}"#,
        );
        assert!(apply(&mut beads, &r));
        let b = beads.iter().find(|b| b.id == "demo-1").unwrap();
        assert_eq!(
            (b.title.as_str(), b.status.as_str(), b.priority),
            ("renamed", "closed", 0)
        );
        assert_eq!(b.dependency_count, 1);
        assert_eq!(b.dependencies.len(), 1);
    }

    #[test]
    fn delete_removes_the_bead() {
        let mut beads = board();
        let r = rec(r#"{"seq":9,"op":"delete","issue_id":"demo-3","issue":null}"#);
        assert!(apply(&mut beads, &r));
        assert!(beads.iter().all(|b| b.id != "demo-3"));
    }

    #[test]
    fn dep_add_and_remove_update_both_ends() {
        let mut beads = board();
        let issue = r#""issue":{"id":"demo-3","title":"Fix flaky navigation test","status":"blocked","priority":0,"issue_type":"bug"}"#;
        let add = rec(&format!(
            r#"{{"seq":9,"op":"dep_add","issue_id":"demo-3",{issue},"dep":{{"kind":"blocks","target":"demo-2"}}}}"#
        ));
        assert!(apply(&mut beads, &add));
        let find = |beads: &Vec<Bead>, id: &str| beads.iter().find(|b| b.id == id).unwrap().clone();
        assert_eq!(find(&beads, "demo-3").dependency_count, 1);
        assert_eq!(
            find(&beads, "demo-3").dependencies[0].other_id(),
            Some("demo-2")
        );
        assert_eq!(find(&beads, "demo-2").dependent_count, 2);

        let remove = rec(&format!(
            r#"{{"seq":10,"op":"dep_remove","issue_id":"demo-3",{issue},"dep":{{"kind":"blocks","target":"demo-2"}}}}"#
        ));
        assert!(apply(&mut beads, &remove));
        assert_eq!(find(&beads, "demo-3").dependency_count, 0);
        assert!(find(&beads, "demo-3").dependencies.is_empty());
        assert_eq!(find(&beads, "demo-2").dependent_count, 1);
    }

    #[test]
    fn comment_bumps_the_count() {
        let mut beads = board();
        let r = rec(
            r#"{"seq":9,"op":"comment","issue_id":"demo-2","issue":{"id":"demo-2","title":"Design token pipeline","status":"in_progress","priority":1,"issue_type":"epic"},"comment":{"text":"hi"}}"#,
        );
        assert!(apply(&mut beads, &r));
        assert_eq!(
            beads
                .iter()
                .find(|b| b.id == "demo-2")
                .unwrap()
                .comment_count,
            3
        );
    }

    #[test]
    fn reopened_bead_is_added_and_hidden_closed_one_skipped() {
        let mut beads = board();
        let reopened = rec(
            r#"{"seq":9,"op":"update","issue_id":"demo-77","issue":{"id":"demo-77","title":"x","status":"open"}}"#,
        );
        assert!(apply(&mut beads, &reopened));
        assert!(beads.iter().any(|b| b.id == "demo-77"));

        let closed = rec(
            r#"{"seq":10,"op":"update","issue_id":"demo-78","issue":{"id":"demo-78","title":"y","status":"closed"}}"#,
        );
        assert!(apply(&mut beads, &closed));
        assert!(beads.iter().all(|b| b.id != "demo-78"));
        assert!(super::apply(&mut beads, &closed, true, false));
        assert!(beads.iter().any(|b| b.id == "demo-78"));
    }

    #[test]
    fn unknown_op_with_state_applies_like_an_update() {
        let mut beads = board();
        let r = rec(
            r#"{"seq":9,"op":"label_add","issue_id":"demo-1","issue":{"id":"demo-1","title":"labelled","status":"open"}}"#,
        );
        assert!(apply(&mut beads, &r));
        assert_eq!(
            beads.iter().find(|b| b.id == "demo-1").unwrap().title,
            "labelled"
        );
        let bare = rec(r#"{"seq":10,"op":"label_add","issue_id":"demo-1"}"#);
        assert!(!apply(&mut beads, &bare));
    }

    #[test]
    fn replayed_comment_is_not_counted_twice() {
        let mut beads = board();
        let r = rec(
            r#"{"seq":9,"op":"comment","issue_id":"demo-2","issue":{"id":"demo-2","title":"Design token pipeline","status":"in_progress","priority":1,"issue_type":"epic"}}"#,
        );
        assert!(super::apply(&mut beads, &r, false, true));
        assert_eq!(
            beads
                .iter()
                .find(|b| b.id == "demo-2")
                .unwrap()
                .comment_count,
            2
        );
    }

    #[test]
    fn cache_follows_the_same_changes() {
        let mut cache: HashMap<String, Bead> =
            board().into_iter().map(|b| (b.id.clone(), b)).collect();
        let r = rec(
            r#"{"seq":9,"op":"update","issue_id":"demo-1","issue":{"id":"demo-1","title":"renamed","status":"open","priority":2,"issue_type":"feature"}}"#,
        );
        apply_to_cache(&mut cache, &r);
        assert_eq!(cache["demo-1"].title, "renamed");
        assert_eq!(cache["demo-1"].dependencies.len(), 1);
        apply_to_cache(
            &mut cache,
            &rec(r#"{"seq":10,"op":"delete","issue_id":"demo-1"}"#),
        );
        assert!(!cache.contains_key("demo-1"));
    }

    #[test]
    fn truncated_reply_gives_the_head() {
        let line = r#"{"code":"events_journal_truncated","error":"x","floor":5,"head":20,"schema_version":1,"since":0}"#;
        assert_eq!(truncated_head(line), Some(20));
        assert_eq!(truncated_head(r#"{"seq":3,"op":"create"}"#), None);
        assert_eq!(truncated_head("not json"), None);
    }

    #[test]
    fn dep_add_applied_twice_counts_once() {
        let mut beads = board();
        let add = rec(
            r#"{"seq":9,"op":"dep_add","issue_id":"demo-3","issue":{"id":"demo-3","title":"t","status":"blocked","priority":0,"issue_type":"bug"},"dep":{"kind":"blocks","target":"demo-2"}}"#,
        );
        assert!(apply(&mut beads, &add));
        assert!(apply(&mut beads, &add));
        let find = |id: &str| beads.iter().find(|b| b.id == id).unwrap().clone();
        assert_eq!(find("demo-3").dependency_count, 1);
        assert_eq!(find("demo-3").dependencies.len(), 1);
        assert_eq!(find("demo-2").dependent_count, 2);
    }

    #[test]
    fn blocked_flag_follows_the_record() {
        let mut beads = board();
        beads[0].is_blocked = true;
        beads[0].blocked_by = vec!["demo-2".into()];
        let id = beads[0].id.clone();
        let still = rec(&format!(
            r#"{{"seq":9,"op":"update","issue_id":"{id}","issue":{{"id":"{id}","title":"t","status":"open","is_blocked":true}}}}"#
        ));
        assert!(apply(&mut beads, &still));
        assert!(beads[0].is_blocked);
        assert_eq!(beads[0].blocked_by, vec!["demo-2".to_string()]);
        // bd leaves `is_blocked` out when it is false.
        let unblocked = rec(&format!(
            r#"{{"seq":10,"op":"update","issue_id":"{id}","issue":{{"id":"{id}","title":"t","status":"open"}}}}"#
        ));
        assert!(apply(&mut beads, &unblocked));
        assert!(!beads[0].is_blocked);
        assert!(beads[0].blocked_by.is_empty());
    }

    #[test]
    fn empty_journal_starts_at_zero() {
        assert_eq!(last_seq(""), 0);
        assert_eq!(last_seq("note: the events journal is disabled\n"), 0);
    }
}
