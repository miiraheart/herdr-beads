//! Grouped-by-status list (phin-board style): collapsible status groups, one
//! bead per line, with a side/modal detail pane driven by the App.

use crate::app::App;
use crate::bd::types::Bead;
use crate::model::{status_glyph, status_label};
use crate::ui::theme;
use crate::ui::widgets::truncate_soft;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

pub fn bead_line(b: &Bead, width: u16) -> Line<'static> {
    // Fixed prefix before the title: "  " (2) + "P0 " (3) + type+space (2)
    // + "{id:<13} " (14) = 21 cols. Reserve the " ⛓N" suffix only when the
    // bead actually carries deps (chain glyph renders 2 cells).
    let dep_w = if b.dependency_count > 0 {
        3 + b.dependency_count.to_string().len()
    } else {
        0
    };
    let title_w = (width as usize).saturating_sub(21 + dep_w).max(6);
    let mut spans = vec![
        Span::raw("  "),
        Span::styled(
            format!("{} ", theme::priority_glyph(b.priority)),
            Style::default().fg(theme::priority_color(b.priority)),
        ),
        Span::styled(
            format!("{} ", theme::type_glyph(&b.issue_type)),
            Style::default().fg(theme::type_color(&b.issue_type)),
        ),
        Span::styled(
            format!("{:<13} ", truncate_soft(&b.id, 13)),
            Style::default().fg(theme::OVERLAY1),
        ),
        Span::styled(
            truncate_soft(&b.title, title_w),
            Style::default().fg(theme::TEXT),
        ),
    ];
    if b.dependency_count > 0 {
        spans.push(Span::styled(
            format!(" ⛓{}", b.dependency_count),
            Style::default().fg(theme::MAROON),
        ));
    }
    Line::from(spans)
}

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut ids: Vec<Option<String>> = Vec::new();

    for status in app.board_statuses() {
        let collapsed = app.collapsed.contains(&status);
        let group_ids = app.ids_in(&status);
        let arrow = if collapsed { "▸" } else { "▾" };
        let color = theme::status_color(&status);
        lines.push(Line::from(vec![
            Span::styled(format!("{arrow} "), Style::default().fg(color)),
            Span::styled(
                format!("{} {} ", status_glyph(&status), status_label(&status)),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({})", group_ids.len()),
                Style::default().fg(theme::OVERLAY0),
            ),
        ]));
        ids.push(None);

        if collapsed {
            continue;
        }
        for id in group_ids {
            if let Some(b) = app.beads.iter().find(|x| x.id == id) {
                lines.push(bead_line(b, area.width));
                ids.push(Some(id));
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "  no beads - press a to create, g for global scope",
            Style::default().fg(theme::OVERLAY0),
        )));
        ids.push(None);
    }

    super::render_lines(f, area, app, lines, ids);
}
