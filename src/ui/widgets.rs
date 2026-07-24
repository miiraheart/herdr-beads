//! Shared UI helpers: activity bar, status bar, input prompt, help overlay.

use crate::app::App;
use crate::form::CreateForm;
use crate::keys::help_lines;
use crate::model::{status_label, View};
use crate::ui::theme;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub fn truncate(s: &str, w: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= w {
        let mut out: String = chars.into_iter().collect();
        while out.chars().count() < w {
            out.push(' ');
        }
        out
    } else if w == 0 {
        String::new()
    } else {
        let mut out: String = chars.into_iter().take(w.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

pub fn truncate_soft(s: &str, w: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= w {
        s.to_string()
    } else if w == 0 {
        String::new()
    } else {
        let mut out: String = chars.into_iter().take(w.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// The activity bar: [List] [Table] [Kanban]  + scope + counts + move badge.
/// Records tab rects into `app.hits.tabs` for mouse switching.
pub fn render_activity_bar(f: &mut Frame, area: Rect, app: &mut App) {
    let mut spans: Vec<Span> = Vec::new();
    let mut x = area.x + 1;
    spans.push(Span::styled(" ", Style::default()));

    for v in View::ALL {
        let label = format!(" {} ", v.title());
        let w = label.chars().count() as u16;
        let style = if v == app.view {
            Style::default()
                .fg(theme::BASE)
                .bg(theme::MAUVE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::SUBTEXT).bg(Color::Reset)
        };
        app.hits.tabs.push((v, Rect::new(x, area.y, w, 1)));
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
        x += w + 1;
    }

    // right side: scope, count, move-mode
    let mut right = format!("  scope:{}", app.scope.label());
    if app.show_closed {
        right.push_str(" +closed");
    }
    if !app.filter.is_empty() {
        right.push_str(&format!("  /{}", app.filter));
    }
    spans.push(Span::styled(right, Style::default().fg(theme::OVERLAY1)));

    if app.move_mode {
        spans.push(Span::styled(
            "  MOVE ",
            Style::default()
                .fg(theme::BASE)
                .bg(theme::PEACH)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Reset));
    f.render_widget(bar, area);
}

pub fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let hint = if app.move_mode {
        "MOVE: h/l retag · v/Esc exit"
    } else {
        "K view · j/k move · v move-mode · c claim · x close · a new · / filter · g scope · ? help"
    };
    let left = Span::styled(
        format!(" {} ", app.status_msg),
        Style::default().fg(theme::GREEN),
    );
    let right = Span::styled(hint, Style::default().fg(theme::OVERLAY0));
    let line = Line::from(vec![left, Span::raw("  "), right]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Reset)),
        area,
    );
}

pub fn centered_rect(px: u16, py: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - py) / 2),
        Constraint::Percentage(py),
        Constraint::Percentage((100 - py) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - px) / 2),
        Constraint::Percentage(px),
        Constraint::Percentage((100 - px) / 2),
    ])
    .split(v[1])[1]
}

pub fn render_input(f: &mut Frame, area: Rect, app: &App) {
    let Some(inp) = app.input.as_ref() else {
        return;
    };
    let w = area.width.min(70);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(w)) / 2,
        area.y + area.height / 2,
        w,
        3,
    );
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE))
        .title(format!(" {} ", inp.title))
        .style(Style::default().bg(Color::Reset));
    let text = Line::from(vec![
        Span::styled("› ", Style::default().fg(theme::MAUVE)),
        Span::styled(inp.buffer.clone(), Style::default().fg(theme::TEXT)),
        Span::styled("█", Style::default().fg(theme::MAUVE)),
    ]);
    f.render_widget(Paragraph::new(text).block(block), rect);
}

