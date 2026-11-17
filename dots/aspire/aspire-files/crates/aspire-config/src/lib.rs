//! Configuración de aspire-files.
//!
//! El archivo vive en `~/.config/aspire-files/config.toml`. Se admiten dos
//! formatos: el documentado (`[general]`, `[view]`, `[window]`) y el antiguo
//! `[preferences]`, que se migra en memoria al cargar.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    #[default]
    Auto,
    Matugen,
    Tinty,
    None,
}

impl Engine {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Matugen => "matugen",
            Self::Tinty => "tinty",
            Self::None => "none",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub engine: Engine,
    pub colors_file: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            engine: Engine::Auto,
            colors_file: "~/.config/aspire-files/colors.css".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub folders_first: bool,
    pub expandable_folders: bool,
    pub open_action: String,
    pub optional_actions: Vec<String>,
}

impl Default for General {
    fn default() -> Self {
        Self {
            folders_first: true,
            expandable_folders: false,
            open_action: "double".into(),
            optional_actions: vec!["create_link".into(), "delete_permanently".into()],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewConfig {
    pub view_mode: String,
    pub icon_size: i32,
    pub show_hidden: bool,
    pub sort_by: String,
    pub sort_descending: bool,
    pub search_subfolders: bool,
    pub show_thumbnails: bool,
    pub count_items_in_folders: bool,
    pub date_format: String,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            view_mode: "grid".into(),
            icon_size: 128,
            show_hidden: false,
            sort_by: "modified".into(),
            sort_descending: false,
            search_subfolders: false,
            show_thumbnails: true,
            count_items_in_folders: true,
            date_format: "simple".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub sidebar_visible: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            sidebar_visible: true,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeConfig,
    pub general: General,
    pub view: ViewConfig,
    pub window: WindowConfig,
}

impl Config {
    /// Carga la configuración. Si no existe, devuelve los valores por defecto.
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("no se pudo leer {}", path.display()))?;
        let mut value: toml::Value = toml::from_str(&text)
            .with_context(|| format!("TOML inválido en {}", path.display()))?;
        migrate_legacy_preferences(&mut value);
        value
            .try_into()
            .with_context(|| format!("estructura inválida en {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(&path, text).with_context(|| format!("no se pudo escribir {}", path.display()))
    }

    /// Ruta del `colors.css` resuelta (con `~` expandido).
    pub fn colors_path(&self) -> PathBuf {
        expand_tilde(&self.theme.colors_file)
    }

    /// Motor efectivo: `auto` se resuelve leyendo `~/.config/color-engine/active`.
    pub fn resolved_engine(&self) -> Engine {
        match self.theme.engine {
            Engine::Auto => {
                let active = config_home().join("color-engine/active");
                match std::fs::read_to_string(active) {
                    Ok(text) if text.trim().eq_ignore_ascii_case("tinty") => Engine::Tinty,
                    _ => Engine::Matugen,
                }
            }
            other => other,
        }
    }
}

/// Migra el bloque `[preferences]` del formato antiguo a `[general]`/`[view]`.
fn migrate_legacy_preferences(value: &mut toml::Value) {
    let Some(table) = value.as_table_mut() else {
        return;
    };
    let Some(prefs) = table.remove("preferences") else {
        return;
    };
    let Some(prefs) = prefs.as_table() else {
        return;
    };

    let mut put = |section: &str, from: &str, to: &str| {
        let Some(v) = prefs.get(from).cloned() else {
            return;
        };
        let entry = table
            .entry(section.to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if let Some(t) = entry.as_table_mut() {
            t.entry(to.to_string()).or_insert(v);
        }
    };

    put("general", "folders_first", "folders_first");
    put("view", "icon_size", "icon_size");
    put("view", "show_hidden", "show_hidden");
    put("view", "sort_by", "sort_by");
    put("view", "sort_descending", "sort_descending");
    put("window", "sidebar_visible", "sidebar_visible");
    // `default_view` del formato viejo corresponde a `view_mode`.
    put("view", "default_view", "view_mode");
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

pub fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local/share"))
}

pub fn config_path() -> PathBuf {
    config_home().join("aspire-files/config.toml")
}

pub fn expand_tilde(input: &str) -> PathBuf {
    if input == "~" {
        home_dir()
    } else if let Some(rest) = input.strip_prefix("~/") {
        home_dir().join(rest)
    } else {
        PathBuf::from(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_preferences() {
        let text = r#"
[theme]
engine = "auto"

[preferences]
default_view = "list"
icon_size = 96
show_hidden = true
sidebar_visible = false
sort_by = "name"
sort_descending = true
folders_first = false
"#;
        let mut value: toml::Value = toml::from_str(text).unwrap();
        migrate_legacy_preferences(&mut value);
        let config: Config = value.try_into().unwrap();
        assert_eq!(config.view.view_mode, "list");
        assert_eq!(config.view.icon_size, 96);
        assert!(config.view.show_hidden);
        assert!(!config.window.sidebar_visible);
        assert_eq!(config.view.sort_by, "name");
        assert!(config.view.sort_descending);
        assert!(!config.general.folders_first);
    }

    #[test]
    fn tilde_expansion() {
        let home = home_dir();
        assert_eq!(expand_tilde("~/x"), home.join("x"));
        assert_eq!(expand_tilde("/tmp/x"), PathBuf::from("/tmp/x"));
        assert_eq!(expand_tilde("~"), home);
    }

    #[test]
    fn defaults_match_documented_values() {
        let config = Config::default();
        assert_eq!(config.theme.engine, Engine::Auto);
        assert_eq!(config.view.view_mode, "grid");
        assert_eq!(config.view.icon_size, 128);
        assert!(config.view.show_thumbnails);
        assert!(config.general.folders_first);
        assert!(config.window.sidebar_visible);
    }

    #[test]
    fn parses_documented_config() {
        let text = r#"
[theme]
engine = "tinty"
colors_file = "~/.config/theme-engine/colors.css"

[general]
folders_first = false
open_action = "single"

[view]
view_mode = "list"
icon_size = 64
date_format = "detailed"

[window]
sidebar_visible = false
"#;
        let mut value: toml::Value = toml::from_str(text).unwrap();
        migrate_legacy_preferences(&mut value);
        let config: Config = value.try_into().unwrap();
        assert_eq!(config.theme.engine, Engine::Tinty);
        assert_eq!(config.view.view_mode, "list");
        assert_eq!(config.view.icon_size, 64);
        assert_eq!(config.view.date_format, "detailed");
        assert_eq!(config.general.open_action, "single");
        assert!(!config.window.sidebar_visible);
    }

    #[test]
    fn legacy_values_do_not_override_new_sections() {
        let text = r#"
[view]
icon_size = 96

[preferences]
icon_size = 48
default_view = "list"
"#;
        let mut value: toml::Value = toml::from_str(text).unwrap();
        migrate_legacy_preferences(&mut value);
        let config: Config = value.try_into().unwrap();
        assert_eq!(config.view.icon_size, 96);
        assert_eq!(config.view.view_mode, "list");
    }

    #[test]
    fn colors_path_expands_tilde() {
        let mut config = Config::default();
        config.theme.colors_file = "~/.config/theme-engine/colors.css".into();
        assert_eq!(
            config.colors_path(),
            home_dir().join(".config/theme-engine/colors.css")
        );
    }

    #[test]
    fn engine_strings_are_stable() {
        assert_eq!(Engine::Auto.as_str(), "auto");
        assert_eq!(Engine::Matugen.as_str(), "matugen");
        assert_eq!(Engine::Tinty.as_str(), "tinty");
        assert_eq!(Engine::None.as_str(), "none");
    }

    #[test]
    fn saves_and_loads_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "aspire-config-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        let mut config = Config::default();
        config.view.view_mode = "list".into();
        config.view.show_hidden = true;
        config.theme.engine = Engine::None;
        std::fs::write(&path, toml::to_string_pretty(&config).unwrap()).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.view.view_mode, "list");
        assert!(loaded.view.show_hidden);
        assert_eq!(loaded.theme.engine, Engine::None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
