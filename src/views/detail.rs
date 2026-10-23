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

/// One `Line` per source line. A `Span` holding `\n` renders as one run-on
/// paragraph, so multi-line bead text has to be split before it reaches the
/// `Paragraph`. `\r\n` and lone `\r` endings count as line breaks too.
fn text_lines(text: &str, style: Style) -> Vec<Line<'static>> {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(|l| Line::from(Span::styled(l.to_string(), style)))
        .collect()
}

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
        lines.extend(text_lines(
            &b.description,
            Style::default().fg(theme::SUBTEXT),
        ));
        lines.push(Line::raw(""));
    }
    if b.is_blocked {
        let by = if b.blocked_by.is_empty() {
            "another bead".to_string()
        } else {
            b.blocked_by.join(", ")
        };
        lines.push(Line::from(Span::styled(
            format!("⛔ Blocked by: {by}"),
            Style::default().fg(theme::RED),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(lines: &[Line]) -> Vec<String> {
        lines.iter().map(|l| l.to_string()).collect()
    }

    #[test]
    fn text_lines_keeps_each_line_and_blank_lines() {
        let got = text_lines("Intro\n\n- one\n- two", Style::default());
        assert_eq!(plain(&got), ["Intro", "", "- one", "- two"]);
    }

    #[test]
    fn text_lines_strips_crlf() {
        let got = text_lines("a\r\n\r\nb\r\n", Style::default());
        assert_eq!(plain(&got), ["a", "", "b"]);
    }

    #[test]
    fn text_lines_splits_lone_cr() {
        let got = text_lines("a\rb", Style::default());
        assert_eq!(plain(&got), ["a", "b"]);
    }

    #[test]
    fn no_line_holds_a_newline() {
        let b: Bead = serde_json::from_str(
            r#"{"id":"x-1","title":"t","description":"first\nsecond\r\nthird"}"#,
        )
        .unwrap();
        let text = plain(&build_lines(&b));
        assert!(text.iter().all(|l| !l.contains('\n') && !l.contains('\r')));
        let at = text.iter().position(|l| l == "Description").unwrap();
        assert_eq!(text[at + 1..at + 4], ["first", "second", "third"]);
    }
}
