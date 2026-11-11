//! Iconos del tema: enlace a `~/.local/share/icons/` y ajuste GTK.
//! Revertir quita los enlaces del tema y restaura icons-global.

use crate::themes::ThemePackage;
use anyhow::{Context, Result};
use std::path::PathBuf;

fn home() -> Result<PathBuf> {
    dirs::home_dir().context("no se pudo obtener HOME")
}

fn remove_path(path: &PathBuf) {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_dir() && !meta.file_type().is_symlink() {
            let _ = std::fs::remove_dir_all(path);
        } else {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn symlink_dir(src: &std::path::Path, dst: &PathBuf) -> Result<()> {
    remove_path(dst);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dst)
            .with_context(|| format!("no se pudo enlazar {}", dst.display()))?;
    }
    Ok(())
}

pub fn set_gtk_icon_theme(name: &str) {
    for ver in ["gtk-3.0", "gtk-4.0"] {
        let Ok(h) = home() else { return };
        let path = h.join(".config").join(ver).join("settings.ini");
        if let Ok(text) = std::fs::read_to_string(&path) {
            let mut out = String::new();
            let mut found = false;
            let mut in_settings = false;
            for line in text.lines() {
                if line.trim() == "[Settings]" {
                    in_settings = true;
                    out.push_str(line);
                    out.push('\n');
                    continue;
                }
                if line.trim().starts_with('[') {
                    in_settings = false;
                }
                if in_settings && line.trim_start().starts_with("gtk-icon-theme-name=") {
                    out.push_str(&format!("gtk-icon-theme-name={name}"));
                    out.push('\n');
                    found = true;
                } else {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            if !found {
                if text.contains("[Settings]") {
                    let mut rebuilt = String::new();
                    let mut inserted = false;
                    for line in text.lines() {
                        rebuilt.push_str(line);
                        rebuilt.push('\n');
                        if line.trim() == "[Settings]" && !inserted {
                            rebuilt.push_str(&format!("gtk-icon-theme-name={name}\n"));
                            inserted = true;
                        }
                    }
                    let _ = std::fs::write(&path, rebuilt);
                } else {
                    let mut t = text;
                    if !t.ends_with('\n') {
                        t.push('\n');
                    }
                    t.push_str("[Settings]\ngtk-icon-theme-name=");
                    t.push_str(name);
                    t.push('\n');
                    let _ = std::fs::write(&path, t);
                }
            } else {
                let _ = std::fs::write(&path, out);
            }
        }
    }
}

pub fn apply(pkg: &ThemePackage) -> Result<()> {
    if let Some(icons) = &pkg.manifest.icons {
        if icons.source == "bundled" {
            let target = home()?.join(".local/share/icons").join(&icons.name);
            symlink_dir(&pkg.root.join("icons"), &target)?;
        }
        set_gtk_icon_theme(&icons.name);
    }
    Ok(())
}

/// Quita los enlaces de iconos que apuntan a temas.
pub fn revert() {
    let Ok(h) = home() else { return };
    let icons_dir = h.join(".local/share/icons");
    if let Ok(entries) = std::fs::read_dir(&icons_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = std::fs::symlink_metadata(&path) {
                if meta.file_type().is_symlink() {
                    if let Ok(target) = std::fs::read_link(&path) {
                        let ts = target.to_string_lossy();
                        if ts.contains("/aspire/themes/") || ts.contains("/.dots/assets/themes/") {
                            remove_path(&path);
                        }
                    }
                }
            }
        }
    }
}

pub fn revert_to_global() {
    revert();
    let icon = crate::state::icons_global().unwrap_or_else(|| "Papirus-Dark".to_string());
    set_gtk_icon_theme(&icon);
}