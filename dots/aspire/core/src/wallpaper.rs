//! Fondos de pantalla: detección de backends, aplicar por monitor y ajuste,
//! elegir uno aleatorio de una carpeta, restaurar el último.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const SUPPORTED_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "bmp", "gif", "mp4", "m4v", "webm", "mkv", "mov", "avi", "ogv",
];

const BACKENDS: [&str; 10] = [
    "swaybg",
    "swww",
    "awww",
    "wallutils",
    "hyprpaper",
    "mpvpaper",
    "gslapper",
    "xwallpaper",
    "feh",
    "linux-wallpaperengine",
];

pub fn backend_bin(name: &str) -> Option<String> {
    let command = match name {
        "wallutils" => "setwallpaper",
        "hyprpaper" => "hyprctl",
        other => other,
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(command))
        .find(|path| {
            use std::os::unix::fs::PermissionsExt;
            path.metadata().is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
        .map(|p| p.to_string_lossy().into_owned())
}

pub fn available_backends() -> Vec<String> {
    BACKENDS
        .iter()
        .filter(|b| backend_bin(b).is_some())
        .map(|s| s.to_string())
        .collect()
}

pub fn random_wallpaper(folder: &Path) -> Result<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(folder) {
        for entry in rd.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') {
                continue;
            }
            let ext_ok = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| SUPPORTED_EXTS.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false);
            if ext_ok {
                files.push(p);
            }
        }
    }
    if files.is_empty() {
        anyhow::bail!("no hay fondos soportados en {}", folder.display());
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as usize)
        .unwrap_or(0);
    Ok(files[nanos % files.len()].clone())
}

/// Aplica un fondo con swaybg (backend por defecto). Guarda last-wallpaper.
pub fn set_wallpaper_swaybg(path: &Path, fit: &str) -> Result<()> {
    if !path.is_file() {
        anyhow::bail!("el fondo no existe: {}", path.display());
    }
    let _ = std::process::Command::new("pkill")
        .args(["-x", "swaybg"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::process::Command::new("setsid")
        .arg("swaybg")
        .args(["-i", &path.to_string_lossy(), "-m", fit])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("no se pudo lanzar swaybg")?;
    crate::state::save_last_wallpaper(path)?;
    Ok(())
}

pub fn restore() -> Result<()> {
    let Some(path) = crate::state::last_wallpaper() else {
        return Ok(());
    };
    if path.is_file() {
        set_wallpaper_swaybg(&path, "fill")?;
    }
    Ok(())
}