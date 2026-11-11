//! Capa de recarga: avisa a los programas del cambio de tema.
//!
//! Cada destino implementa `Reload`. Los fallos se registran y no abortan
//! el resto. Se ejecutan secuencialmente (idempotentes).

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Ctx<'a> {
    pub palette: Option<&'a crate::theme_engine::Palette>,
}

pub trait Reload {
    fn name(&self) -> &str;
    fn reload(&self, ctx: &Ctx) -> Result<()>;
}

// --- helpers ---

fn home() -> Result<PathBuf> {
    dirs::home_dir().context("no se pudo obtener HOME")
}

fn config_dir() -> Result<PathBuf> {
    Ok(home()?.join(".config"))
}

fn run(program: &str, args: &[&str]) {
    let _ = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn process_running(name: &str) -> bool {
    std::process::Command::new("pgrep")
        .args(["-x", name])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// --- Destinos ---

struct MangoReload;
impl Reload for MangoReload {
    fn name(&self) -> &str {
        "mango"
    }
    fn reload(&self, _ctx: &Ctx) -> Result<()> {
        run("mmsg", &["dispatch", "reload_config"]);
        Ok(())
    }
}

struct WaybarReload;
impl Reload for WaybarReload {
    fn name(&self) -> &str {
        "waybar"
    }
    fn reload(&self, _ctx: &Ctx) -> Result<()> {
        restart_waybar();
        Ok(())
    }
}

struct TerminalsReload;
impl Reload for TerminalsReload {
    fn name(&self) -> &str {
        "terminals"
    }
    fn reload(&self, _ctx: &Ctx) -> Result<()> {
        for (signal, name) in [
            ("-SIGUSR1", "foot"),
            ("-SIGUSR1", "footclient"),
            ("-SIGUSR1", "kitty"),
            ("-SIGUSR2", "ghostty"),
        ] {
            run("pkill", &[signal, name]);
        }
        if process_running("rio") {
            if let Ok(dir) = config_dir() {
                for path in [dir.join("rio/config.toml"), dir.join("rio/themes/matugen.toml")] {
                    let _ = std::process::Command::new("touch")
                        .arg(&path)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                }
            }
        }
        if process_running("aspire-files") {
            run("pkill", &["-USR1", "-x", "aspire-files"]);
        }
        Ok(())
    }
}

struct MakoReload;
impl Reload for MakoReload {
    fn name(&self) -> &str {
        "mako"
    }
    fn reload(&self, _ctx: &Ctx) -> Result<()> {
        if process_running("mako") {
            run("makoctl", &["reload"]);
        }
        Ok(())
    }
}

struct GtkReload;
impl Reload for GtkReload {
    fn name(&self) -> &str {
        "gtk"
    }
    fn reload(&self, _ctx: &Ctx) -> Result<()> {
        // settings.ini ya lo mantiene icons/fonts; aquí solo disparamos gsettings si existe.
        let icon = crate::state::icons_global().unwrap_or_else(|| "Papirus-Dark".to_string());
        for schema_key in ["org.gnome.desktop.interface icon-theme", "org.gnome.desktop.interface gtk-theme"] {
            let _ = schema_key;
        }
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "icon-theme", &icon])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.desktop.interface", "color-scheme", "prefer-dark"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        Ok(())
    }
}

struct QtReload;
impl Reload for QtReload {
    fn name(&self) -> &str {
        "qt"
    }
    fn reload(&self, _ctx: &Ctx) -> Result<()> {
        let icon = crate::state::icons_global().unwrap_or_else(|| "Papirus-Dark".to_string());
        let font = crate::state::font_global().unwrap_or_else(|| "Sans 11".to_string());
        for (dir_name, file_name) in [("qt5ct", "qt5ct.conf"), ("qt6ct", "qt6ct.conf")] {
            let Ok(h) = home() else { continue };
            let dir = h.join(".config").join(dir_name);
            if !dir.is_dir() {
                continue;
            }
            let path = dir.join(file_name);
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            // icon theme
            if text.contains("icon_theme=") {
                // replace line
                let mut out = String::new();
                for line in text.lines() {
                    if line.trim_start().starts_with("icon_theme=") {
                        out.push_str(&format!("icon_theme={icon}"));
                    } else {
                        out.push_str(line);
                    }
                    out.push('\n');
                }
                let _ = std::fs::write(&path, out);
            }
            let _ = font;
        }
        Ok(())
    }
}

// --- targets.toml handling ---

#[derive(serde::Deserialize, Default)]
struct RestartCfg {
    stop_command: Option<String>,
    start_command: Option<String>,
}

#[derive(serde::Deserialize, Default)]
pub struct TargetCfg {
    signal: Option<String>,
    process: Option<String>,
    command: Option<String>,
    restart: Option<RestartCfg>,
}