pub fn render_create_form(f: &mut Frame, area: Rect, form: &CreateForm) {
    use crate::form::{
        F_ASSIGNEE, F_BACKLOG, F_DESC, F_EPIC, F_LABELS, F_PRIORITY, F_TITLE, F_TYPE,
    };
    let rect = centered_rect(68, 84, area);
    f.render_widget(Clear, rect);
    let title = if form.edit_id.is_some() {
        " Edit bead - Tab/↑↓ field · ←→ choose · Space toggle · Enter save · Esc cancel "
    } else {
        " New bead - Tab/↑↓ field · ←→ choose · Space toggle · Enter create · Esc cancel "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE))
        .title(title)
        .style(Style::default().bg(Color::Reset));

    let cur = form.field;
    let label = |i: u8, name: &str| {
        let style = if i == cur {
            Style::default()
                .fg(theme::MAUVE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::OVERLAY1)
        };
        Span::styled(format!("  {name:<13}"), style)
    };
    let cursor = |i: u8| if i == cur { "█" } else { "" };
    let text = |s: &str, ph: &str| {
        if s.is_empty() {
            Span::styled(ph.to_string(), Style::default().fg(theme::OVERLAY0))
        } else {
            Span::styled(s.to_string(), Style::default().fg(theme::TEXT))
        }
    };

    let it = form.issue_type();
    let pname = match form.priority {
        0 => "critical",
        1 => "high",
        2 => "medium",
        3 => "low",
        _ => "backlog",
    };
    let check = if form.deferred { "[x]" } else { "[ ]" };
    let epic_fg = if form.parent_id().is_empty() {
        theme::OVERLAY1
    } else {
        theme::MAUVE
    };

    let lines = vec![
        Line::from(vec![
            label(F_TYPE, "Type"),
            Span::styled(
                format!("‹ {it} ›"),
                Style::default()
                    .fg(theme::type_color(it))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            label(F_PRIORITY, "Priority"),
            Span::styled(
                format!("‹ {} · {pname} ›", theme::priority_glyph(form.priority)),
                Style::default()
                    .fg(theme::priority_color(form.priority))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            label(F_TITLE, "Title"),
            text(&form.title, "what needs doing?"),
            Span::styled(cursor(F_TITLE), Style::default().fg(theme::MAUVE)),
        ]),
        Line::from(vec![
            label(F_DESC, "Description"),
            text(&form.description, "optional details / acceptance"),
            Span::styled(cursor(F_DESC), Style::default().fg(theme::MAUVE)),
        ]),
        Line::raw(""),
        Line::from(vec![
            label(F_ASSIGNEE, "Assignee"),
            text(&form.assignee, "unassigned"),
            Span::styled(cursor(F_ASSIGNEE), Style::default().fg(theme::MAUVE)),
        ]),
        Line::from(vec![
            label(F_EPIC, "Parent epic"),
            Span::styled(
                format!("‹ {} ›", form.epic_label()),
                Style::default().fg(epic_fg),
            ),
        ]),
        Line::from(vec![
            label(F_LABELS, "Labels"),
            text(&form.labels, "comma,separated"),
            Span::styled(cursor(F_LABELS), Style::default().fg(theme::MAUVE)),
        ]),
        Line::raw(""),
        Line::from(vec![
            label(F_BACKLOG, "Backlog"),
            Span::styled(
                format!("{check} start deferred  (Space)"),
                Style::default().fg(if form.deferred {
                    theme::YELLOW
                } else {
                    theme::OVERLAY0
                }),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

/// Status picker overlay: board statuses numbered 1-9 for a one-key retag.
pub fn render_status_pick(f: &mut Frame, area: Rect, app: &App) {
    let statuses = app.board_statuses();
    let mut spans: Vec<Span> = Vec::new();
    for (i, s) in statuses.iter().enumerate().take(9) {
        spans.push(Span::styled(
            format!(" {} ", i + 1),
            Style::default().fg(theme::OVERLAY0),
        ));
        spans.push(Span::styled(
            format!("{}  ", status_label(s)),
            Style::default()
                .fg(theme::status_color(s))
                .add_modifier(Modifier::BOLD),
        ));
    }
    let w = area.width.min(74);
    let rect = Rect::new(
        area.x + (area.width.saturating_sub(w)) / 2,
        area.y + area.height / 2,
        w,
        3,
    );
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE))
        .title(" Set status - press 1-9 (Esc cancels) ")
        .style(Style::default().bg(Color::Reset));
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), rect);
}

pub fn render_help(f: &mut Frame, area: Rect) {
    let rect = centered_rect(70, 80, area);
    f.render_widget(Clear, rect);
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "herdr-beads - keys",
            Style::default()
                .fg(theme::MAUVE)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
    ];
    for (k, d) in help_lines() {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<10}", k), Style::default().fg(theme::YELLOW)),
            Span::styled(d.to_string(), Style::default().fg(theme::SUBTEXT)),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "  any key to close",
        Style::default().fg(theme::OVERLAY0),
    )));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE))
        .style(Style::default().bg(Color::Reset));
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false }),
        rect,
    );
}
