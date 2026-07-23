//! View renderers. `render_lines` is the shared vertical renderer: it draws a
//! prebuilt list of lines with per-row mouse hit rects, keeps the selection in
//! view via scroll, and paints a selection/move-mode highlight bar.

pub mod detail;
pub mod kanban;
pub mod list;
pub mod table;

use crate::app::App;
use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// `ids[i] == Some(id)` marks line i as a selectable bead row; `None` is a
/// header/spacer. Highlights every line whose id equals the selection.
pub fn render_lines(
    f: &mut Frame,
    area: Rect,
    app: &mut App,
    lines: Vec<Line<'static>>,
    ids: Vec<Option<String>>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let total = lines.len();
    let h = area.height as usize;
    let sel = app.selected.clone();
    let sel_idx = sel
        .as_ref()
        .and_then(|id| ids.iter().position(|x| x.as_deref() == Some(id.as_str())));
    let offset = match sel_idx {
        Some(s) if total > h => s.saturating_sub(h / 2).min(total.saturating_sub(h)),
        _ => 0,
    };

    // Transparent background: let the terminal (image/blur) show through.
    let para = Paragraph::new(lines)
        .scroll((offset as u16, 0))
        .style(Style::default().bg(Color::Reset));
    f.render_widget(para, area);

    let hl_bg = if app.move_mode {
        theme::SURFACE2
    } else {
        theme::SURFACE1
    };
    for (i, id) in ids.iter().enumerate() {
        if i < offset || i >= offset + h {
            continue;
        }
        let Some(id) = id else { continue };
        let dy = area.y + (i - offset) as u16;
        let rect = Rect::new(area.x, dy, area.width, 1);
        app.hits.rows.push((id.clone(), rect));
        if sel.as_deref() == Some(id.as_str()) {
            f.buffer_mut().set_style(rect, Style::default().bg(hl_bg));
        }
    }
}