pub fn mango_target_handled(targets: &HashMap<String, TargetCfg>) -> bool {
    targets.get("mango").is_some_and(|t| {
        t.command.is_some()
            || t.restart.is_some()
            || (t.process.is_some() && t.signal.is_some())
    })
}

pub fn waybar_target_handled(targets: &HashMap<String, TargetCfg>) -> bool {
    targets.get("waybar").is_some_and(|t| {
        (t.process.as_deref() == Some("waybar") && t.signal.is_some())
            || t.command.is_some()
            || t.restart.is_some()
    })
}

struct TargetsReload;
impl Reload for TargetsReload {
    fn name(&self) -> &str {
        "targets"
    }
    fn reload(&self, _ctx: &Ctx) -> Result<()> {
        let targets_path = config_dir().ok().map(|d| d.join("theme-engine/targets.toml"));
        let mut used = false;
        if let Some(path) = targets_path.as_ref() {
            if path.is_file() {
                if let Ok(text) = std::fs::read_to_string(path) {
                    if let Ok(targets) =
                        toml::from_str::<HashMap<String, TargetCfg>>(&text)
                    {
                        used = true;
                        let mango_handled = mango_target_handled(&targets);
                        let waybar_handled = waybar_target_handled(&targets);
                        for (_name, t) in &targets {
                            if let (Some(sig), Some(proc)) = (&t.signal, &t.process) {
                                if process_running(proc) {
                                    let arg = if sig.starts_with('-') {
                                        sig.clone()
                                    } else {
                                        format!("-{}", sig)
                                    };
                                    run("pkill", &[&arg, "-x", proc]);
                                }
                            }
                            if let Some(cmd) = &t.command {
                                let _ = std::process::Command::new("sh")
                                    .args(["-c", cmd])
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .status();
                            }
                            if let Some(r) = &t.restart {
                                if let Some(stop) = &r.stop_command {
                                    let _ = std::process::Command::new("sh")
                                        .args(["-c", stop])
                                        .stdin(std::process::Stdio::null())
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .status();
                                }
                                if let Some(start) = &r.start_command {
                                    let _ = std::process::Command::new("sh")
                                        .args(["-c", start])
                                        .stdin(std::process::Stdio::null())
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .spawn();
                                }
                            }
                        }
                        if !mango_handled {
                            // mango fallback handled by MangoReload separately; skip here
                        }
                        if !waybar_handled {
                            // waybar fallback handled by WaybarReload separately; skip here
                        }
                        let _ = (mango_handled, waybar_handled);
                    }
                }
            }
        }
        if !used {
            // Fallback hardcoded handled by other destinations; nothing extra.
        }
        Ok(())
    }
}

// --- restart_waybar (moved from theme_engine) ---

fn resolve_import_path(base: &std::path::Path, url: &str) -> Option<PathBuf> {
    if let Some(path) = url.strip_prefix("file://") {
        if !path.starts_with('/') {
            return None;
        }
        return percent_decode_path(path).map(PathBuf::from);
    }
    Some(base.join(url))
}

fn percent_decode_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push((hex_digit(high)? << 4) | hex_digit(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub fn restart_waybar() {
    let base = config_dir().ok().map(|d| d.join("waybar")).unwrap_or_default();
    let style = base.join("style.css");
    if let Ok(text) = std::fs::read_to_string(&style) {
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("@import") {
                if let Some(start) = rest.find("url(") {
                    let after = &rest[start + 4..];
                    if let Some(end) = after.find(')') {
                        let url = after[..end].trim().trim_matches('"').trim_matches('\'');
                        if !url.is_empty() && !url.starts_with("http") {
                            let resolved = resolve_import_path(&base, url);
                            let Some(resolved) = resolved else {
                                eprintln!("waybar: no se pudo resolver el import CSS {url}, no se recarga");
                                return;
                            };
                            if !resolved.is_file() {
                                eprintln!(
                                    "waybar: import no resuelve ({}), no se recarga",
                                    resolved.display()
                                );
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
    let running = std::process::Command::new("pgrep")
        .args(["-x", "waybar"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !running {
        return;
    }
    run("pkill", &["-x", "waybar"]);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let _ = std::process::Command::new("setsid")
        .arg("waybar")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Punto de entrada de la capa de recarga.
pub fn reload_all(ctx: &Ctx) {
    let destinations: Vec<Box<dyn Reload>> = vec![
        Box::new(MangoReload),
        Box::new(WaybarReload),
        Box::new(TerminalsReload),
        Box::new(MakoReload),
        Box::new(GtkReload),
        Box::new(QtReload),
        Box::new(TargetsReload),
    ];
    for d in destinations {
        if let Err(e) = d.reload(ctx) {
            eprintln!("reload {}: {e:#}", d.name());
        }
    }
}