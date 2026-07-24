//! Multi-field "new bead" form: type / priority / title / description /
//! assignee / parent epic / labels / backlog. Matches the fields bd create
//! supports (a terminal can't do the GUI's dropdowns/image-drop).

pub const TYPES: &[&str] = &[
    "task", "bug", "feature", "epic", "chore", "spike", "story", "decision",
];

// Field indices.
pub const F_TYPE: u8 = 0;
pub const F_PRIORITY: u8 = 1;
pub const F_TITLE: u8 = 2;
pub const F_DESC: u8 = 3;
pub const F_ASSIGNEE: u8 = 4;
pub const F_EPIC: u8 = 5;
pub const F_LABELS: u8 = 6;
pub const F_BACKLOG: u8 = 7;
pub const FIELDS: u8 = 8;

#[derive(Debug, Clone)]
pub struct CreateForm {
    pub title: String,
    pub description: String,
    pub assignee: String,
    pub labels: String,
    pub type_idx: usize,
    pub priority: u8,
    /// 0 = "No epic"; otherwise index+1 into `epics`.
    pub epic_idx: usize,
    pub deferred: bool,
    pub field: u8,
    /// (id, title) of epics available as a parent.
    pub epics: Vec<(String, String)>,
    /// Some(id) when the form edits an existing bead; None for a new bead.
    pub edit_id: Option<String>,
}

impl CreateForm {
    pub fn new(epics: Vec<(String, String)>) -> Self {
        CreateForm {
            title: String::new(),
            description: String::new(),
            assignee: String::new(),
            labels: String::new(),
            type_idx: 0,
            priority: 2,
            epic_idx: 0,
            deferred: false,
            field: F_TITLE,
            epics,
            edit_id: None,
        }
    }

    pub fn issue_type(&self) -> &'static str {
        TYPES[self.type_idx.min(TYPES.len() - 1)]
    }

    /// The selected parent epic id, or "" for none.
    pub fn parent_id(&self) -> &str {
        if self.epic_idx == 0 {
            ""
        } else {
            self.epics
                .get(self.epic_idx - 1)
                .map(|(id, _)| id.as_str())
                .unwrap_or("")
        }
    }

    pub fn epic_label(&self) -> String {
        if self.epic_idx == 0 {
            "No epic".to_string()
        } else {
            match self.epics.get(self.epic_idx - 1) {
                Some((id, title)) => {
                    format!("{id} - {}", title.chars().take(28).collect::<String>())
                }
                None => "No epic".to_string(),
            }
        }
    }

    pub fn next_field(&mut self) {
        self.field = (self.field + 1) % FIELDS;
    }

    pub fn prev_field(&mut self) {
        self.field = (self.field + FIELDS - 1) % FIELDS;
    }

    pub fn left(&mut self) {
        match self.field {
            F_TYPE => self.cycle_type(-1),
            F_PRIORITY => self.adjust_priority(-1),
            F_EPIC => self.cycle_epic(-1),
            _ => {}
        }
    }

    pub fn right(&mut self) {
        match self.field {
            F_TYPE => self.cycle_type(1),
            F_PRIORITY => self.adjust_priority(1),
            F_EPIC => self.cycle_epic(1),
            _ => {}
        }
    }

    fn cycle_type(&mut self, dir: i32) {
        let n = TYPES.len() as i32;
        self.type_idx = (((self.type_idx as i32 + dir) % n + n) % n) as usize;
    }

    fn adjust_priority(&mut self, dir: i32) {
        self.priority = (self.priority as i32 + dir).clamp(0, 4) as u8;
    }

    fn cycle_epic(&mut self, dir: i32) {
        let n = (self.epics.len() + 1) as i32; // +1 for "No epic"
        self.epic_idx = (((self.epic_idx as i32 + dir) % n + n) % n) as usize;
    }

    pub fn input_char(&mut self, c: char) {
        match self.field {
            F_TITLE => self.title.push(c),
            F_DESC => self.description.push(c),
            F_ASSIGNEE => self.assignee.push(c),
            F_LABELS => self.labels.push(c),
            F_PRIORITY => {
                if let Some(d) = c.to_digit(10) {
                    if d <= 4 {
                        self.priority = d as u8;
                    }
                }
            }
            F_BACKLOG if c == ' ' => {
                self.deferred = !self.deferred;
            }
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        let s = match self.field {
            F_TITLE => &mut self.title,
            F_DESC => &mut self.description,
            F_ASSIGNEE => &mut self.assignee,
            F_LABELS => &mut self.labels,
            _ => return,
        };
        s.pop();
    }
}
