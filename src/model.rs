//! View-independent state: the three views, the scope (repo vs global), the
//! launch mode (docked vs floating popup), the table sort key, and the bd
//! status vocabulary that drives kanban columns and list groups.

/// The status columns/groups, in board order. Mirrors bd 1.1.0 built-ins.
/// Unknown/custom statuses are appended after these at render time.
pub const STATUS_ORDER: &[&str] = &[
    "open",
    "in_progress",
    "blocked",
    "hooked",
    "deferred",
    "pinned",
    "closed",
];

/// Glyph + category for a status (category unused for now but kept for parity
/// with `bd statuses`).
pub fn status_glyph(status: &str) -> &'static str {
    match status {
        "open" => "○",
        "in_progress" => "◐",
        "blocked" => "●",
        "deferred" => "❄",
        "closed" => "✓",
        "pinned" => "📌",
        "hooked" => "◇",
        _ => "•",
    }
}

pub fn status_label(status: &str) -> String {
    match status {
        "open" => "Open".into(),
        "in_progress" => "In Progress".into(),
        "blocked" => "Blocked".into(),
        "deferred" => "Deferred".into(),
        "closed" => "Closed".into(),
        "pinned" => "Pinned".into(),
        "hooked" => "Hooked".into(),
        other => {
            // Title-case a custom status for display.
            let mut c = other.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => other.into(),
            }
        }
    }
}

/// Rank a status for ordering; known statuses first (board order), unknowns last.
pub fn status_rank(status: &str) -> usize {
    STATUS_ORDER
        .iter()
        .position(|s| *s == status)
        .unwrap_or(STATUS_ORDER.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    List,
    Table,
    Kanban,
}

impl View {
    pub const ALL: [View; 3] = [View::List, View::Table, View::Kanban];

    pub fn title(self) -> &'static str {
        match self {
            View::List => "List",
            View::Table => "Table",
            View::Kanban => "Kanban",
        }
    }

    pub fn next(self) -> View {
        match self {
            View::List => View::Table,
            View::Table => View::Kanban,
            View::Kanban => View::List,
        }
    }

    pub fn prev(self) -> View {
        match self {
            View::List => View::Kanban,
            View::Table => View::List,
            View::Kanban => View::Table,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Repo,
    Global,
}

impl Scope {
    pub fn label(self) -> &'static str {
        match self {
            Scope::Repo => "repo",
            Scope::Global => "global",
        }
    }

    pub fn toggled(self) -> Scope {
        match self {
            Scope::Repo => Scope::Global,
            Scope::Global => Scope::Repo,
        }
    }
}

/// How the board was launched: a narrow docked side panel, or a wide floating
/// popup. Affects the default view and layout density only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Dock,
    Popup,
}

impl Mode {
    pub fn default_view(self) -> View {
        match self {
            Mode::Dock => View::List,
            Mode::Popup => View::Kanban,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Status,
    Priority,
    Changed,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Status => "status",
            SortKey::Priority => "priority",
            SortKey::Changed => "changed",
        }
    }

    pub fn next(self) -> SortKey {
        match self {
            SortKey::Status => SortKey::Priority,
            SortKey::Priority => SortKey::Changed,
            SortKey::Changed => SortKey::Status,
        }
    }
}
