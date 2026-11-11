//! Fuentes del tema: enlace a `~/.local/share/fonts/aspire/theme/`, fc-cache,
//! fijar la familia. Revertir restaura font-global.

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

fn run(program: &str, args: &[&str]) {
    let _ = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Escribe la familia de fuente en settings.ini (GTK 3/4) y gsettings.
pub fn set_gtk_font(family: &str) {
    for ver in ["gtk-3.0", "gtk-4.0"] {
        let Ok(h) = home() else { return };
        let path = h.join(".config").join(ver).join("settings.ini");
        let mut text = std::fs::read_to_string(&path).unwrap_or_default();
        if !text.contains("[Settings]") {
            if !text.ends_with('\n') && !text.is_empty() {
                text.push('\n');
            }
            text.push_str("[Settings]\n");
        }
        // replace or append gtk-font-name
        if text.contains("gtk-font-name=") {
            let mut out = String::new();
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
                if in_settings && line.trim_start().starts_with("gtk-font-name=") {
                    out.push_str(&format!("gtk-font-name={family}"));
                    out.push('\n');
                } else {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            let _ = std::fs::write(&path, out);
        } else {
            // insert after header
            let mut rebuilt = String::new();
            let mut inserted = false;
            for line in text.lines() {
                rebuilt.push_str(line);
                rebuilt.push('\n');
                if line.trim() == "[Settings]" && !inserted {
                    rebuilt.push_str(&format!("gtk-font-name={family}\n"));
                    inserted = true;
                }
            }
            if !inserted {
                rebuilt.push_str(&format!("gtk-font-name={family}\n"));
            }
            let _ = std::fs::write(&path, rebuilt);
        }
    }
    let _ = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "font-name", family])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Aplica la fuente del paquete si es bundled. No toca font-global.
pub fn apply(pkg: &ThemePackage) -> Result<()> {
    if let Some(font) = &pkg.manifest.font {
        if font.source == "bundled" {
            let target = home()?.join(".local/share/fonts/aspire/theme");
            symlink_dir(&pkg.root.join("fonts"), &target)?;
            run("fc-cache", &["-f", &target.to_string_lossy()]);
            if !font.family.is_empty() {
                set_gtk_font(&font.family);
            }
        } else if !font.family.is_empty() {
            set_gtk_font(&font.family);
        }
    }
    Ok(())
}

/// Revierte los overrides de fuente del tema.
pub fn revert() {
    let Ok(h) = home() else { return };
    let font_link = h.join(".local/share/fonts/aspire/theme");
    let had_font = font_link.exists() || std::fs::symlink_metadata(&font_link).is_ok();
    remove_path(&font_link);
    if had_font {
        run(
            "fc-cache",
            &["-f", &h.join(".local/share/fonts").to_string_lossy()],
        );
    }
    let family = crate::state::font_global().unwrap_or_else(|| "Sans 11".to_string());
    set_gtk_font(&family);
}