//! Flat, aligned table view - the only re-sortable view (`o` cycles sort).

use crate::app::App;
use crate::model::{status_glyph, status_label};
use crate::ui::theme;
use crate::ui::widgets::{truncate, truncate_soft};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let w = area.width as usize;
    // Fixed columns; title takes the remainder.
    let id_w = 14;
    let pri_w = 3;
    let status_w = 13;
    let type_w = 9;
    let asg_w = 10;
    let fixed = 2 + id_w + 1 + pri_w + 1 + status_w + 1 + type_w + 1 + asg_w + 1;
    let title_w = w.saturating_sub(fixed).max(8);

    let header = Line::from(vec![Span::styled(
        format!(
            "  {:<id_w$} {:<pri_w$} {:<status_w$} {:<type_w$} {:<asg_w$} {}",
            "ID",
            "P",
            "STATUS",
            "TYPE",
            "OWNER",
            "TITLE",
            id_w = id_w,
            pri_w = pri_w,
            status_w = status_w,
            type_w = type_w,
            asg_w = asg_w,
        ),
        Style::default()
            .fg(theme::OVERLAY1)
            .add_modifier(Modifier::BOLD),
    )]);

    let mut lines: Vec<Line<'static>> = vec![header];
    let mut ids: Vec<Option<String>> = vec![None];

    for id in app.flat_order() {
        let Some(b) = app.beads.iter().find(|x| x.id == id) else {
            continue;
        };
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{:<id_w$} ", truncate(&b.id, id_w), id_w = id_w),
                Style::default().fg(theme::OVERLAY1),
            ),
            Span::styled(
                format!(
                    "{:<pri_w$} ",
                    theme::priority_glyph(b.priority),
                    pri_w = pri_w
                ),
                Style::default().fg(theme::priority_color(b.priority)),
            ),
            Span::styled(
                format!(
                    "{} {:<sw$} ",
                    status_glyph(&b.status),
                    truncate(&status_label(&b.status), status_w - 2),
                    sw = status_w - 2,
                ),
                Style::default().fg(theme::status_color(&b.status)),
            ),
            Span::styled(
                format!(
                    "{:<type_w$} ",
                    truncate(&b.issue_type, type_w),
                    type_w = type_w
                ),
                Style::default().fg(theme::SUBTEXT),
            ),
            Span::styled(
                format!("{:<asg_w$} ", truncate(b.assignee(), asg_w), asg_w = asg_w),
                Style::default().fg(theme::OVERLAY0),
            ),
            Span::styled(
                truncate_soft(&b.title, title_w),
                Style::default().fg(theme::TEXT),
            ),
        ]);
        lines.push(line);
        ids.push(Some(id));
    }

    super::render_lines(f, area, app, lines, ids);
}
