//! Catppuccin Macchiato palette (a dark terminal theme) + semantic mappings.
//! The full palette and a few helpers are kept complete for future views.
#![allow(dead_code)]

use ratatui::style::Color;

pub const BASE: Color = Color::Rgb(0x24, 0x27, 0x3a);
pub const MANTLE: Color = Color::Rgb(0x1e, 0x20, 0x30);
pub const SURFACE0: Color = Color::Rgb(0x36, 0x3a, 0x4f);
pub const SURFACE1: Color = Color::Rgb(0x49, 0x4d, 0x64);
pub const SURFACE2: Color = Color::Rgb(0x5b, 0x60, 0x78);
pub const OVERLAY0: Color = Color::Rgb(0x6e, 0x73, 0x8d);
pub const OVERLAY1: Color = Color::Rgb(0x8a, 0x8f, 0xa8);
pub const TEXT: Color = Color::Rgb(0xca, 0xd3, 0xf5);
pub const SUBTEXT: Color = Color::Rgb(0xb8, 0xc0, 0xe0);

pub const ROSEWATER: Color = Color::Rgb(0xf4, 0xdb, 0xd6);
pub const RED: Color = Color::Rgb(0xed, 0x87, 0x96);
pub const MAROON: Color = Color::Rgb(0xee, 0x99, 0xa0);
pub const PEACH: Color = Color::Rgb(0xf5, 0xa9, 0x7f);
pub const YELLOW: Color = Color::Rgb(0xee, 0xd4, 0x9f);
pub const GREEN: Color = Color::Rgb(0xa6, 0xda, 0x95);
pub const TEAL: Color = Color::Rgb(0x8b, 0xd5, 0xca);
pub const SKY: Color = Color::Rgb(0x91, 0xd7, 0xe3);
pub const BLUE: Color = Color::Rgb(0x8a, 0xad, 0xf4);
pub const LAVENDER: Color = Color::Rgb(0xb7, 0xbd, 0xf8);
pub const MAUVE: Color = Color::Rgb(0xc6, 0xa0, 0xf6);
pub const FLAMINGO: Color = Color::Rgb(0xf0, 0xc6, 0xc6);

/// Column/group accent for a status.
pub fn status_color(status: &str) -> Color {
    match status {
        "open" => BLUE,
        "in_progress" => YELLOW,
        "blocked" => RED,
        "deferred" => OVERLAY1,
        "closed" => GREEN,
        "pinned" => MAUVE,
        "hooked" => TEAL,
        _ => LAVENDER,
    }
}

/// Priority 0..=4 color (0 = critical/red .. 4 = backlog/overlay).
pub fn priority_color(p: u8) -> Color {
    match p {
        0 => RED,
        1 => PEACH,
        2 => YELLOW,
        3 => TEAL,
        _ => OVERLAY0,
    }
}

pub fn priority_glyph(p: u8) -> &'static str {
    match p {
        0 => "P0",
        1 => "P1",
        2 => "P2",
        3 => "P3",
        _ => "P4",
    }
}

/// Dim agent-state hint color (idle/working/blocked/unknown), never authoritative.
pub fn agent_color(state: &str) -> Color {
    match state {
        "blocked" => RED,
        "working" => BLUE,
        "idle" => GREEN,
        _ => OVERLAY0,
    }
}

/// A short 1-char tag for an issue type (`·` for a plain task).
pub fn type_glyph(t: &str) -> &'static str {
    match t {
        "bug" => "B",
        "feature" => "F",
        "epic" => "◆",
        "chore" => "C",
        "decision" => "D",
        "spike" => "S",
        "story" => "Y",
        "milestone" => "M",
        _ => "·",
    }
}

pub fn type_color(t: &str) -> Color {
    match t {
        "bug" => RED,
        "feature" => GREEN,
        "epic" => MAUVE,
        "chore" => OVERLAY1,
        "decision" => SKY,
        "spike" => PEACH,
        "story" => TEAL,
        "milestone" => YELLOW,
        _ => OVERLAY0,
    }
}
