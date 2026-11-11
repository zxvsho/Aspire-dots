//! Estado centralizado del dotfile en `~/.config/color-engine/`.
//!
//! Un solo sitio para leer y escribir `active` (motor), `tinty-theme`,
//! `font-global`, `icons-global` y `last-wallpaper`. El resto del crate
//! (y los binarios `aspire-*`) consumen estas funciones en lugar de tocar
//! los archivos a mano.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

fn home() -> Result<PathBuf> {
    dirs::home_dir().context("no se pudo obtener HOME")
}

/// Lee el estado completo.
pub fn state() -> Result<State> {
    State::load()
}

pub fn state_dir() -> Result<PathBuf> {
    let dir = home()?.join(".config/color-engine");
    std::fs::create_dir_all(&dir).context("no se pudo crear ~/.config/color-engine")?;
    Ok(dir)
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub active: String,
    pub tinty_theme: Option<String>,
    pub font_global: Option<String>,
    pub icons_global: Option<String>,
    pub last_wallpaper: Option<PathBuf>,
}

impl State {
    pub fn load() -> Result<Self> {
        let dir = state_dir()?;
        let active = std::fs::read_to_string(dir.join("active"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "matugen".to_string());
        let tinty_theme = std::fs::read_to_string(dir.join("tinty-theme"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let font_global = std::fs::read_to_string(dir.join("font-global"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let icons_global = std::fs::read_to_string(dir.join("icons-global"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let last_wallpaper = std::fs::read_to_string(dir.join("last-wallpaper"))
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        Ok(Self {
            active,
            tinty_theme,
            font_global,
            icons_global,
            last_wallpaper,
        })
    }

    pub fn save(&self) -> Result<()> {
        let dir = state_dir()?;
        std::fs::write(dir.join("active"), format!("{}\n", self.active))
            .context("no se pudo guardar active")?;
        match &self.tinty_theme {
            Some(v) => std::fs::write(dir.join("tinty-theme"), format!("{v}\n"))?,
            None => {
                let _ = std::fs::remove_file(dir.join("tinty-theme"));
            }
        }
        match &self.font_global {
            Some(v) => std::fs::write(dir.join("font-global"), format!("{v}\n"))?,
            None => {
                let _ = std::fs::remove_file(dir.join("font-global"));
            }
        }
        match &self.icons_global {
            Some(v) => std::fs::write(dir.join("icons-global"), format!("{v}\n"))?,
            None => {
                let _ = std::fs::remove_file(dir.join("icons-global"));
            }
        }
        match &self.last_wallpaper {
            Some(v) => std::fs::write(dir.join("last-wallpaper"), format!("{}\n", v.display()))?,
            None => {
                let _ = std::fs::remove_file(dir.join("last-wallpaper"));
            }
        }
        Ok(())
    }
}

// --- Granular accessors (compatibilidad con el código existente) ---

pub fn active_engine_str() -> String {
    State::load().map(|s| s.active).unwrap_or_else(|_| "matugen".to_string())
}

pub fn set_active_engine_str(value: &str) -> Result<()> {
    let mut s = State::load().unwrap_or_default();
    s.active = value.to_string();
    s.save()
}

pub fn tinty_theme() -> Option<String> {
    State::load().ok().and_then(|s| s.tinty_theme)
}

pub fn save_tinty_theme(value: &str) -> Result<()> {
    let mut s = State::load().unwrap_or_default();
    s.tinty_theme = Some(value.to_string());
    s.save()
}

pub fn font_global() -> Option<String> {
    State::load().ok().and_then(|s| s.font_global)
}

pub fn save_font_global(value: &str) -> Result<()> {
    let mut s = State::load().unwrap_or_default();
    s.font_global = Some(value.to_string());
    s.save()
}

pub fn icons_global() -> Option<String> {
    State::load().ok().and_then(|s| s.icons_global)
}

pub fn save_icons_global(value: &str) -> Result<()> {
    let mut s = State::load().unwrap_or_default();
    s.icons_global = Some(value.to_string());
    s.save()
}

pub fn last_wallpaper() -> Option<PathBuf> {
    State::load().ok().and_then(|s| s.last_wallpaper)
}

pub fn save_last_wallpaper(value: &Path) -> Result<()> {
    let mut s = State::load().unwrap_or_default();
    s.last_wallpaper = Some(value.to_path_buf());
    s.save()
}