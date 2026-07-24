//! The detail pane/modal: full bead info, enriched by `bd show` when cached.

use crate::app::App;
use crate::bd::types::Bead;
use crate::model::{status_glyph, status_label};
use crate::ui::theme;
use crate::ui::widgets::centered_rect;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub fn build_lines(b: &Bead) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(Line::from(Span::styled(
        b.id.clone(),
        Style::default().fg(theme::OVERLAY1),
    )));
    lines.push(Line::from(Span::styled(
        b.title.clone(),
        Style::default()
            .fg(theme::TEXT)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!("{} {}", status_glyph(&b.status), status_label(&b.status)),
            Style::default().fg(theme::status_color(&b.status)),
        ),
        Span::raw("   "),
        Span::styled(
            theme::priority_glyph(b.priority).to_string(),
            Style::default().fg(theme::priority_color(b.priority)),
        ),
        Span::raw("   "),
        Span::styled(b.issue_type.clone(), Style::default().fg(theme::SUBTEXT)),
        Span::raw("   "),
        Span::styled(
            format!("@{}", b.assignee()),
            Style::default().fg(theme::OVERLAY0),
        ),
    ]));
    if let (Some(c), Some(u)) = (&b.created_at, &b.updated_at) {
        lines.push(Line::from(Span::styled(
            format!("created {c} · updated {u}"),
            Style::default().fg(theme::OVERLAY0),
        )));
    }
    lines.push(Line::raw(""));
    if !b.description.is_empty() {
        lines.push(Line::from(Span::styled(
            "Description",
            Style::default().fg(theme::YELLOW),
        )));
        lines.push(Line::from(Span::styled(
            b.description.clone(),
            Style::default().fg(theme::SUBTEXT),
        )));
        lines.push(Line::raw(""));
    }
    if !b.dependencies.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Dependencies ({})", b.dependencies.len()),
            Style::default().fg(theme::YELLOW),
        )));
        for d in &b.dependencies {
            lines.push(Line::from(Span::styled(
                format!("  • {}", d.label()),
                Style::default().fg(theme::SUBTEXT),
            )));
        }
        lines.push(Line::raw(""));
    }
    lines.push(Line::from(Span::styled(
        format!(
            "deps {} · dependents {} · comments {}",
            b.dependency_count, b.dependent_count, b.comment_count
        ),
        Style::default().fg(theme::OVERLAY0),
    )));
    lines
}

fn lines_for(app: &App) -> Vec<Line<'static>> {
    match app.detail_bead() {
        Some(b) => build_lines(&b),
        None => vec![Line::from(Span::styled(
            "no selection",
            Style::default().fg(theme::OVERLAY0),
        ))],
    }
}

pub fn render_side(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::SURFACE2))
        .title(" Detail ")
        .style(Style::default().bg(Color::Reset));
    f.render_widget(
        Paragraph::new(lines_for(app))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub fn render_modal(f: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(70, 80, area);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE))
        .title(" Detail - Esc to close ")
        .style(Style::default().bg(Color::Reset));
    f.render_widget(
        Paragraph::new(lines_for(app))
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}
