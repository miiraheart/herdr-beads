//! Serde types mirroring `bd ... --json` output (bd 1.1.0).
//!
//! `bd list --json` and `bd ready --json` return a flat array of issues.
//! `bd show <id> --json` returns a one-element array whose `dependencies`
//! entries are FULL issue objects (with `id`/`title`), whereas in `list` the
//! `dependencies` entries are edge records (`issue_id`/`depends_on_id`/`type`).
//! Every field is optional-tolerant so both shapes parse.
//! Some fields exist only to be deserialized (schema completeness).
#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Dependency {
    // edge form (list)
    #[serde(default)]
    pub issue_id: Option<String>,
    #[serde(default)]
    pub depends_on_id: Option<String>,
    #[serde(rename = "type", default)]
    pub dep_type: Option<String>,
    // expanded form (show): a full issue
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

impl Dependency {
    /// The id of the other bead this edge points at, whichever shape we got.
    pub fn other_id(&self) -> Option<&str> {
        self.depends_on_id.as_deref().or(self.id.as_deref())
    }

    pub fn label(&self) -> String {
        match (
            self.other_id(),
            self.title.as_deref(),
            self.dep_type.as_deref(),
        ) {
            (Some(id), Some(t), Some(k)) => format!("{k}: {id} - {t}"),
            (Some(id), Some(t), None) => format!("{id} - {t}"),
            (Some(id), None, Some(k)) => format!("{k}: {id}"),
            (Some(id), None, None) => id.to_string(),
            _ => "(dependency)".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Bead {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub priority: u8,
    #[serde(rename = "issue_type", default)]
    pub issue_type: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub dependency_count: u32,
    #[serde(default)]
    pub dependent_count: u32,
    #[serde(default)]
    pub comment_count: u32,
}

fn default_status() -> String {
    "open".to_string()
}

impl Bead {
    /// Short assignee (local-part of the owner email, or "-").
    pub fn assignee(&self) -> &str {
        match &self.owner {
            Some(o) if !o.is_empty() => o.split('@').next().unwrap_or(o),
            _ => "-",
        }
    }

    pub fn is_closed(&self) -> bool {
        matches!(self.status.as_str(), "closed")
    }

    /// The blocking edges only (dependencies of kind blocks / depends-on).
    pub fn blocking_deps(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies.iter().filter(|d| {
            d.dep_type
                .as_deref()
                .map(|t| t == "blocks" || t == "depends-on" || t == "hard")
                .unwrap_or(true)
        })
    }

    /// A single-line search haystack.
    pub fn haystack(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.id, self.title, self.description, self.issue_type, self.status
        )
        .to_lowercase()
    }
}
