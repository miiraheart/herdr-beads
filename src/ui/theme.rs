//! Palettes (Catppuccin Macchiato by default, Solarized light on request) +
//! semantic mappings. The full palettes and a few helpers are kept complete
//! for future views.
#![allow(dead_code)]

use std::sync::OnceLock;

use ratatui::style::Color;

/// A palette: the colours every view draws with. Views read the active one
/// through [`p`].
pub struct Palette {
    pub base: Color,
    pub mantle: Color,
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,
    pub overlay0: Color,
    pub overlay1: Color,
    pub text: Color,
    pub subtext: Color,
    pub rosewater: Color,
    pub red: Color,
    pub maroon: Color,
    pub peach: Color,
    pub yellow: Color,
    pub green: Color,
    pub teal: Color,
    pub sky: Color,
    pub blue: Color,
    pub lavender: Color,
    pub mauve: Color,
    pub flamingo: Color,
}

/// Catppuccin Macchiato, a dark theme: the default.
pub static MACCHIATO: Palette = Palette {
    base: Color::Rgb(0x24, 0x27, 0x3a),
    mantle: Color::Rgb(0x1e, 0x20, 0x30),
    surface0: Color::Rgb(0x36, 0x3a, 0x4f),
    surface1: Color::Rgb(0x49, 0x4d, 0x64),
    surface2: Color::Rgb(0x5b, 0x60, 0x78),
    overlay0: Color::Rgb(0x6e, 0x73, 0x8d),
    overlay1: Color::Rgb(0x8a, 0x8f, 0xa8),
    text: Color::Rgb(0xca, 0xd3, 0xf5),
    subtext: Color::Rgb(0xb8, 0xc0, 0xe0),
    rosewater: Color::Rgb(0xf4, 0xdb, 0xd6),
    red: Color::Rgb(0xed, 0x87, 0x96),
    maroon: Color::Rgb(0xee, 0x99, 0xa0),
    peach: Color::Rgb(0xf5, 0xa9, 0x7f),
    yellow: Color::Rgb(0xee, 0xd4, 0x9f),
    green: Color::Rgb(0xa6, 0xda, 0x95),
    teal: Color::Rgb(0x8b, 0xd5, 0xca),
    sky: Color::Rgb(0x91, 0xd7, 0xe3),
    blue: Color::Rgb(0x8a, 0xad, 0xf4),
    lavender: Color::Rgb(0xb7, 0xbd, 0xf8),
    mauve: Color::Rgb(0xc6, 0xa0, 0xf6),
    flamingo: Color::Rgb(0xf0, 0xc6, 0xc6),
};

/// Solarized light (Ethan Schoonover), for light terminals.
pub static SOLARIZED_LIGHT: Palette = Palette {
    base: Color::Rgb(0xfd, 0xf6, 0xe3),
    mantle: Color::Rgb(0xee, 0xe8, 0xd5),
    surface0: Color::Rgb(0xee, 0xe8, 0xd5),
    surface1: Color::Rgb(0xe4, 0xdd, 0xc8),
    surface2: Color::Rgb(0xd6, 0xcf, 0xba),
    overlay0: Color::Rgb(0x93, 0xa1, 0xa1),
    overlay1: Color::Rgb(0x83, 0x94, 0x96),
    text: Color::Rgb(0x58, 0x6e, 0x75),
    subtext: Color::Rgb(0x65, 0x7b, 0x83),
    rosewater: Color::Rgb(0xd3, 0x36, 0x82),
    red: Color::Rgb(0xdc, 0x32, 0x2f),
    maroon: Color::Rgb(0xdc, 0x32, 0x2f),
    peach: Color::Rgb(0xcb, 0x4b, 0x16),
    yellow: Color::Rgb(0xb5, 0x89, 0x00),
    green: Color::Rgb(0x85, 0x99, 0x00),
    teal: Color::Rgb(0x2a, 0xa1, 0x98),
    sky: Color::Rgb(0x2a, 0xa1, 0x98),
    blue: Color::Rgb(0x26, 0x8b, 0xd2),
    lavender: Color::Rgb(0x6c, 0x71, 0xc4),
    mauve: Color::Rgb(0xd3, 0x36, 0x82),
    flamingo: Color::Rgb(0xd3, 0x36, 0x82),
};

static ACTIVE: OnceLock<&'static Palette> = OnceLock::new();

/// The palette for a theme name; None for an unknown name.
pub fn by_name(name: &str) -> Option<&'static Palette> {
    match name.trim() {
        "macchiato" | "catppuccin-macchiato" => Some(&MACCHIATO),
        "solarized-light" => Some(&SOLARIZED_LIGHT),
        _ => None,
    }
}

/// The theme name the user chose: HERDR_BEADS_THEME, else the `theme` file in
/// the plugin config directory (HERDR_PLUGIN_CONFIG_DIR), the same place as the
/// auto-dock marker. None means the default.
pub fn configured_name() -> Option<String> {
    if let Ok(name) = std::env::var("HERDR_BEADS_THEME") {
        if !name.trim().is_empty() {
            return Some(name.trim().to_string());
        }
    }
    let dir = std::env::var("HERDR_PLUGIN_CONFIG_DIR")
        .ok()
        .filter(|d| !d.is_empty())?;
    let name = std::fs::read_to_string(std::path::Path::new(&dir).join("theme")).ok()?;
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Choose the palette once, at start-up. An unknown name falls back to the
/// default and returns the name, so the caller can say so.
pub fn init() -> Option<String> {
    let name = configured_name();
    let palette = name.as_deref().and_then(by_name).unwrap_or(&MACCHIATO);
    let _ = ACTIVE.set(palette);
    name.filter(|n| by_name(n).is_none())
}

/// The active palette (the default before [`init`]).
pub fn p() -> &'static Palette {
    ACTIVE.get().copied().unwrap_or(&MACCHIATO)
}

/// Column/group accent for a status.
pub fn status_color(status: &str) -> Color {
    match status {
        "open" => p().blue,
        "in_progress" => p().yellow,
        "blocked" => p().red,
        "deferred" => p().overlay1,
        "closed" => p().green,
        "pinned" => p().mauve,
        "hooked" => p().teal,
        _ => p().lavender,
    }
}

/// Priority 0..=4 color (0 = critical/red .. 4 = backlog/overlay).
pub fn priority_color(prio: u8) -> Color {
    match prio {
        0 => p().red,
        1 => p().peach,
        2 => p().yellow,
        3 => p().teal,
        _ => p().overlay0,
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
        "blocked" => p().red,
        "working" => p().blue,
        "idle" => p().green,
        _ => p().overlay0,
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
        "bug" => p().red,
        "feature" => p().green,
        "epic" => p().mauve,
        "chore" => p().overlay1,
        "decision" => p().sky,
        "spike" => p().peach,
        "story" => p().teal,
        "milestone" => p().yellow,
        _ => p().overlay0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_names() {
        assert!(std::ptr::eq(by_name("macchiato").unwrap(), &MACCHIATO));
        assert!(std::ptr::eq(
            by_name(" solarized-light\n").unwrap(),
            &SOLARIZED_LIGHT
        ));
        assert!(by_name("dracula").is_none());
    }

    #[test]
    fn solarized_text_is_dark() {
        // On a light background the body text must be dark: base01.
        assert_eq!(SOLARIZED_LIGHT.text, Color::Rgb(0x58, 0x6e, 0x75));
    }
}
