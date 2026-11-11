#![allow(dead_code, unused_variables, unused_mut)]
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

fn config_home() -> Result<PathBuf> {
    dirs::config_dir().context("no se pudo determinar XDG_CONFIG_HOME")
}

pub fn aspire_bar_dir() -> Result<PathBuf> {
    Ok(config_home()?.join("aspire-bar"))
}

pub fn waybar_dir() -> Result<PathBuf> {
    Ok(config_home()?.join("waybar"))
}

mod apply;
mod catalog;
mod css;
mod document;
mod generator;
mod model;
mod palette;
mod preview;
mod scripts;
mod style;
mod templates;
mod themes;
mod ui;

const CONFIG_PATH: &str = "~/.config/waybar/config.jsonc";
const STYLE_PATH: &str = "~/.config/waybar/style.css";
const THEMES_DIR: &str = "~/.dots/assets/waybar-themes";
const BACKUP_DIR: &str = "~/.config/waybar/backups";

fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        dirs::home_dir().unwrap().join(rest)
    } else {
        PathBuf::from(p)
    }
}

fn read_config(path: &Path) -> Result<serde_json::Value> {
    let text = std::fs::read_to_string(path).context("no se pudo leer config")?;
    // json5 handles JSONC (comments, trailing commas)
    let v: serde_json::Value = json5::from_str(&text).context("JSONC inválido")?;
    Ok(v)
}

fn validate_css(css: &str) -> Result<()> {
    // Basic: non-empty
    if css.trim().is_empty() {
        anyhow::bail!("CSS vacío");
    }
    Ok(())
}

fn backup() -> Result<PathBuf> {
    let dir = expand(BACKUP_DIR);
    std::fs::create_dir_all(&dir)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let dest = dir.join(format!("backup-{}", stamp));
    std::fs::create_dir_all(&dest)?;
    let cfg = expand(CONFIG_PATH);
    let sty = expand(STYLE_PATH);
    if cfg.is_file() {
        std::fs::copy(&cfg, dest.join("config.jsonc"))?;
    }
    if sty.is_file() {
        std::fs::copy(&sty, dest.join("style.css"))?;
    }
    // rotate: keep last 5
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    while entries.len() > 5 {
        let old = entries.remove(0).path();
        let _ = std::fs::remove_dir_all(&old);
    }
    Ok(dest)
}

fn cmd_check() -> Result<()> {
    let cfg = expand(CONFIG_PATH);
    let v = read_config(&cfg)?;
    println!("config OK: {}", cfg.display());
    let style = std::fs::read_to_string(expand(STYLE_PATH))?;
    validate_css(&style)?;
    println!("style OK");
    let _ = v;
    Ok(())
}

fn cmd_list() -> Result<()> {
    let dir = expand(THEMES_DIR);
    for e in std::fs::read_dir(&dir)? {
        let e = e?;
        if e.path().is_dir() {
            let name = e.file_name().to_string_lossy().to_string();
            println!("{}", name);
        }
    }
    Ok(())
}

fn cmd_apply(theme: &str) -> Result<()> {
    let tdir = expand(THEMES_DIR).join(theme);
    if !tdir.is_dir() {
        anyhow::bail!("tema no encontrado: {}", theme);
    }
    // backup
    let b = backup()?;
    println!("backup: {}", b.display());
    // copy config.jsonc and style.css
    let cfg_src = tdir.join("config.jsonc");
    let sty_src = tdir.join("style.css");
    if cfg_src.is_file() {
        std::fs::copy(&cfg_src, expand(CONFIG_PATH))?;
    }
    if sty_src.is_file() {
        std::fs::copy(&sty_src, expand(STYLE_PATH))?;
    }
    // reload waybar (matar instancias existentes y esperar)
    let _ = std::process::Command::new("pkill")
        .args(["-x", "waybar"])
        .status();
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let running = std::process::Command::new("pgrep")
            .args(["-x", "waybar"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !running {
            break;
        }
    }
    let _ = std::process::Command::new("setsid")
        .arg("waybar")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    println!("aplicado: {}", theme);
    Ok(())
}

fn cmd_restore() -> Result<()> {
    let dir = expand(BACKUP_DIR);
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    let last = entries.last().ok_or_else(|| anyhow::anyhow!("sin backups"))?;
    let p = last.path();
    if p.join("config.jsonc").is_file() {
        std::fs::copy(p.join("config.jsonc"), expand(CONFIG_PATH))?;
    }
    if p.join("style.css").is_file() {
        std::fs::copy(p.join("style.css"), expand(STYLE_PATH))?;
    }
    println!("restaurado: {}", p.display());
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let res = match args.get(1).map(|s| s.as_str()) {
        None => ui::run(),
        Some("check") => cmd_check(),
        Some("list") => cmd_list(),
        Some("apply") => {
            let theme = args.get(2).map(|s| s.as_str()).unwrap_or("");
            if theme.is_empty() {
                Err(anyhow::anyhow!("falta nombre de tema"))
            } else {
                cmd_apply(theme)
            }
        }
        Some("restore") => cmd_restore(),
        Some(other) => Err(anyhow::anyhow!("subcomando desconocido: {}", other)),
    };
    if let Err(e) = res {
        eprintln!("aspire-bar: {e:#}");
        std::process::exit(1);
    }
}