//! User settings from `config.toml` in the plugin config directory
//! (`herdr plugin config-dir herdr-beads`). A missing file or key keeps the
//! default.

use serde::Deserialize;
use std::path::PathBuf;

/// The largest `list_indent`, so a typo cannot push rows off the pane.
const MAX_LIST_INDENT: usize = 8;

/// herdr's per-plugin config directory, which it passes to every plugin
/// command. None outside herdr.
pub fn plugin_config_dir() -> Option<PathBuf> {
    std::env::var_os("HERDR_PLUGIN_CONFIG_DIR")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Columns each nesting level adds when the List view puts child beads
    /// under their parent. 0 means a flat list sorted by priority.
    pub list_indent: usize,
}

impl Config {
    /// The settings, or the defaults and the reason the file could not be used.
    pub fn load() -> (Config, Option<String>) {
        let Some(dir) = plugin_config_dir() else {
            return (Config::default(), None);
        };
        match std::fs::read_to_string(dir.join("config.toml")) {
            Ok(text) => match Config::parse(&text) {
                Ok(c) => (c, None),
                Err(e) => (Config::default(), Some(e)),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
            Err(e) => (Config::default(), Some(format!("config.toml: {e}"))),
        }
    }

    fn parse(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| match e.span() {
            Some(span) => {
                let line = text[..span.start].matches('\n').count() + 1;
                format!("config.toml line {line}: {}", e.message())
            }
            None => format!("config.toml: {}", e.message()),
        })?;
        if config.list_indent > MAX_LIST_INDENT {
            return Err(format!(
                "config.toml: list_indent must be 0 to {MAX_LIST_INDENT}"
            ));
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_list_indent_and_defaults_to_zero() {
        assert_eq!(Config::parse("list_indent = 3").unwrap().list_indent, 3);
        assert_eq!(Config::parse("").unwrap().list_indent, 0);
    }

    #[test]
    fn rejects_a_bad_value_or_unknown_key() {
        assert!(Config::parse("list_indent = -1").is_err());
        assert!(Config::parse("list_indnet = 2").is_err());
        assert!(Config::parse("list_indent = 9").is_err());
    }

    #[test]
    fn names_the_line_of_a_parse_error() {
        let e = Config::parse("# settings\nlist_indent = \"two\"").unwrap_err();
        assert!(e.starts_with("config.toml line 2:"), "{e}");
    }
}
