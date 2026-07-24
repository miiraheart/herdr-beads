//! Kanban: one column per status, cards sorted by priority. `v` then h/l retags
//! a card to the adjacent column via `bd update --status`.

use crate::app::App;
use crate::model::status_label;
use crate::ui::theme;
use crate::ui::widgets::truncate_soft;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, app: &mut App) {
    let cols = app.columns();
    if cols.is_empty() || area.width == 0 {
        return;
    }
    let n = cols.len();
    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
    // One-column gutter so adjacent boxes never share a border edge.
    let rects = Layout::horizontal(constraints).spacing(1).split(area);

    let sel_status = app.selected_bead().map(|b| b.status.clone());

    for (i, (status, col_ids)) in cols.iter().enumerate() {
        let rect = rects[i];
        if rect.width < 3 {
            continue;
        }
        let color = theme::status_color(status);
        let active = sel_status.as_deref() == Some(status.as_str());
        // Selected column's box lights up in its status color; the rest stay dim.
        let border_style = if active {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::OVERLAY0)
        };
        let title = Line::from(vec![
            Span::styled(
                format!(" {} ", status_label(status)),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} ", col_ids.len()),
                Style::default().fg(theme::OVERLAY0),
            ),
        ]);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(title)
            .style(Style::default().bg(Color::Reset));
        let inner = block.inner(rect);
        f.render_widget(block, rect);

        let cw = inner.width as usize;
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut ids: Vec<Option<String>> = Vec::new();
        for id in col_ids {
            if let Some(b) = app.beads.iter().find(|x| &x.id == id) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} ", theme::priority_glyph(b.priority)),
                        Style::default().fg(theme::priority_color(b.priority)),
                    ),
                    Span::styled(
                        truncate_soft(&b.id, cw.saturating_sub(5)),
                        Style::default().fg(theme::OVERLAY1),
                    ),
                ]));
                ids.push(Some(id.clone()));
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled(
                        truncate_soft(&b.title, cw.saturating_sub(2)),
                        Style::default().fg(theme::TEXT),
                    ),
                ]));
                ids.push(Some(id.clone()));
                lines.push(Line::raw(""));
                ids.push(None);
            }
        }
        super::render_lines(f, inner, app, lines, ids);
    }
}
