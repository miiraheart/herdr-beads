//! A single-line text prompt overlay used for filter / close-reason / note /
//! priority / create. The App owns dispatch on submit (see `submit_input`).

#[derive(Debug, Clone)]
pub enum InputKind {
    Filter,
    /// close the given bead id with the typed reason (required)
    CloseReason(String),
    /// append a note to the given bead id
    Note(String),
    /// set priority on the given bead id
    Priority(String),
    /// add a comment to the given bead id
    Comment(String),
}

#[derive(Debug, Clone)]
pub struct Input {
    pub kind: InputKind,
    pub title: String,
    pub buffer: String,
}

impl Input {
    pub fn push(&mut self, c: char) {
        self.buffer.push(c);
    }

    pub fn backspace(&mut self) {
        self.buffer.pop();
    }
}
