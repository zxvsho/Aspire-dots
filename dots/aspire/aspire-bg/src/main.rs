#![allow(unused)]
use aspire_core::colors::{self, RgbColor};
use aspire_core::theme_engine::{self, Engine};
use gdk_pixbuf::prelude::PixbufAnimationExtManual;
use gtk::gdk::{Key, ModifierType};
use gtk::glib::{self, Propagation};
use gtk::prelude::*;
#[allow(deprecated)]
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, ContentFit, CssProvider,
    Dialog, DropDown, Entry, FlowBox, Label, Orientation, Picture, PolicyType, ResponseType,
    ScrolledWindow,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use relm4::{ComponentParts, ComponentSender, RelmApp, SimpleComponent};
use std::cell::{Cell, RefCell};
#[cfg(feature = "wallhaven")]
use std::collections::HashSet;
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::rc::Rc;
#[cfg(feature = "wallhaven")]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const WIN_W: i32 = 1000;
const WIN_H: i32 = 640;
const THUMB_PX: i32 = 720;
const TILE_W: i32 = 300;
const TILE_H: i32 = 168;
const TILE_BORDER_PX: i32 = 2;
const CAROUSEL_BORDER_PX: i32 = 2;
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
const FILLS: [&str; 5] = ["fill", "stretch", "fit", "center", "tile"];
#[cfg(feature = "wallhaven")]
const WALLHAVEN_CATEGORIES: &str = "011";

#[cfg(feature = "wallhaven")]
mod wallhaven;

#[derive(Clone)]
struct WallhavenConfig {
    api_key: String,
    purity: String,
    categories: String,
    atleast: String,
}

#[allow(deprecated)]
fn move_wallhaven_selection(ui: &Ui, delta: isize, by_row: bool) {
    let (Some(grid), Some(scroll)) = (
        ui.wallhaven_grid.borrow().as_ref().cloned(),
        ui.wallhaven_scroll.borrow().as_ref().cloned(),
    ) else {
        return;
    };
    let mut count = 0_usize;
    while grid.child_at_index(count as i32).is_some() {
        count += 1;
    }
    if count == 0 {
        return;
    }
    let step = if by_row { delta * 3 } else { delta };
    let next = (ui.wallhaven_index.get() as isize + step).rem_euclid(count as isize) as usize;
    ui.wallhaven_index.set(next);
    if let Some(child) = grid.child_at_index(next as i32) {
        grid.select_child(&child);
        let alloc = child.allocation();
        let view_h = scroll.allocation().height();
        if alloc.height() > 0 && view_h > 0 {
            let adj = scroll.vadjustment();
            let target = (alloc.y() + alloc.height() / 2 - view_h / 2) as f64;
            let max = (adj.upper() - adj.page_size()).max(0.0);
            adj.set_value(target.clamp(0.0, max));
        }
    }
}

#[derive(Clone)]
struct BgConfig {
    folder: String,
    backend: String,
    monitors: String,
    monitor_wallpapers: HashMap<String, String>,
    fill: String,
    sort: String,
    sort_mode: String,
    subfolders: bool,
    show_hidden: bool,
    zen_mode: bool,
    carousel_mode: bool,
    color: String,
    wallpaper: String,
    post_command: String,
    wallhaven: WallhavenConfig,
}

fn home() -> String {
    dirs::home_dir()
        .map(|h| h.display().to_string())
        .unwrap_or_default()
}

fn ini_path() -> String {
    if let Ok(path) = std::env::var("ASPIRE_BG_CONFIG") {
        return path;
    }
    format!("{}/.config/aspire-bg/config.ini", home())
}

fn legacy_ini_path() -> String {
    format!("{}/.config/waypaper/config.ini", home())
}

/// Primera ejecución: hereda la config de waypaper (si existe) una sola vez.
/// El `post_command` antiguo se vacía porque el color ya lo lleva aspire-bg.
fn ensure_config() {
    let path = ini_path();
    if std::path::Path::new(&path).is_file() {
        return;
    }
    let Ok(text) = std::fs::read_to_string(legacy_ini_path()) else {
        return;
    };
    if let Some(dir) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if line.trim_start().starts_with("post_command") && line.contains("cambio_color.sh") {
            out.push_str("post_command = ");
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    let _ = std::fs::write(&path, out);
}

fn load_config() -> BgConfig {
    ensure_config();
    let mut cfg = BgConfig {
        folder: format!("{}/Imágenes/Fondos", home()),
        backend: "swaybg".to_string(),
        monitors: "All".to_string(),
        monitor_wallpapers: HashMap::new(),
        fill: "fill".to_string(),
        sort: "name".to_string(),
        sort_mode: "random".to_string(),
        subfolders: false,
        show_hidden: false,
        zen_mode: false,
        carousel_mode: false,
        color: "#000000".to_string(),
        wallpaper: String::new(),
        post_command: String::new(),
        wallhaven: WallhavenConfig {
            api_key: String::new(),
            purity: "100".to_string(),
            categories: "011".to_string(),
            atleast: "1920x1080".to_string(),
        },
    };
    let Ok(text) = std::fs::read_to_string(ini_path()) else {
        return cfg;
    };
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_lowercase();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim();
        if section == "wallhaven" {
            match k {
                "api_key" => cfg.wallhaven.api_key = v.to_string(),
                "purity" => cfg.wallhaven.purity = "100".to_string(),
                "categories" => cfg.wallhaven.categories = "011".to_string(),
                "atleast" => cfg.wallhaven.atleast = v.to_string(),
                _ => {}
            }
        } else {
            match k {
                "folder" => cfg.folder = shellexpand_home(v),
                "backend" => cfg.backend = v.to_string(),
                "monitors" => cfg.monitors = v.to_string(),
                "monitor_wallpapers" => match decode_monitor_wallpapers(v) {
                    Some(wallpapers) => cfg.monitor_wallpapers = wallpapers,
                    None => eprintln!("aspire-bg: invalid monitor_wallpapers value in config"),
                },
                "fill" => cfg.fill = v.to_string(),
                "sort" => cfg.sort = v.to_string(),
                "sort_mode" => cfg.sort_mode = v.to_string(),
                "subfolders" => cfg.subfolders = v.eq_ignore_ascii_case("true"),
                "show_hidden" => cfg.show_hidden = v.eq_ignore_ascii_case("true"),
                "zen_mode" => cfg.zen_mode = v.eq_ignore_ascii_case("true"),
                "carousel_mode" => cfg.carousel_mode = v.eq_ignore_ascii_case("true"),
                "color" => cfg.color = v.to_string(),
                "wallpaper" => cfg.wallpaper = shellexpand_home(v),
                "post_command" => cfg.post_command = v.to_string(),
                _ => {}
            }
        }
    }
    cfg
}

fn shellexpand_home(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~/") {
        format!("{}/{rest}", home())
    } else {
        s.to_string()
    }
}

fn encode_monitor_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn decode_monitor_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let pair = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            decoded.push(u8::from_str_radix(pair, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn encode_monitor_wallpapers(wallpapers: &HashMap<String, String>) -> String {
    let mut entries: Vec<_> = wallpapers.iter().collect();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    entries
        .into_iter()
        .map(|(monitor, path)| {
            format!(
                "{}={}",
                encode_monitor_component(monitor),
                encode_monitor_component(path)
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn decode_monitor_wallpapers(value: &str) -> Option<HashMap<String, String>> {
    let mut wallpapers = HashMap::new();
    if value.trim().is_empty() {
        return Some(wallpapers);
    }
    for entry in value.split(',') {
        let (monitor, path) = entry.split_once('=')?;
        let monitor = decode_monitor_component(monitor)?;
        let path = decode_monitor_component(path)?;
        if monitor.is_empty() || path.is_empty() {
            return None;
        }
        wallpapers.insert(monitor, path);
    }
    Some(wallpapers)
}

/// Reescribe las claves conocidas preservando el resto del ini.
fn save_config(cfg: &BgConfig) {
    let settings_keys = [
        "folder",
        "backend",
        "monitors",
        "monitor_wallpapers",
        "fill",
        "sort",
        "sort_mode",
        "subfolders",
        "show_hidden",
        "zen_mode",
        "carousel_mode",
        "color",
        "wallpaper",
        "post_command",
    ];
    let settings_vals = [
        cfg.folder.clone(),
        cfg.backend.clone(),
        cfg.monitors.clone(),
        encode_monitor_wallpapers(&cfg.monitor_wallpapers),
        cfg.fill.clone(),
        cfg.sort.clone(),
        cfg.sort_mode.clone(),
        (if cfg.subfolders { "True" } else { "False" }).to_string(),
        (if cfg.show_hidden { "True" } else { "False" }).to_string(),
        (if cfg.zen_mode { "True" } else { "False" }).to_string(),
        (if cfg.carousel_mode { "True" } else { "False" }).to_string(),
        cfg.color.clone(),
        cfg.wallpaper.clone(),
        cfg.post_command.clone(),
    ];
    let wh_keys = ["api_key", "purity", "categories", "atleast"];
    let wh_vals = [
        cfg.wallhaven.api_key.clone(),
        cfg.wallhaven.purity.clone(),
        cfg.wallhaven.categories.clone(),
        cfg.wallhaven.atleast.clone(),
    ];

    let mut lines: Vec<String> = std::fs::read_to_string(ini_path())
        .map(|t| t.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default();

    let mut seen_settings = vec![false; settings_keys.len()];
    let mut seen_wh = vec![false; wh_keys.len()];
    let mut section = String::new();
    let mut settings_header: Option<usize> = None;
    let mut wh_header: Option<usize> = None;

    for (i, line) in lines.iter_mut().enumerate() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            section = t[1..t.len() - 1].trim().to_lowercase();
            if section == "settings" {
                settings_header = Some(i);
            } else if section == "wallhaven" {
                wh_header = Some(i);
            }
            continue;
        }
        let Some((k, _)) = t.split_once('=') else {
            continue;
        };
        let k = k.trim();
        if section.is_empty() || section == "settings" {
            if let Some(idx) = settings_keys.iter().position(|x| *x == k) {
                *line = format!("{} = {}", settings_keys[idx], settings_vals[idx]);
                seen_settings[idx] = true;
            }
        } else if section == "wallhaven" {
            if let Some(idx) = wh_keys.iter().position(|x| *x == k) {
                *line = format!("{} = {}", wh_keys[idx], wh_vals[idx]);
                seen_wh[idx] = true;
            }
        }
    }

    // Asegura [Settings].
    if settings_header.is_none() {
        let mut block = vec!["[Settings]".to_string()];
        for (i, k) in settings_keys.iter().enumerate() {
            if !seen_settings[i] {
                block.push(format!("{} = {}", k, settings_vals[i]));
                seen_settings[i] = true;
            }
        }
        block.push(String::new());
        let mut new_lines = block;
        new_lines.extend(lines);
        lines = new_lines;
        // recalcula header
        settings_header = Some(0);
    }

    // Inserta settings faltantes tras el header.
    let missing_settings: Vec<usize> = (0..settings_keys.len())
        .filter(|&i| !seen_settings[i])
        .collect();
    if !missing_settings.is_empty() {
        let hi = settings_header.unwrap();
        // insertar en orden, de atrás a delante
        for &i in missing_settings.iter().rev() {
            lines.insert(
                hi + 1,
                format!("{} = {}", settings_keys[i], settings_vals[i]),
            );
        }
    }

    // Asegura [wallhaven] si faltan claves.
    let missing_wh: Vec<usize> = (0..wh_keys.len()).filter(|&i| !seen_wh[i]).collect();
    if !missing_wh.is_empty() {
        if wh_header.is_none()
            && !lines
                .iter()
                .any(|l| l.trim().eq_ignore_ascii_case("[wallhaven]"))
        {
            lines.push("[wallhaven]".to_string());
        }
        if wh_header.is_none() {
            wh_header = lines
                .iter()
                .rposition(|l| l.trim().eq_ignore_ascii_case("[wallhaven]"));
        }
        if let Some(hi) = wh_header {
            for &i in missing_wh.iter().rev() {
                lines.insert(hi + 1, format!("{} = {}", wh_keys[i], wh_vals[i]));
            }
        } else {
            lines.push("[wallhaven]".to_string());
            for (i, k) in wh_keys.iter().enumerate() {
                if !seen_wh[i] {
                    lines.push(format!("{} = {}", k, wh_vals[i]));
                }
            }
        }
    }

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    let _ = std::fs::write(ini_path(), out);

    let state = format!("{}/.config/color-engine", home());
    let _ = std::fs::create_dir_all(&state);
    let _ = std::fs::write(
        format!("{state}/last-wallpaper"),
        format!("{}\n", cfg.wallpaper),
    );
}

fn is_image(name: &str) -> bool {
    matches!(
        std::path::Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "webp"
            | "bmp"
            | "gif"
            | "mp4"
            | "m4v"
            | "webm"
            | "mkv"
            | "mov"
            | "avi"
            | "ogv"
    )
}

fn is_video(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "mp4" | "m4v" | "webm" | "mkv" | "mov" | "avi" | "ogv"
    )
}

fn walk_images(folder: &str, recursive: bool, hidden: bool) -> Vec<String> {
    let mut v = Vec::new();
    let mut stack = vec![folder.to_string()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = p
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !hidden && name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                if recursive {
                    stack.push(p.to_string_lossy().into_owned());
                }
            } else if p.is_file() && is_image(&name) {
                v.push(p.to_string_lossy().into_owned());
            }
        }
    }
    v
}

fn sample_images(
    folder: &str,
    recursive: bool,
    hidden: bool,
    sort_mode: &str,
    n: usize,
) -> Vec<String> {
    if n == 0 {
        return Vec::new();
    }
    if sort_mode == "random" {
        // Reservoir sampling: O(N), memoria O(n).
        let mut reservoir: Vec<String> = Vec::with_capacity(n);
        let mut count: u64 = 0;
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0x9e3779b9)
            ^ (std::process::id() as u64);
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut stack = vec![folder.to_string()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in rd.flatten() {
                let p = e.path();
                let name = p
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !hidden && name.starts_with('.') {
                    continue;
                }
                if p.is_dir() {
                    if recursive {
                        stack.push(p.to_string_lossy().into_owned());
                    }
                } else if p.is_file() && is_image(&name) {
                    count += 1;
                    if reservoir.len() < n {
                        reservoir.push(p.to_string_lossy().into_owned());
                    } else {
                        let j = (rnd() % count) as usize;
                        if j < n {
                            reservoir[j] = p.to_string_lossy().into_owned();
                        }
                    }
                }
            }
        }
        let _ = count;
        reservoir
    } else {
        // Top-n por mtime/created con heap acotado (sin Vec completo).
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let mut heap: BinaryHeap<Reverse<(SystemTime, String)>> = BinaryHeap::new();
        let mut stack = vec![folder.to_string()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in rd.flatten() {
                let p = e.path();
                let name = p
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !hidden && name.starts_with('.') {
                    continue;
                }
                if p.is_dir() {
                    if recursive {
                        stack.push(p.to_string_lossy().into_owned());
                    }
                } else if p.is_file() && is_image(&name) {
                    let key = std::fs::metadata(&p).ok().and_then(|m| {
                        if sort_mode == "created" {
                            m.created().ok().or_else(|| m.modified().ok())
                        } else {
                            m.modified().ok()
                        }
                    });
                    if let Some(k) = key {
                        heap.push(Reverse((k, p.to_string_lossy().into_owned())));
                        if heap.len() > n {
                            heap.pop();
                        }
                    }
                }
            }
        }
        let mut v: Vec<_> = heap.into_sorted_vec();
        if sort_mode != "oldest" {
            v.reverse();
        }
        v.into_iter().map(|Reverse((_, p))| p).collect()
    }
}

fn order_images(mut v: Vec<String>, sort: &str) -> Vec<String> {
    if sort == "random" {
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0x9e3779b9)
            ^ (std::process::id() as u64);
        // xorshift64
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for i in (1..v.len()).rev() {
            let j = (rnd() as usize) % (i + 1);
            v.swap(i, j);
        }
    } else {
        v.sort();
    }
    v
}

/// Backends con binario presente. Si no hay ninguno, la UI lo dice y
/// no se puede aplicar.
fn available_backends() -> Vec<String> {
    BACKENDS
        .iter()
        .filter(|backend| {
            let command = match **backend {
                "wallutils" => "setwallpaper",
                "hyprpaper" => "hyprctl",
                command => command,
            };
            backend_bin(command).is_some()
        })
        .map(|s| s.to_string())
        .collect()
}

fn backend_bin(backend: &str) -> Option<String> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(backend))
        .find(|path| {
            use std::os::unix::fs::PermissionsExt;
            path.metadata().is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
        .map(|path| path.to_string_lossy().into_owned())
}

fn stop_persistent_wallpapers() {
    for name in [
        "swaybg",
        "mpvpaper",
        "gslapper",
        "linux-wallpaperengine",
        "swww-daemon",
        "awww-daemon",
    ] {
        let _ = std::process::Command::new("pkill")
            .args(["-x", name])
            .output();
    }
}

fn wallpaper_targets(cfg: &BgConfig, path: &str) -> Vec<(Option<String>, String)> {
    if cfg.monitor_wallpapers.is_empty() {
        return if cfg.monitors == "All" {
            vec![(None, path.to_string())]
        } else {
            vec![(Some(cfg.monitors.clone()), path.to_string())]
        };
    }

    let mut wallpapers = cfg.monitor_wallpapers.clone();
    for monitor in monitor_list()
        .into_iter()
        .filter(|monitor| monitor != "All")
    {
        wallpapers
            .entry(monitor)
            .or_insert_with(|| cfg.wallpaper.clone());
    }
    let mut targets: Vec<_> = wallpapers
        .into_iter()
        .filter(|(_, wallpaper)| !wallpaper.is_empty())
        .map(|(monitor, wallpaper)| (Some(monitor), wallpaper))
        .collect();
    targets.sort_by(|(left, _), (right, _)| left.cmp(right));
    targets
}

fn same_wallpaper_on_all_targets(targets: &[(Option<String>, String)]) -> Option<&str> {
    let (first_monitor, first_path) = targets.first()?;
    if first_monitor.is_none() || targets.iter().all(|(_, path)| path == first_path) {
        Some(first_path)
    } else {
        None
    }
}

fn run_checked(mut command: std::process::Command, label: &str) -> Result<(), String> {
    let output = command
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| format!("{label}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if detail.is_empty() {
            Err(format!("{label}: exited with {}", output.status))
        } else {
            Err(format!("{label}: {detail}"))
        }
    }
}

fn set_wallpaper(cfg: &BgConfig, path: &str) -> Result<(), String> {
    let targets = wallpaper_targets(cfg, path);
    if targets.is_empty() {
        return Err("No wallpapers are assigned to connected monitors".to_string());
    }
    if is_video(path) && !matches!(cfg.backend.as_str(), "mpvpaper" | "gslapper") {
        return Err(format!(
            "{} cannot play video files; choose mpvpaper or gslapper",
            cfg.backend
        ));
    }
    match cfg.backend.as_str() {
        "swww" | "awww" => {
            let bin = if cfg.backend == "awww" {
                "awww"
            } else {
                "swww"
            };
            let daemon = format!("{bin}-daemon");
            if backend_bin(&daemon).is_none() && backend_bin(bin).is_none() {
                return Err(format!("{bin} is not installed"));
            }
            stop_persistent_wallpapers();
            if std::process::Command::new("pgrep")
                .args(["-x", &daemon])
                .output()
                .is_ok_and(|o| o.stdout.is_empty())
            {
                std::process::Command::new("setsid")
                    .arg(&daemon)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|error| format!("start {daemon}: {error}"))?;
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
            for (monitor, wallpaper) in &targets {
                let mut cmd = std::process::Command::new(bin);
                cmd.args(["img", wallpaper]);
                if let Some(monitor) = monitor {
                    cmd.args(["-o", monitor]);
                }
                run_checked(cmd, bin)?;
            }
        }
        "wallutils" => {
            if backend_bin("setwallpaper").is_none() {
                return Err("setwallpaper is not installed".to_string());
            }
            let wallpaper = same_wallpaper_on_all_targets(&targets).ok_or_else(|| {
                "wallutils cannot apply different wallpapers per monitor".to_string()
            })?;
            stop_persistent_wallpapers();
            let mut command = std::process::Command::new("setwallpaper");
            command.arg(wallpaper);
            run_checked(command, "wallutils")?;
        }
        "hyprpaper" => {
            if backend_bin("hyprctl").is_none() {
                return Err("hyprctl is not installed".to_string());
            }
            stop_persistent_wallpapers();
            for (monitor, wallpaper) in &targets {
                let mut preload = std::process::Command::new("hyprctl");
                preload.args(["hyprpaper", "preload", wallpaper]);
                run_checked(preload, "hyprpaper preload")?;
                let target = monitor
                    .as_ref()
                    .map(|monitor| format!("{monitor},{wallpaper}"))
                    .unwrap_or_else(|| wallpaper.clone());
                let mut apply = std::process::Command::new("hyprctl");
                apply.args(["hyprpaper", "wallpaper", &target]);
                run_checked(apply, "hyprpaper wallpaper")?;
            }
        }
        "mpvpaper" => {
            if backend_bin("mpvpaper").is_none() {
                return Err("mpvpaper is not installed".to_string());
            }
            stop_persistent_wallpapers();
            for (monitor, wallpaper) in &targets {
                let output = monitor.as_deref().unwrap_or("*");
                std::process::Command::new("setsid")
                    .args([
                        "--fork",
                        "mpvpaper",
                        "-o",
                        "no-audio loop hwdec=auto",
                        output,
                        wallpaper,
                    ])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|error| format!("start mpvpaper: {error}"))?;
            }
        }
        "gslapper" => {
            if backend_bin("gslapper").is_none() {
                return Err("gslapper is not installed".to_string());
            }
            stop_persistent_wallpapers();
            for (monitor, wallpaper) in &targets {
                let output = monitor.as_deref().unwrap_or("*");
                std::process::Command::new("setsid")
                    .args(["--fork", "gslapper", output, wallpaper])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|error| format!("start gslapper: {error}"))?;
            }
        }
        "linux-wallpaperengine" => {
            if backend_bin("linux-wallpaperengine").is_none() {
                return Err("linux-wallpaperengine is not installed".to_string());
            }
            let wallpaper = same_wallpaper_on_all_targets(&targets).ok_or_else(|| {
                "linux-wallpaperengine cannot apply different wallpapers per monitor".to_string()
            })?;
            stop_persistent_wallpapers();
            std::process::Command::new("setsid")
                .args(["--fork", "linux-wallpaperengine", wallpaper])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|error| format!("start linux-wallpaperengine: {error}"))?;
        }
        "feh" => {
            if backend_bin("feh").is_none() {
                return Err("feh is not installed".to_string());
            }
            let wallpaper = same_wallpaper_on_all_targets(&targets).ok_or_else(|| {
                "feh cannot apply different wallpapers per monitor on Wayland".to_string()
            })?;
            stop_persistent_wallpapers();
            let mode = match cfg.fill.as_str() {
                "tile" => "--bg-tile",
                "center" => "--bg-center",
                "stretch" => "--bg-scale",
                "fit" => "--bg-max",
                _ => "--bg-fill",
            };
            let mut command = std::process::Command::new("feh");
            command.args([mode, wallpaper]);
            run_checked(command, "feh")?;
        }
        "xwallpaper" => {
            if backend_bin("xwallpaper").is_none() {
                return Err("xwallpaper is not installed".to_string());
            }
            let wallpaper = same_wallpaper_on_all_targets(&targets).ok_or_else(|| {
                "xwallpaper cannot apply different wallpapers per monitor on Wayland".to_string()
            })?;
            stop_persistent_wallpapers();
            let mode = match cfg.fill.as_str() {
                "tile" => "--tile",
                "center" => "--center",
                "stretch" => "--stretch",
                _ => "--zoom",
            };
            let mut command = std::process::Command::new("xwallpaper");
            command.args([mode, wallpaper]);
            run_checked(command, "xwallpaper")?;
        }
        _ => {
            if backend_bin("swaybg").is_none() {
                return Err("swaybg is not installed".to_string());
            }
            stop_persistent_wallpapers();
            for (monitor, wallpaper) in &targets {
                let mut cmd = std::process::Command::new("setsid");
                cmd.arg("swaybg");
                if let Some(monitor) = monitor {
                    cmd.args(["-o", monitor]);
                }
                cmd.args(["-i", wallpaper, "-m", &cfg.fill, "-c", &cfg.color]);
                cmd.stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|error| format!("start swaybg: {error}"))?;
            }
        }
    }
    Ok(())
}

/// Pone el fondo, lo persiste y lanza el `post_command` externo si existe.
/// El color lo aplica `theme_engine` aparte (así cambiar ajuste/monitor no
/// recalcula la paleta).
fn record_wallpaper(cfg: &mut BgConfig, path: &str, monitors: &[String]) {
    record_wallpaper_assignment(
        &cfg.monitors,
        &mut cfg.wallpaper,
        &mut cfg.monitor_wallpapers,
        monitors,
        path,
    );
}

fn record_wallpaper_assignment(
    selected_monitor: &str,
    wallpaper: &mut String,
    assignments: &mut HashMap<String, String>,
    monitors: &[String],
    path: &str,
) {
    if selected_monitor == "All" {
        *wallpaper = path.to_string();
        assignments.clear();
    } else {
        if assignments.is_empty() && !wallpaper.is_empty() {
            for monitor in monitors.iter().filter(|monitor| monitor.as_str() != "All") {
                assignments.insert(monitor.clone(), wallpaper.clone());
            }
        }
        assignments.insert(selected_monitor.to_string(), path.to_string());
    }
}

fn can_load_more(page: u32, last_page: u32, loading: bool) -> bool {
    !loading && page < last_page
}

fn apply_wallpaper(cfg: &mut BgConfig, path: &str, no_post: bool) -> Result<(), String> {
    let mut next_cfg = cfg.clone();
    record_wallpaper(&mut next_cfg, path, &monitor_list());
    set_wallpaper(&next_cfg, path)?;
    if !no_post && !next_cfg.post_command.is_empty() {
        let cmd = next_cfg
            .post_command
            .replace("$wallpaper", path)
            .replace("$monitor", &next_cfg.monitors);
        let _ = std::process::Command::new("bash")
            .args(["-c", &cmd])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
    save_config(&next_cfg);
    *cfg = next_cfg;
    Ok(())
}

/// Reaplica el fondo actual (sin tocar la paleta) tras cambiar
/// motor de fondos, monitor o ajuste.
fn reapply_wallpaper(ui: &Ui) {
    let cfg = ui.config.borrow().clone();
    let wallpaper = selected_wallpaper(&cfg).to_string();
    if wallpaper.is_empty() || !Path::new(&wallpaper).is_file() {
        return;
    }
    if let Err(e) = set_wallpaper(&cfg, &wallpaper) {
        set_status(ui, &format!("Error applying wallpaper: {e}"));
    }
}

fn pick_random(images: &[String], exclude: &str) -> Option<String> {
    let pool: Vec<&String> = images.iter().filter(|p| p.as_str() != exclude).collect();
    if pool.is_empty() {
        return images.first().cloned();
    }
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0)
        ^ (std::process::id() as usize);
    pool.get(seed % pool.len()).map(|s| (*s).clone())
}

fn pick_next(images: &[String], current: &str) -> Option<String> {
    if images.is_empty() {
        return None;
    }
    let i = images.iter().position(|p| p == current).map(|i| i + 1);
    match i {
        Some(n) if n < images.len() => Some(images[n].clone()),
        _ => images.first().cloned(),
    }
}

fn cache_path(path: &str) -> Option<String> {
    let md = std::fs::metadata(path).ok()?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    mtime.hash(&mut h);
    md.len().hash(&mut h);
    THUMB_PX.hash(&mut h);
    Some(format!(
        "{}/.cache/aspire-bg/{:016x}.{}",
        home(),
        h.finish(),
        if is_video(path) { "gif" } else { "png" }
    ))
}

fn load_cached_thumb(path: &str) -> Option<gtk::gdk::Texture> {
    let cp = cache_path(path)?;
    if !std::path::Path::new(&cp).is_file() {
        return None;
    }
    let px = gdk_pixbuf::Pixbuf::from_file(&cp).ok()?;
    Some(gtk::gdk::Texture::for_pixbuf(&px))
}

/// Guarda en caché como PNG (el JPEG no admite alfa y fallaba en esos).
fn save_png_cache(path: &str, px: &gdk_pixbuf::Pixbuf) -> Option<()> {
    let cp = cache_path(path)?;
    if std::path::Path::new(&cp).is_file() {
        return Some(());
    }
    let _ = std::fs::create_dir_all(format!("{}/.cache/aspire-bg", home()));
    px.savev(&cp, "png", &[]).ok()?;
    Some(())
}

fn decode_pixbuf(bytes: &[u8]) -> Option<gdk_pixbuf::Pixbuf> {
    let loader = gdk_pixbuf::PixbufLoader::new();
    loader.write(bytes).ok()?;
    loader.close().ok()?;
    let px = loader.pixbuf()?;
    let (w, h) = (px.width() as f64, px.height() as f64);
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let scale = (THUMB_PX as f64 / w).min(THUMB_PX as f64 / h).min(1.0);
    px.scale_simple(
        (w * scale) as i32,
        (h * scale) as i32,
        gdk_pixbuf::InterpType::Bilinear,
    )
}

fn basename(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

struct State {
    all: Vec<String>,
    all_loaded: bool,
    shown: Vec<String>,
    base_shown: Vec<String>,
    current: usize,
    thumbs: HashMap<String, gtk::gdk::Texture>,
    pictures: HashMap<String, Picture>,
    video_previews: HashMap<String, gdk_pixbuf::PixbufAnimation>,
    video_preview_src: Option<glib::SourceId>,
    thumb_gen: u64,
    search_gen: u64,
    search_pending: bool,
    search_rx: Option<std::sync::mpsc::Receiver<SearchMsg>>,
    search_drainer_active: bool,
    sample_gen: u64,
    sample_pending: bool,
    sample_rx: Option<std::sync::mpsc::Receiver<SampleMsg>>,
    sample_drainer_active: bool,
    index_ready: bool,
}

enum SampleMsg {
    Sample { gen: u64, shown: Vec<String> },
}

enum SearchMsg {
    Result {
        gen: u64,
        all: Option<Vec<String>>,
        index: Option<Vec<(String, String)>>,
        shown: Vec<String>,
    },
}

#[cfg(feature = "wallhaven")]
#[derive(Clone)]
enum WhEvent {
    Results {
        gen: u64,
        page: u32,
        last_page: u32,
        items: Vec<wallhaven::WhItem>,
        seed: Option<String>,
    },
    Thumb {
        gen: u64,
        id: String,
        width: i32,
        height: i32,
        stride: usize,
        bytes: Vec<u8>,
    },
    Error {
        gen: u64,
        msg: String,
    },
    DownloadDone {
        path: String,
    },
    DownloadError {
        msg: String,
    },
}

#[cfg(feature = "wallhaven")]
struct WhState {
    categories: String,
    sort: String,
    purity: String,
    seed: Option<String>,
    page: u32,
    last_page: u32,
    items: Vec<wallhaven::WhItem>,
    seen: HashSet<String>,
    downloading: HashSet<String>,
    thumbs: HashMap<String, gtk::gdk::Texture>,
    pictures: HashMap<String, Picture>,
    gen: u64,
    pending: Arc<AtomicUsize>,
    tx: Option<std::sync::mpsc::Sender<WhEvent>>,
    rx: Option<std::sync::mpsc::Receiver<WhEvent>>,
    drainer_active: bool,
    search_src: Rc<RefCell<Option<glib::SourceId>>>,
    opened: bool,
    loading_page: bool,
}

#[derive(Clone)]
struct Ui {
    window: ApplicationWindow,
    card: GtkBox,
    bar: GtkBox,
    provider: CssProvider,
    grid: FlowBox,
    scroll: ScrolledWindow,
    local_display: gtk::Stack,
    carousel_picture: Picture,
    carousel_side_pictures: Vec<Picture>,
    view_toggle: Button,
    wallhaven_grid: Rc<RefCell<Option<FlowBox>>>,
    wallhaven_scroll: Rc<RefCell<Option<ScrolledWindow>>>,
    wallhaven_index: Rc<Cell<usize>>,
    backend_drop: DropDown,
    monitor_drop: DropDown,
    fill_drop: DropDown,
    sort_drop: DropDown,
    sub_check: CheckButton,
    hidden_check: CheckButton,
    state: Rc<RefCell<State>>,
    config: Rc<RefCell<BgConfig>>,
    /// Si viene de `--create-tinty-theme`: sistema base16/base24.
    theme_system: Option<String>,
    busy: Arc<AtomicBool>,
    search_index: Arc<Mutex<Arc<Vec<(String, String)>>>>,
    index_ready: Arc<AtomicBool>,
    index_building: Arc<AtomicBool>,
    pending_query: Arc<Mutex<Option<String>>>,
    view_stack: Rc<RefCell<Option<gtk::Stack>>>,
    toolbar_stack: Rc<RefCell<Option<gtk::Stack>>>,
}

struct BgAppInit {
    config: BgConfig,
    images: Vec<String>,
    theme_system: Option<String>,
}

#[derive(Debug)]
enum BgMsg {
    Close,
    ApplyCurrent,
    ApplyRandom,
    Refresh,
    ChooseFolder,
    MoveSelection { delta: isize, by_row: bool },
    ToggleCarousel,
}

struct BgApp {
    ui: Ui,
}

impl SimpleComponent for BgApp {
    type Init = BgAppInit;
    type Input = BgMsg;
    type Output = ();
    type Widgets = ();
    type Root = ApplicationWindow;

    fn init_root() -> Self::Root {
        ApplicationWindow::builder()
            .title("Aspire Background")
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let ui = build_ui(root, init.config, init.images, init.theme_system, sender);
        ComponentParts {
            model: Self { ui },
            widgets: (),
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            BgMsg::Close => self.ui.window.close(),
            BgMsg::ApplyCurrent => apply_current(&self.ui),
            BgMsg::ApplyRandom => apply_random(&self.ui),
            BgMsg::Refresh => refresh(&self.ui),
            BgMsg::ChooseFolder => choose_folder(&self.ui),
            BgMsg::MoveSelection { delta, by_row } => {
                move_selection(&self.ui, delta, by_row);
            }
            BgMsg::ToggleCarousel => {
                let carousel =
                    self.ui.local_display.visible_child_name().as_deref() != Some("carousel");
                set_carousel_mode(&self.ui, carousel);
            }
        }
    }
}

fn hex(c: Option<RgbColor>, fallback: &str) -> String {
    c.map(|v| format!("#{:02x}{:02x}{:02x}", v.r, v.g, v.b))
        .unwrap_or_else(|| fallback.to_string())
}

fn set_status(_ui: &Ui, text: &str) {
    if text.starts_with("Error") {
        eprintln!("aspire-bg: {text}");
        notify("Wallpaper error", text, true);
    }
}

fn notify(title: &str, message: &str, critical: bool) {
    let mut command = std::process::Command::new("notify-send");
    command.args(["--app-name=Aspire Background"]);
    if critical {
        command.args(["--urgency=critical"]);
    }
    match command.args([title, message]).output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => eprintln!(
            "aspire-bg: notify-send failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => eprintln!("aspire-bg: cannot run notify-send: {error}"),
    }
}

fn notify_wallpaper_changed() {
    notify(
        "Wallpaper changed",
        "Your wallpaper has been updated",
        false,
    );
}

fn reload_css(ui: &Ui) {
    ui.provider.load_from_string(&build_css());
}

/// Ejecuta el trabajo de color en un hilo y refleja el resultado en la barra.
fn run_color_job<F>(ui: &Ui, job: F, quit_on_ok: bool)
where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    if ui.busy.swap(true, Ordering::AcqRel) {
        return;
    }
    set_status(ui, "Applying colors...");
    let slot: Arc<Mutex<Option<Result<String, String>>>> = Arc::new(Mutex::new(None));
    let slot_thread = slot.clone();
    std::thread::spawn(move || {
        let result = job();
        if let Ok(mut guard) = slot_thread.lock() {
            *guard = Some(result);
        }
    });
    let ui_t = ui.clone();
    glib::timeout_add_local(Duration::from_millis(120), move || {
        let result = slot.lock().ok().and_then(|mut guard| guard.take());
        match result {
            Some(Ok(message)) => {
                ui_t.busy.store(false, Ordering::Release);
                set_status(&ui_t, &message);
                reload_css(&ui_t);
                if quit_on_ok {
                    ui_t.window.close();
                }
                glib::ControlFlow::Break
            }
            Some(Err(error)) => {
                ui_t.busy.store(false, Ordering::Release);
                set_status(&ui_t, &format!("Error: {error}"));
                glib::ControlFlow::Break
            }
            None => glib::ControlFlow::Continue,
        }
    });
}

fn selected_wallpaper(cfg: &BgConfig) -> &str {
    if cfg.monitors == "All" {
        &cfg.wallpaper
    } else {
        cfg.monitor_wallpapers
            .get(&cfg.monitors)
            .map(String::as_str)
            .unwrap_or(&cfg.wallpaper)
    }
}

/// Reacciona a un cambio de fondo: matugen recolorea, tinty no toca nada.
/// En modo "crear tema tinty" genera el esquema y lo aplica.
fn start_color_job(ui: &Ui, path: String, quit_on_ok: bool) {
    if let Some(system) = ui.theme_system.clone() {
        run_color_job(
            ui,
            move || {
                let scheme = theme_engine::create_tinty_scheme(Path::new(&path), &system)
                    .map_err(|e| format!("crear tema tinty: {e:#}"))?;
                theme_engine::apply_tinty_scheme(&scheme)
                    .map_err(|e| format!("apply Tinty theme: {e:#}"))?;
                theme_engine::set_active_engine(Engine::Tinty)
                    .map_err(|e| format!("activar tinty: {e:#}"))?;
                Ok(format!("Theme created and applied: {scheme}"))
            },
            true,
        );
        return;
    }
    match theme_engine::active_engine() {
        Engine::Matugen => {
            run_color_job(
                ui,
                move || {
                    theme_engine::apply_matugen(Path::new(&path))
                        .map(|_| "Colors updated".to_string())
                        .map_err(|e| format!("{e:#}"))
                },
                false,
            );
        }
        Engine::Tinty => set_status(ui, "Wallpaper applied · Tinty keeps the current colors"),
    }
}

fn build_css() -> String {
    let colors = colors::load_theme_colors();
    let bg = hex(colors.background, "#1e1e2e");
    let alt = hex(colors.background_alt, "#313244");
    let fg = hex(colors.foreground, "#cdd6f4");
    let sel = hex(colors.selected, "#cba6f7");
    let tile_border = TILE_BORDER_PX;
    let carousel_border = CAROUSEL_BORDER_PX;
    format!(
        r#"
.bg-window {{ background-color: transparent; border: none; font-family: "JetBrainsMono Nerd Font"; }}
.bg-card {{ background-color: {bg}; border: 1px solid {sel}; border-radius: 0; }}
.bg-card.bg-carousel-mode {{ background-color: transparent; border: none; }}
.bg-bar {{ min-height: 34px; border-bottom: 1px solid {alt}; padding: 1px 14px; background: transparent; }}
.bg-entry {{ background: transparent; border: none; box-shadow: none; outline: none; color: {fg}; font-size: 16px; }}
.bg-entry placeholder {{ color: {fg}; opacity: 0.9; }}
.bg-btn {{ background: {alt}; border: 1px solid {alt}; border-radius: 0; color: {fg}; padding: 1px 10px; }}
menubutton.bg-options-button {{ background: transparent; border: none; box-shadow: none; padding: 0; }}
menubutton.bg-options-button > button {{ background: {alt}; border: 1px solid {sel}; border-radius: 0; color: {fg}; padding: 1px 10px; box-shadow: none; }}
menubutton.bg-options-button > button:hover {{ background: {sel}; border-color: {sel}; color: {bg}; }}
.bg-toolbar-btn {{ background: {alt}; border: 1px solid {sel}; border-radius: 0; color: {fg}; padding: 1px 10px; }}
.bg-toolbar-btn:hover {{ background: {sel}; border-color: {sel}; color: {bg}; }}
.bg-btn:hover {{ border-color: {sel}; }}
.bg-flat-button {{ background: transparent; border: none; box-shadow: none; color: {sel}; padding: 2px 6px; font-size: 12px; }}
.bg-flat-button:hover {{ background: {alt}; }}
.bg-engine-indicator {{ color: {sel}; background: transparent; font-size: 11px; padding: 3px 8px; }}
.bg-status {{ color: {fg}; font-size: 12px; opacity: 0.75; }}
.bg-engine {{ color: {sel}; font-size: 11px; padding: 2px 8px; border: 1px solid {alt}; }}
.bg-opt {{ background: transparent; border-bottom: 1px solid {alt}; padding: 6px 20px; }}
.bg-optlabel {{ color: {fg}; font-size: 12px; opacity: 0.7; }}
.bg-grid {{ background: transparent; padding: 12px; }}
.bg-tile {{ background: transparent; border: {tile_border}px solid transparent; border-radius: 0; padding: 4px; }}
.bg-grid child:selected .bg-tile, .bg-grid flowboxchild:focus .bg-tile {{ background: {alt}; border-color: {sel}; }}
.bg-current {{ background: {alt}; border-color: {sel}; }}
.bg-thumb {{ background: transparent; }}
.bg-carousel-side {{ opacity: 0.68; }}
.bg-carousel-center {{ border: {carousel_border}px solid {sel}; }}
"#
    )
}

fn make_tile(tex: Option<&gtk::gdk::Texture>, is_current: bool) -> (GtkBox, Picture) {
    let tile = GtkBox::new(Orientation::Vertical, 0);
    tile.add_css_class("bg-tile");
    if is_current {
        tile.add_css_class("bg-current");
    }
    let picture = Picture::new();
    picture.set_content_fit(ContentFit::Cover);
    picture.set_size_request(TILE_W, TILE_H);
    picture.set_hexpand(true);
    picture.add_css_class("bg-thumb");
    if let Some(t) = tex {
        picture.set_paintable(Some(t));
    }
    tile.append(&picture);
    (tile, picture)
}

fn rebuild_grid(ui: &Ui) {
    while let Some(child) = ui.grid.first_child() {
        ui.grid.remove(&child);
    }
    // Sin motores instalados se pide instalar uno en vez de cuadrícula.
    if available_backends().is_empty() {
        let label = Label::new(Some("No wallpaper engine found. Install swaybg."));
        label.add_css_class("bg-name");
        label.set_margin_top(40);
        label.set_margin_bottom(40);
        ui.grid.append(&label);
        return;
    }
    let (shown, thumbs, wallpaper) = {
        let st = ui.state.borrow();
        (
            st.shown.clone(),
            st.thumbs.clone(),
            selected_wallpaper(&ui.config.borrow()).to_string(),
        )
    };
    {
        let mut st = ui.state.borrow_mut();
        st.pictures.clear();
        st.video_previews.clear();
        for path in &shown {
            let (tile, picture) = make_tile(thumbs.get(path), path == &wallpaper);
            tile.set_hexpand(true);
            ui.grid.append(&tile);
            st.pictures.insert(path.clone(), picture);
        }
        if st.current >= shown.len() {
            st.current = 0;
        }
    }
    let current_index = ui.state.borrow().current as i32;
    if let Some(child) = ui.grid.child_at_index(current_index) {
        ui.grid.select_child(&child);
        scroll_to_child(ui, &child);
    }
    update_video_preview(ui);
}

fn update_video_preview(ui: &Ui) {
    if let Some(source) = ui.state.borrow_mut().video_preview_src.take() {
        source.remove();
    }
    let (current, shown, thumbs, animation, picture, thumb) = {
        let state = ui.state.borrow();
        let current = state.shown.get(state.current).cloned();
        let shown = state.shown.clone();
        let thumbs = state.thumbs.clone();
        let animation = current
            .as_ref()
            .and_then(|path| state.video_previews.get(path))
            .cloned();
        let picture = current
            .as_ref()
            .and_then(|path| state.pictures.get(path))
            .cloned();
        let thumb = current
            .as_ref()
            .and_then(|path| state.thumbs.get(path))
            .cloned();
        (current, shown, thumbs, animation, picture, thumb)
    };
    let Some(path) = current else {
        ui.carousel_picture
            .set_paintable(None::<&gtk::gdk::Paintable>);
        for side in &ui.carousel_side_pictures {
            side.set_paintable(None::<&gtk::gdk::Paintable>);
        }
        return;
    };
    let current_index = ui.state.borrow().current;
    for (side, offset) in ui.carousel_side_pictures.iter().zip([-2_isize, -1, 1, 2]) {
        let index = (current_index as isize + offset).rem_euclid(shown.len() as isize) as usize;
        let texture = shown.get(index).and_then(|side_path| thumbs.get(side_path));
        if let Some(texture) = texture {
            side.set_paintable(Some(texture));
        } else {
            side.set_paintable(None::<&gtk::gdk::Paintable>);
        }
    }
    if let Some(texture) = thumb.as_ref() {
        ui.carousel_picture.set_paintable(Some(texture));
    } else {
        ui.carousel_picture
            .set_paintable(None::<&gtk::gdk::Paintable>);
    }
    let (Some(animation), Some(picture)) = (animation, picture) else {
        return;
    };
    if !is_video(&path) {
        return;
    }
    let iter = animation.iter(Some(SystemTime::now()));
    let first_frame = gtk::gdk::Texture::for_pixbuf(&iter.pixbuf());
    picture.set_paintable(Some(&first_frame));
    ui.carousel_picture.set_paintable(Some(&first_frame));
    let tick_iter = iter.clone();
    let tick_picture = picture.clone();
    let tick_carousel = ui.carousel_picture.clone();
    let tick_ui = ui.clone();
    let source = glib::timeout_add_local(Duration::from_millis(40), move || {
        let still_selected = {
            let state = tick_ui.state.borrow();
            state.shown.get(state.current).is_some_and(|p| p == &path)
        };
        if !still_selected {
            return glib::ControlFlow::Break;
        }
        tick_iter.advance(SystemTime::now());
        let frame = gtk::gdk::Texture::for_pixbuf(&tick_iter.pixbuf());
        tick_picture.set_paintable(Some(&frame));
        tick_carousel.set_paintable(Some(&frame));
        glib::ControlFlow::Continue
    });
    ui.state.borrow_mut().video_preview_src = Some(source);
}

#[allow(deprecated)]
fn scroll_to_child(ui: &Ui, child: &gtk::FlowBoxChild) {
    let alloc = child.allocation();
    let view_h = ui.scroll.allocation().height();
    if alloc.height() <= 0 || view_h <= 0 {
        return;
    }
    let adj = ui.scroll.vadjustment();
    let target = (alloc.y() + alloc.height() / 2 - view_h / 2) as f64;
    let max = (adj.upper() - adj.page_size()).max(0.0);
    adj.set_value(target.clamp(0.0, max));
}

#[allow(deprecated)]
fn move_selection(ui: &Ui, delta: isize, by_row: bool) {
    const GRID_COLUMNS: isize = 3;
    let step = if by_row { delta * GRID_COLUMNS } else { delta };
    let next = {
        let mut st = ui.state.borrow_mut();
        let n = st.shown.len() as isize;
        if n == 0 {
            return;
        }
        st.current = (st.current as isize + step).rem_euclid(n) as usize;
        st.current as i32
    };
    if let Some(child) = ui.grid.child_at_index(next) {
        ui.grid.select_child(&child);
        scroll_to_child(ui, &child);
    }
}

fn apply_current(ui: &Ui) {
    let path = {
        let st = ui.state.borrow();
        match st.shown.get(st.current).cloned() {
            Some(p) => p,
            None => return,
        }
    };
    let was_carousel = ui.local_display.visible_child_name().as_deref() == Some("carousel");
    if let Err(e) = apply_wallpaper(&mut ui.config.borrow_mut(), &path, false) {
        set_status(ui, &format!("Error applying wallpaper: {e}"));
        return;
    }
    if was_carousel {
        // En carousel, al aplicar un fondo se cierra la app (no se vuelve a la grilla).
        notify_wallpaper_changed();
        if !is_video(&path) && theme_engine::active_engine() == theme_engine::Engine::Matugen {
            // Solo matugen recolorea al cambiar de fondo; tinty conserva su paleta.
            // Se aplica en un proceso aparte para no bloquear el cierre.
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe)
                    .arg("--apply-colors")
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }
        }
        ui.window.close();
        return;
    }
    notify_wallpaper_changed();
    rebuild_grid(ui);
    if !is_video(&path) {
        start_color_job(ui, path, false);
    }
}

fn apply_random(ui: &Ui) {
    let (images, wallpaper) = {
        let st = ui.state.borrow();
        (
            st.shown.clone(),
            selected_wallpaper(&ui.config.borrow()).to_string(),
        )
    };
    if let Some(path) = pick_random(&images, &wallpaper) {
        let was_carousel = ui.local_display.visible_child_name().as_deref() == Some("carousel");
        if let Err(e) = apply_wallpaper(&mut ui.config.borrow_mut(), &path, false) {
            set_status(ui, &format!("Error applying wallpaper: {e}"));
            return;
        }
        if was_carousel {
            notify_wallpaper_changed();
            if !is_video(&path) && theme_engine::active_engine() == theme_engine::Engine::Matugen {
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe)
                        .arg("--apply-colors")
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();
                }
            }
            ui.window.close();
            return;
        }
        notify_wallpaper_changed();
        rebuild_grid(ui);
        if !is_video(&path) {
            start_color_job(ui, path, false);
        }
    } else if ui.state.borrow().sample_pending {
        notify(
            "Wallpapers loading",
            "Please wait for the wallpaper list to load.",
            false,
        );
    } else {
        set_status(ui, "No wallpapers available");
    }
}

fn pick_batch(all: &[String], n: usize) -> Vec<String> {
    if all.is_empty() || n == 0 {
        return Vec::new();
    }
    let mut idx: Vec<usize> = (0..all.len()).collect();
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0x9e3779b9)
        ^ (std::process::id() as u64);
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for i in (1..idx.len()).rev() {
        let j = (rnd() as usize) % (i + 1);
        idx.swap(i, j);
    }
    idx.truncate(n.min(idx.len()));
    idx.iter().map(|&i| all[i].clone()).collect()
}

fn launch_sample(ui: &Ui) {
    let cfg = ui.config.borrow().clone();
    let gen = {
        let mut st = ui.state.borrow_mut();
        st.sample_gen = st.sample_gen.wrapping_add(1);
        st.sample_gen
    };
    let (tx, rx) = std::sync::mpsc::channel();
    {
        let mut st = ui.state.borrow_mut();
        st.sample_rx = Some(rx);
        st.sample_pending = true;
        st.sample_drainer_active = false;
    }
    let folder = cfg.folder.clone();
    let recursive = cfg.subfolders;
    let hidden = cfg.show_hidden;
    let sort_mode = cfg.sort_mode.clone();
    std::thread::spawn(move || {
        let shown = sample_images(&folder, recursive, hidden, &sort_mode, 30);
        let _ = tx.send(SampleMsg::Sample { gen, shown });
    });
    let ui2 = ui.clone();
    if !ui.state.borrow().sample_drainer_active {
        ui.state.borrow_mut().sample_drainer_active = true;
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            let msg = {
                let st = ui2.state.borrow();
                st.sample_rx.as_ref().and_then(|rx| rx.try_recv().ok())
            };
            if let Some(SampleMsg::Sample { gen, shown }) = msg {
                let cur = ui2.state.borrow().sample_gen;
                if gen == cur {
                    {
                        let mut st = ui2.state.borrow_mut();
                        st.shown = shown.clone();
                        st.base_shown = shown.clone();
                        st.current = 0;
                        st.thumbs.clear();
                        st.pictures.clear();
                        st.video_previews.clear();
                        st.thumb_gen = st.thumb_gen.wrapping_add(1);
                    }
                    rebuild_grid(&ui2);
                    let g = ui2.state.borrow().thumb_gen;
                    let sh = ui2.state.borrow().shown.clone();
                    load_thumbs(&ui2, sh, g);
                }
                ui2.state.borrow_mut().sample_pending = false;
            }
            if ui2.state.borrow().sample_pending {
                glib::ControlFlow::Continue
            } else {
                ui2.state.borrow_mut().sample_rx = None;
                ui2.state.borrow_mut().sample_drainer_active = false;
                glib::ControlFlow::Break
            }
        });
    }
}

fn resample(ui: &Ui) {
    {
        let mut st = ui.state.borrow_mut();
        st.all.clear();
        st.all_loaded = false;
        st.index_ready = false;
        st.shown.clear();
        st.base_shown.clear();
        st.thumbs.clear();
        st.pictures.clear();
        st.video_previews.clear();
        st.current = 0;
        st.thumb_gen = st.thumb_gen.wrapping_add(1);
        st.sample_gen = st.sample_gen.wrapping_add(1);
        st.sample_pending = true;
    }
    *ui.search_index.lock().unwrap() = Arc::new(Vec::new());
    ui.index_ready
        .store(false, std::sync::atomic::Ordering::Release);
    rebuild_grid(ui);
    launch_sample(ui);
}

fn rescan(ui: &Ui) {
    resample(ui);
}

fn refresh(ui: &Ui) {
    // Re-read folder and re-apply current mode (new random 30 if Random)
    resample(ui);
}

fn load_thumbs(ui: &Ui, paths: Vec<String>, gen: u64) {
    // Hilo efímero: solo I/O de disco. La decodificación y la UI ocurren
    // en el hilo principal, y solo si la generación sigue vigente.
    let queue: Arc<Mutex<VecDeque<(String, Vec<u8>)>>> = Arc::new(Mutex::new(VecDeque::new()));
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let q = queue.clone();
        let d = done.clone();
        std::thread::spawn(move || {
            for path in paths {
                let cached = cache_path(&path);
                if cached
                    .as_ref()
                    .is_some_and(|cp| std::path::Path::new(cp).is_file())
                {
                    q.lock().unwrap().push_back((path, Vec::new()));
                } else if is_video(&path) {
                    let Some(thumb_path) = cached else {
                        eprintln!("aspire-bg: cannot create video preview path for {path}");
                        continue;
                    };
                    if let Some(parent) = Path::new(&thumb_path).parent() {
                        if let Err(error) = std::fs::create_dir_all(parent) {
                            eprintln!("aspire-bg: cannot create video preview cache: {error}");
                            continue;
                        }
                    }
                    let scale = format!(
                        "fps=8,scale={THUMB_PX}:{THUMB_PX}:force_original_aspect_ratio=decrease"
                    );
                    let result = std::process::Command::new("ffmpeg")
                        .args([
                            "-hide_banner",
                            "-loglevel",
                            "error",
                            "-ss",
                            "0.5",
                            "-i",
                            &path,
                            "-t",
                            "1.5",
                            "-an",
                            "-vf",
                            &scale,
                            "-loop",
                            "0",
                            "-y",
                            &thumb_path,
                        ])
                        .output();
                    match result {
                        Ok(output) if output.status.success() => {
                            q.lock().unwrap().push_back((path, Vec::new()));
                        }
                        Ok(output) => {
                            let detail = String::from_utf8_lossy(&output.stderr);
                            eprintln!(
                                "aspire-bg: video preview failed for {path}: {}",
                                detail.trim()
                            );
                        }
                        Err(error) => {
                            eprintln!("aspire-bg: cannot run ffmpeg for {path}: {error}");
                        }
                    }
                } else if let Ok(bytes) = std::fs::read(&path) {
                    q.lock().unwrap().push_back((path, bytes));
                }
            }
            d.store(true, std::sync::atomic::Ordering::Release);
        });
    }
    let ui_t = ui.clone();
    let q2 = queue.clone();
    let d2 = done.clone();
    glib::idle_add_local(move || {
        if ui_t.state.borrow().thumb_gen != gen {
            return glib::ControlFlow::Break;
        }
        for _ in 0..2 {
            let next = q2.lock().unwrap().pop_front();
            let Some((path, bytes)) = next else { break };
            let animation = if bytes.is_empty() && is_video(&path) {
                cache_path(&path)
                    .and_then(|cache| gdk_pixbuf::PixbufAnimation::from_file(cache).ok())
            } else {
                None
            };
            let tex = if bytes.is_empty() {
                load_cached_thumb(&path)
            } else {
                decode_pixbuf(&bytes).map(|px| {
                    let _ = save_png_cache(&path, &px);
                    gtk::gdk::Texture::for_pixbuf(&px)
                })
            };
            if let Some(tex) = tex {
                let mut st = ui_t.state.borrow_mut();
                if st.thumb_gen == gen {
                    st.thumbs.insert(path.clone(), tex.clone());
                    if let Some(animation) = animation {
                        st.video_previews.insert(path.clone(), animation);
                    }
                    if let Some(pic) = st.pictures.get(&path) {
                        pic.set_paintable(Some(&tex));
                    }
                }
                drop(st);
                update_video_preview(&ui_t);
            }
        }
        if d2.load(std::sync::atomic::Ordering::Acquire) && q2.lock().unwrap().is_empty() {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

fn drop_index(list: &[&str], value: &str) -> u32 {
    list.iter().position(|s| *s == value).unwrap_or(0) as u32
}

#[allow(deprecated)]
fn show_help(parent: &ApplicationWindow) {
    let dlg = Dialog::builder()
        .transient_for(parent)
        .modal(true)
        .title("Aspire Background shortcuts")
        .build();
    let label = Label::new(Some(
        "        Arrow keys or Alt+H/J/K/L — move through wallpapers in every mode\n\
         Enter — apply wallpaper\n\
         Random — apply a random wallpaper   Ctrl+R — refresh the grid\n\
         Ctrl+S — toggle grid/carousel (saved for next launch)\n\
         Ctrl+D — refresh wallpapers in Carousel\n\
         Ctrl+Z — hide or show controls in grid mode\n\
         Esc — exit   ? — help\n\
         Search is available in Wallhaven mode\n\
         Change the color engine in aspire-menu › Style › Engine",
    ));
    label.set_margin_top(16);
    label.set_margin_bottom(8);
    label.set_margin_start(20);
    label.set_margin_end(20);
    label.set_xalign(0.0);
    dlg.content_area().append(&label);
    dlg.add_button("Cerrar", ResponseType::Close);
    dlg.connect_response(|d, _| d.destroy());
    dlg.present();
}

fn set_zen(ui: &Ui, zen: bool) {
    if let Some(toolbar) = ui.toolbar_stack.borrow().as_ref() {
        let is_carousel = ui.local_display.visible_child_name().as_deref() == Some("carousel");
        toolbar.set_visible(!zen && !is_carousel);
    }
    ui.config.borrow_mut().zen_mode = zen;
    save_config(&ui.config.borrow());
}

fn set_carousel_mode(ui: &Ui, enabled: bool) {
    for edge in [
        gtk4_layer_shell::Edge::Top,
        gtk4_layer_shell::Edge::Bottom,
        gtk4_layer_shell::Edge::Left,
        gtk4_layer_shell::Edge::Right,
    ] {
        ui.window.set_anchor(edge, enabled);
    }
    if enabled {
        ui.local_display.set_visible_child_name("carousel");
        ui.card.add_css_class("bg-carousel-mode");
        ui.carousel_picture.grab_focus();
    } else {
        ui.local_display.set_visible_child_name("grid");
        ui.card.remove_css_class("bg-carousel-mode");
        ui.grid.grab_focus();
    }
    if ui.config.borrow().carousel_mode != enabled {
        ui.config.borrow_mut().carousel_mode = enabled;
        save_config(&ui.config.borrow());
    }
    if let Some(toolbar) = ui.toolbar_stack.borrow().as_ref() {
        toolbar.set_visible(!enabled && !ui.config.borrow().zen_mode);
    }
    update_video_preview(ui);
}

fn start_search(ui: &Ui, query: String) {
    let q = query.to_lowercase();
    let gen = {
        let mut st = ui.state.borrow_mut();
        st.search_gen = st.search_gen.wrapping_add(1);
        st.search_gen
    };
    if q.is_empty() {
        *ui.pending_query.lock().unwrap() = None;
        let base = ui.state.borrow().base_shown.clone();
        {
            let mut st = ui.state.borrow_mut();
            st.shown = base.clone();
            st.current = 0;
            st.thumbs.clear();
            st.pictures.clear();
            st.video_previews.clear();
            st.thumb_gen = st.thumb_gen.wrapping_add(1);
        }
        rebuild_grid(ui);
        let g = ui.state.borrow().thumb_gen;
        let sh = ui.state.borrow().shown.clone();
        load_thumbs(ui, sh, g);
        set_status(ui, "");
        return;
    }
    if !ui.index_ready.load(std::sync::atomic::Ordering::Acquire)
        && ui.index_building.load(std::sync::atomic::Ordering::Acquire)
    {
        *ui.pending_query.lock().unwrap() = Some(q.clone());
        return;
    }
    let index_ready = ui.index_ready.load(std::sync::atomic::Ordering::Acquire);
    let index_clone: Option<Vec<(String, String)>> = if index_ready {
        let v: Vec<(String, String)> = (*ui.search_index.lock().unwrap()).to_vec();
        Some(v)
    } else {
        ui.index_building
            .store(true, std::sync::atomic::Ordering::Release);
        None
    };
    let cfg = ui.config.borrow().clone();
    let (tx, rx) = std::sync::mpsc::channel();
    {
        let mut st = ui.state.borrow_mut();
        st.search_rx = Some(rx);
        st.search_pending = true;
        st.search_drainer_active = false;
    }
    std::thread::spawn(move || {
        if let Some(index) = index_clone {
            let shown: Vec<String> = index
                .iter()
                .filter(|(_, name)| name.contains(&q))
                .take(30)
                .map(|(p, _)| p.clone())
                .collect();
            let _ = tx.send(SearchMsg::Result {
                gen,
                all: None,
                index: None,
                shown,
            });
        } else {
            let all = walk_images(&cfg.folder, cfg.subfolders, cfg.show_hidden);
            let index: Vec<(String, String)> = all
                .iter()
                .map(|p| (p.clone(), basename(p).to_lowercase()))
                .collect();
            let shown: Vec<String> = index
                .iter()
                .filter(|(_, name)| name.contains(&q))
                .take(30)
                .map(|(p, _)| p.clone())
                .collect();
            let _ = tx.send(SearchMsg::Result {
                gen,
                all: Some(all),
                index: Some(index),
                shown,
            });
        }
    });
    // drainer
    let ui2 = ui.clone();
    if !ui.state.borrow().search_drainer_active {
        ui.state.borrow_mut().search_drainer_active = true;
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            let msg = {
                let st = ui2.state.borrow();
                st.search_rx.as_ref().and_then(|rx| rx.try_recv().ok())
            };
            if let Some(SearchMsg::Result {
                gen,
                all,
                index,
                shown,
            }) = msg
            {
                let cur = ui2.state.borrow().search_gen;
                {
                    let mut st = ui2.state.borrow_mut();
                    if let Some(a) = all {
                        st.all = a;
                        st.all_loaded = true;
                    }
                    if let Some(idx) = index {
                        *ui2.search_index.lock().unwrap() = Arc::new(idx);
                        ui2.index_ready
                            .store(true, std::sync::atomic::Ordering::Release);
                        st.index_ready = true;
                        ui2.index_building
                            .store(false, std::sync::atomic::Ordering::Release);
                    }
                }
                let pending = if ui2.index_ready.load(std::sync::atomic::Ordering::Acquire) {
                    ui2.pending_query.lock().unwrap().take()
                } else {
                    None
                };
                if let Some(q) = pending {
                    let ui3 = ui2.clone();
                    glib::idle_add_local_once(move || {
                        start_search(&ui3, q);
                    });
                }
                if gen == cur {
                    {
                        let mut st = ui2.state.borrow_mut();
                        st.shown = shown.clone();
                        st.current = 0;
                        st.thumbs.clear();
                        st.pictures.clear();
                        st.video_previews.clear();
                        st.thumb_gen = st.thumb_gen.wrapping_add(1);
                    }
                    rebuild_grid(&ui2);
                    let g = ui2.state.borrow().thumb_gen;
                    let sh = ui2.state.borrow().shown.clone();
                    load_thumbs(&ui2, sh, g);
                    set_status(&ui2, "");
                }
                ui2.state.borrow_mut().search_pending = false;
            }
            if ui2.state.borrow().search_pending {
                glib::ControlFlow::Continue
            } else {
                ui2.state.borrow_mut().search_rx = None;
                ui2.state.borrow_mut().search_drainer_active = false;
                glib::ControlFlow::Break
            }
        });
    }
}

fn connect_signals(ui: &Ui, sender: ComponentSender<BgApp>) {
    {
        let ui_s = ui.clone();
        ui.grid.connect_selected_children_changed(move |_| {
            if let Some(first) = ui_s.grid.selected_children().first() {
                if let Ok(mut st) = ui_s.state.try_borrow_mut() {
                    st.current = first.index() as usize;
                }
            }
            update_video_preview(&ui_s);
        });
    }
    {
        let sender = sender.clone();
        ui.grid.connect_child_activated(move |_, _| {
            sender.input(BgMsg::ApplyCurrent);
        });
    }
    // Cambios de opciones: persisten y (backend/fill/monitor/sort) reescanean.
    {
        let ui_o = ui.clone();
        ui.backend_drop.connect_selected_notify(move |d| {
            let avail = available_backends();
            let v = avail
                .get(d.selected() as usize)
                .map(|s| s.as_str())
                .unwrap_or("swaybg");
            ui_o.config.borrow_mut().backend = v.to_string();
            save_config(&ui_o.config.borrow());
            reapply_wallpaper(&ui_o);
        });
    }
    {
        let ui_o = ui.clone();
        ui.monitor_drop.connect_selected_notify(move |d| {
            let mons = monitor_list();
            let v = mons
                .get(d.selected() as usize)
                .map(|s| s.as_str())
                .unwrap_or("All");
            ui_o.config.borrow_mut().monitors = v.to_string();
            save_config(&ui_o.config.borrow());
        });
    }
    {
        let ui_o = ui.clone();
        let fills: Vec<&str> = FILLS.to_vec();
        ui.fill_drop.connect_selected_notify(move |d| {
            let v = fills.get(d.selected() as usize).unwrap_or(&"fill");
            ui_o.config.borrow_mut().fill = v.to_string();
            save_config(&ui_o.config.borrow());
            reapply_wallpaper(&ui_o);
        });
    }
    {
        let ui_o = ui.clone();
        let sort_modes = ["random", "modified", "created", "oldest"];
        ui.sort_drop.connect_selected_notify(move |d| {
            let v = sort_modes
                .get(d.selected() as usize)
                .copied()
                .unwrap_or("random");
            ui_o.config.borrow_mut().sort_mode = v.to_string();
            save_config(&ui_o.config.borrow());
            resample(&ui_o);
        });
    }
    {
        let ui_o = ui.clone();
        ui.sub_check.connect_toggled(move |c| {
            ui_o.config.borrow_mut().subfolders = c.is_active();
            save_config(&ui_o.config.borrow());
            resample(&ui_o);
        });
    }
    {
        let ui_o = ui.clone();
        ui.hidden_check.connect_toggled(move |c| {
            ui_o.config.borrow_mut().show_hidden = c.is_active();
            save_config(&ui_o.config.borrow());
            resample(&ui_o);
        });
    }
    let key = gtk::EventControllerKey::new();
    key.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let ui_k = ui.clone();
        let sender = sender.clone();
        key.connect_key_pressed(move |_, keyval, _, state| {
            let wh_mode = ui_k
                .view_stack
                .borrow()
                .as_ref()
                .map(|s| s.visible_child_name().as_deref() == Some("wallhaven"))
                .unwrap_or(false);
            if state.contains(ModifierType::ALT_MASK) {
                match keyval {
                    Key::h => {
                        if wh_mode {
                            move_wallhaven_selection(&ui_k, -1, false);
                        } else {
                            sender.input(BgMsg::MoveSelection {
                                delta: -1,
                                by_row: false,
                            });
                        }
                        return Propagation::Stop;
                    }
                    Key::j => {
                        if wh_mode {
                            move_wallhaven_selection(&ui_k, 1, true);
                        } else {
                            sender.input(BgMsg::MoveSelection {
                                delta: 1,
                                by_row: true,
                            });
                        }
                        return Propagation::Stop;
                    }
                    Key::k => {
                        if wh_mode {
                            move_wallhaven_selection(&ui_k, -1, true);
                        } else {
                            sender.input(BgMsg::MoveSelection {
                                delta: -1,
                                by_row: true,
                            });
                        }
                        return Propagation::Stop;
                    }
                    Key::l => {
                        if wh_mode {
                            move_wallhaven_selection(&ui_k, 1, false);
                        } else {
                            sender.input(BgMsg::MoveSelection {
                                delta: 1,
                                by_row: false,
                            });
                        }
                        return Propagation::Stop;
                    }
                    _ => {}
                }
            }
            if !wh_mode && state.contains(ModifierType::CONTROL_MASK) && keyval == Key::s {
                sender.input(BgMsg::ToggleCarousel);
                return Propagation::Stop;
            }
            if !wh_mode
                && ui_k.local_display.visible_child_name().as_deref() == Some("carousel")
                && state.contains(ModifierType::CONTROL_MASK)
                && keyval == Key::d
            {
                sender.input(BgMsg::Refresh);
                return Propagation::Stop;
            }
            if !wh_mode
                && ui_k.local_display.visible_child_name().as_deref() != Some("carousel")
                && state.contains(ModifierType::CONTROL_MASK)
                && keyval == Key::z
            {
                let zen = !ui_k.config.borrow().zen_mode;
                set_zen(&ui_k, zen);
                return Propagation::Stop;
            }
            if wh_mode {
                if keyval == Key::Escape {
                    sender.input(BgMsg::Close);
                    return Propagation::Stop;
                }
                return Propagation::Proceed;
            }
            // Refresh: Ctrl+R
            if state.contains(ModifierType::CONTROL_MASK) && keyval == Key::r {
                sender.input(BgMsg::Refresh);
                return Propagation::Stop;
            }
            if keyval == Key::Escape {
                sender.input(BgMsg::Close);
                Propagation::Stop
            } else if keyval == Key::Return || keyval == Key::KP_Enter {
                sender.input(BgMsg::ApplyCurrent);
                Propagation::Stop
            } else if keyval == Key::Up {
                sender.input(BgMsg::MoveSelection {
                    delta: -1,
                    by_row: true,
                });
                Propagation::Stop
            } else if keyval == Key::Down {
                sender.input(BgMsg::MoveSelection {
                    delta: 1,
                    by_row: true,
                });
                Propagation::Stop
            } else if keyval == Key::Left {
                sender.input(BgMsg::MoveSelection {
                    delta: -1,
                    by_row: false,
                });
                Propagation::Stop
            } else if keyval == Key::Right {
                sender.input(BgMsg::MoveSelection {
                    delta: 1,
                    by_row: false,
                });
                Propagation::Stop
            } else {
                Propagation::Proceed
            }
        });
    }
    ui.window.add_controller(key);
}

fn monitor_list() -> Vec<String> {
    let mut v = vec!["All".to_string()];
    if let Some(display) = gtk::gdk::Display::default() {
        let monitors = display.monitors();
        for i in 0..monitors.n_items() {
            if let Some(obj) = monitors.item(i) {
                if let Ok(monitor) = obj.downcast::<gtk::gdk::Monitor>() {
                    if let Some(connector) = monitor.connector() {
                        let connector = connector.to_string();
                        if !v.contains(&connector) {
                            v.push(connector);
                        }
                    }
                }
            }
        }
    }
    let saved = load_config().monitors;
    if saved != "All" && !v.contains(&saved) {
        v.push(saved);
    }
    v
}

fn scroll_edge(ui: &Ui, top: bool) {
    let adj = ui.scroll.vadjustment();
    adj.set_value(if top {
        adj.lower()
    } else {
        adj.upper() - adj.page_size()
    });
    let n = ui.state.borrow().shown.len() as i32;
    if n > 0 {
        let idx = if top { 0 } else { n - 1 };
        ui.state.borrow_mut().current = idx as usize;
        if let Some(child) = ui.grid.child_at_index(idx) {
            ui.grid.select_child(&child);
        }
    }
}

fn choose_folder(ui: &Ui) {
    let folder = ui.config.borrow().folder.clone();
    let bin_name = format!("{}", "aspire-files");
    let mut command = std::process::Command::new(&bin_name);
    command
        .args(["--select-folder", &folder])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            set_status(ui, &format!("Error: could not open aspire-files: {error}"));
            return;
        }
    };

    set_status(ui, "Select a folder in aspire-files");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });
    let ui = ui.clone();
    glib::timeout_add_local(Duration::from_millis(100), move || match rx.try_recv() {
        Ok(Ok(output)) if output.status.success() => {
            let folder = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if folder.is_empty() {
                set_status(&ui, "Folder selection cancelled");
            } else if !Path::new(&folder).is_dir() {
                let detail = String::from_utf8_lossy(&output.stderr);
                let detail = detail.trim();
                let message = if detail.is_empty() {
                    "Error: aspire-files did not return a valid folder".to_string()
                } else {
                    format!("Error: aspire-files: {detail}")
                };
                set_status(&ui, &message);
            } else {
                ui.config.borrow_mut().folder = folder;
                save_config(&ui.config.borrow());
                resample(&ui);
                set_status(&ui, "");
            }
            glib::ControlFlow::Break
        }
        Ok(Ok(output)) => {
            let detail = String::from_utf8_lossy(&output.stderr);
            if !detail.trim().is_empty() {
                set_status(&ui, &format!("Error: aspire-files: {}", detail.trim()));
            } else {
                set_status(&ui, "Folder selection cancelled");
            }
            glib::ControlFlow::Break
        }
        Ok(Err(error)) => {
            set_status(&ui, &format!("Error: aspire-files: {error}"));
            glib::ControlFlow::Break
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            set_status(&ui, "Error: aspire-files folder selector disconnected");
            glib::ControlFlow::Break
        }
    });
}

fn opt_row(label: &str) -> (GtkBox, Label) {
    let b = GtkBox::new(Orientation::Horizontal, 6);
    let l = Label::new(Some(label));
    l.add_css_class("bg-optlabel");
    b.append(&l);
    (b, l)
}

#[allow(clippy::too_many_lines)]
fn build_ui(
    window: ApplicationWindow,
    config: BgConfig,
    _images: Vec<String>,
    theme_system: Option<String>,
    sender: ComponentSender<BgApp>,
) -> Ui {
    window.add_css_class("bg-window");
    window.set_size_request(WIN_W, -1);
    window.set_decorated(false);

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("aspire-bg"));
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_exclusive_zone(0);
    for edge in [
        gtk4_layer_shell::Edge::Top,
        gtk4_layer_shell::Edge::Bottom,
        gtk4_layer_shell::Edge::Left,
        gtk4_layer_shell::Edge::Right,
    ] {
        window.set_anchor(edge, config.carousel_mode);
    }

    let card = GtkBox::new(Orientation::Vertical, 0);
    card.add_css_class("bg-card");
    card.set_size_request(WIN_W, WIN_H);
    card.set_halign(gtk::Align::Center);
    card.set_valign(gtk::Align::Center);
    card.set_hexpand(false);
    card.set_vexpand(false);

    let bar = GtkBox::new(Orientation::Horizontal, 10);
    bar.add_css_class("bg-bar");
    let engine_name = match theme_engine::active_engine() {
        Engine::Matugen => "Matugen",
        Engine::Tinty => "Tinty",
    };
    let engine_label = Label::new(Some(engine_name));
    engine_label.add_css_class("bg-engine-indicator");
    engine_label.set_valign(gtk::Align::Center);
    let btn_random = Button::with_label("Random");
    btn_random.add_css_class("bg-toolbar-btn");
    btn_random.set_valign(gtk::Align::Center);
    btn_random.set_tooltip_text(Some("Apply a random wallpaper"));
    let btn_reshuffle = Button::with_label("Refresh");
    btn_reshuffle.add_css_class("bg-toolbar-btn");
    btn_reshuffle.set_valign(gtk::Align::Center);
    btn_reshuffle.set_tooltip_text(Some("Refresh wallpapers (Ctrl+R)"));
    let view_toggle = Button::with_label("Carousel");
    view_toggle.add_css_class("bg-toolbar-btn");
    view_toggle.set_valign(gtk::Align::Center);
    view_toggle.set_tooltip_text(Some("Switch grid/carousel (Ctrl+S)"));
    let btn_folder = Button::with_label("Folder");
    btn_folder.add_css_class("bg-btn");
    btn_folder.set_valign(gtk::Align::Center);
    btn_folder.set_tooltip_text(Some("Choose folder with aspire-files"));
    let btn_options = gtk::MenuButton::new();
    btn_options.set_label("Options");
    btn_options.add_css_class("bg-options-button");
    btn_options.set_valign(gtk::Align::Center);
    let toolbar_spacer_left = GtkBox::new(Orientation::Horizontal, 0);
    toolbar_spacer_left.set_hexpand(true);
    let toolbar_spacer_right = GtkBox::new(Orientation::Horizontal, 0);
    toolbar_spacer_right.set_hexpand(true);
    let toolbar_actions = GtkBox::new(Orientation::Horizontal, 12);
    toolbar_actions.append(&view_toggle);
    toolbar_actions.append(&btn_reshuffle);
    toolbar_actions.append(&btn_random);
    bar.append(&engine_label);
    bar.append(&toolbar_spacer_left);
    bar.append(&toolbar_actions);
    bar.append(&toolbar_spacer_right);
    bar.append(&btn_options);

    let (b_backend, _) = opt_row("Wallpaper engine:");
    // Solo motores instalados; si no hay ninguno se avisa en la cuadrícula.
    let avail = available_backends();
    let backend_drop =
        DropDown::from_strings(&avail.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    let (b_monitor, _) = opt_row("Monitor:");
    let mons = monitor_list();
    let monitor_drop = DropDown::from_strings(&mons.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    let (b_fill, _) = opt_row("Fit:");
    let fill_drop = DropDown::from_strings(&FILLS);
    let (_b_sort, _) = opt_row("Sort:");
    let sort_drop =
        DropDown::from_strings(&["Random", "Last modified", "Date added", "Oldest first"]);
    let sub_check = CheckButton::with_label("Subfolders");
    let hidden_check = CheckButton::with_label("Hidden files");
    for w in [&backend_drop, &monitor_drop, &fill_drop, &sort_drop] {
        w.set_valign(gtk::Align::Center);
    }
    for w in [&sub_check, &hidden_check] {
        w.set_valign(gtk::Align::Center);
    }
    let scroll = ScrolledWindow::new();
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_has_frame(false);
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    let grid = FlowBox::new();
    grid.add_css_class("bg-grid");
    grid.set_homogeneous(true);
    grid.set_min_children_per_line(3);
    grid.set_max_children_per_line(3);
    grid.set_column_spacing(12);
    grid.set_row_spacing(12);
    grid.set_selection_mode(gtk::SelectionMode::Single);
    grid.set_activate_on_single_click(true);
    grid.set_focusable(true);
    scroll.set_child(Some(&grid));

    // Stack: página local + página Wallhaven (misma ventana, layer-shell).
    let local_display = gtk::Stack::new();
    local_display.set_vexpand(true);
    local_display.set_hexpand(true);
    local_display.add_named(&scroll, Some("grid"));
    let carousel_box = GtkBox::new(Orientation::Vertical, 10);
    carousel_box.set_halign(gtk::Align::Center);
    carousel_box.set_valign(gtk::Align::Fill);
    carousel_box.set_hexpand(true);
    carousel_box.set_vexpand(true);
    let carousel_row = GtkBox::new(Orientation::Horizontal, 10);
    carousel_row.set_halign(gtk::Align::Center);
    carousel_row.set_valign(gtk::Align::Center);
    carousel_row.set_hexpand(true);
    let carousel_side_pictures: Vec<Picture> = [80, 130, 130, 80]
        .into_iter()
        .map(|width| {
            let picture = Picture::new();
            picture.set_content_fit(ContentFit::Cover);
            picture.set_can_shrink(true);
            let height = if width >= 130 { 260 } else { 180 };
            picture.set_size_request(width, height);
            picture.add_css_class("bg-carousel-side");
            carousel_row.append(&picture);
            picture
        })
        .collect();
    let carousel_picture = Picture::new();
    carousel_picture.set_content_fit(ContentFit::Cover);
    carousel_picture.set_can_shrink(true);
    carousel_picture.set_size_request(520, 400);
    carousel_picture.set_focusable(true);
    carousel_picture.add_css_class("bg-carousel-center");
    carousel_row.insert_child_after(&carousel_picture, carousel_side_pictures.get(1));
    carousel_box.append(&carousel_row);
    local_display.add_named(&carousel_box, Some("carousel"));

    let view_stack = gtk::Stack::new();
    view_stack.set_vexpand(true);
    view_stack.set_hexpand(true);
    view_stack.add_named(&local_display, Some("local"));

    let local_toolbar = GtkBox::new(Orientation::Vertical, 0);
    local_toolbar.append(&bar);
    let toolbar_stack = gtk::Stack::new();
    toolbar_stack.set_hexpand(true);
    toolbar_stack.add_named(&local_toolbar, Some("local"));
    toolbar_stack.set_visible_child_name("local");

    view_stack.set_visible_child_name("local");

    let opt_popover = gtk::Popover::new();
    let opt_box = GtkBox::new(Orientation::Vertical, 6);
    opt_box.append(&b_backend);
    opt_box.append(&backend_drop);
    opt_box.append(&b_monitor);
    opt_box.append(&monitor_drop);
    opt_box.append(&b_fill);
    opt_box.append(&fill_drop);
    let sort_label = Label::new(Some("Sort by"));
    sort_label.add_css_class("bg-optlabel");
    opt_box.append(&sort_label);
    opt_box.append(&sort_drop);
    opt_box.append(&sub_check);
    opt_box.append(&hidden_check);
    opt_box.append(&btn_folder);
    let btn_exit = Button::with_label("Exit");
    btn_exit.add_css_class("bg-btn");
    opt_box.append(&btn_exit);
    #[cfg(feature = "wallhaven")]
    let btn_wh = {
        let button = Button::with_label("Wallhaven");
        button.add_css_class("bg-btn");
        opt_box.append(&button);
        button
    };
    opt_popover.set_child(Some(&opt_box));
    btn_options.set_popover(Some(&opt_popover));

    card.append(&toolbar_stack);
    card.append(&view_stack);
    let root = gtk::Overlay::new();
    root.set_hexpand(true);
    root.set_vexpand(true);
    root.set_child(Some(&card));
    window.set_child(Some(&root));

    let provider = CssProvider::new();
    provider.load_from_string(&build_css());
    let display = gtk::gdk::Display::default().expect("aspire-bg: no display");
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    // recarga en vivo al cambiar el tema
    let theme_dir = gtk::gio::File::for_path(&format!(
        "{}/.config/theme-engine/current",
        std::env::var("HOME").unwrap_or_default()
    ));
    if let Ok(monitor) = theme_dir.monitor_directory(gtk::gio::FileMonitorFlags::NONE, gtk::gio::Cancellable::NONE) {
        let p = provider.clone();
        monitor.connect_changed(move |_, _, _, _| {
            p.load_from_string(&build_css());
        });
        std::mem::forget(monitor);
    }

    let ui = Ui {
        window: window.clone(),
        card: card.clone(),
        bar: bar.clone(),
        provider: provider.clone(),
        grid: grid.clone(),
        scroll: scroll.clone(),
        local_display: local_display.clone(),
        carousel_picture: carousel_picture.clone(),
        carousel_side_pictures: carousel_side_pictures.clone(),
        view_toggle: view_toggle.clone(),
        backend_drop: backend_drop.clone(),
        monitor_drop: monitor_drop.clone(),
        fill_drop: fill_drop.clone(),
        sort_drop: sort_drop.clone(),
        sub_check: sub_check.clone(),
        hidden_check: hidden_check.clone(),
        wallhaven_grid: Rc::new(RefCell::new(None)),
        wallhaven_scroll: Rc::new(RefCell::new(None)),
        wallhaven_index: Rc::new(Cell::new(0)),
        state: Rc::new(RefCell::new(State {
            all: Vec::new(),
            all_loaded: false,
            shown: Vec::new(),
            base_shown: Vec::new(),
            current: 0,
            thumbs: HashMap::new(),
            pictures: HashMap::new(),
            video_previews: HashMap::new(),
            video_preview_src: None,
            thumb_gen: 0,
            search_gen: 0,
            search_pending: false,
            search_rx: None,
            search_drainer_active: false,
            sample_gen: 1,
            sample_pending: true,
            sample_rx: None,
            sample_drainer_active: false,
            index_ready: false,
        })),
        config: Rc::new(RefCell::new(config)),
        theme_system: theme_system.clone(),
        busy: Arc::new(AtomicBool::new(false)),
        search_index: Arc::new(Mutex::new(Arc::new(Vec::new()))),
        index_ready: Arc::new(AtomicBool::new(false)),
        index_building: Arc::new(AtomicBool::new(false)),
        pending_query: Arc::new(Mutex::new(None)),
        view_stack: Rc::new(RefCell::new(Some(view_stack.clone()))),
        toolbar_stack: Rc::new(RefCell::new(Some(toolbar_stack.clone()))),
    };

    #[cfg(feature = "wallhaven")]
    setup_wallhaven(&ui, &view_stack, &toolbar_stack, &btn_wh);

    // Sincroniza los controles con la config (sin disparar rescans).
    // Si el motor guardado no está instalado, cae al primero disponible.
    {
        let avail = available_backends();
        let mut cfg = ui.config.borrow_mut();
        if !avail.iter().any(|b| *b == cfg.backend) {
            if let Some(first) = avail.first() {
                cfg.backend = first.clone();
            }
        }
    }
    save_config(&ui.config.borrow());
    backend_drop.set_selected({
        let avail = available_backends();
        avail
            .iter()
            .position(|b| *b == ui.config.borrow().backend)
            .unwrap_or(0) as u32
    });
    monitor_drop.set_selected({
        let mons = monitor_list();
        mons.iter()
            .position(|m| *m == ui.config.borrow().monitors)
            .unwrap_or(0) as u32
    });
    fill_drop.set_selected(drop_index(&FILLS, &ui.config.borrow().fill));
    sort_drop.set_selected(match ui.config.borrow().sort_mode.as_str() {
        "modified" => 1,
        "created" => 2,
        "oldest" => 3,
        _ => 0,
    });
    sub_check.set_active(ui.config.borrow().subfolders);
    hidden_check.set_active(ui.config.borrow().show_hidden);
    // NOTA: set_selected/set_active disparan los handlers de arriba; el
    // rescan resultante es inofensivo (misma carpeta y orden).
    {
        let sender = sender.clone();
        btn_random.connect_clicked(move |_| sender.input(BgMsg::ApplyRandom));
    }
    {
        let sender = sender.clone();
        btn_reshuffle.connect_clicked(move |_| sender.input(BgMsg::Refresh));
    }
    {
        let sender = sender.clone();
        btn_folder.connect_clicked(move |_| sender.input(BgMsg::ChooseFolder));
    }
    {
        let sender = sender.clone();
        btn_exit.connect_clicked(move |_| sender.input(BgMsg::Close));
    }
    connect_signals(&ui, sender.clone());
    {
        let click = gtk::GestureClick::new();
        let sender = sender.clone();
        click.connect_released(move |_, _, _, _| sender.input(BgMsg::ApplyCurrent));
        carousel_picture.add_controller(click);
    }
    {
        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        let sender = sender.clone();
        scroll.connect_scroll(move |_, _, dy| {
            if dy == 0.0 {
                return Propagation::Proceed;
            }
            sender.input(BgMsg::MoveSelection {
                delta: if dy > 0.0 { 1 } else { -1 },
                by_row: false,
            });
            Propagation::Stop
        });
        carousel_row.add_controller(scroll);
    }
    {
        let sender = sender.clone();
        view_toggle.connect_clicked(move |_| {
            sender.input(BgMsg::ToggleCarousel);
        });
    }
    for (picture, offset) in carousel_side_pictures.iter().zip([-2_isize, -1, 1, 2]) {
        let click = gtk::GestureClick::new();
        let sender = sender.clone();
        click.connect_released(move |_, _, _, _| {
            sender.input(BgMsg::MoveSelection {
                delta: offset,
                by_row: false,
            });
            sender.input(BgMsg::ApplyCurrent);
        });
        picture.add_controller(click);
    }
    rebuild_grid(&ui);
    launch_sample(&ui);
    if ui.config.borrow().carousel_mode {
        set_carousel_mode(&ui, true);
    } else if ui.config.borrow().zen_mode {
        toolbar_stack.set_visible(false);
    }
    if let Some(system) = &ui.theme_system {
        window.set_title(Some(&format!("Aspire Background — theme {system}")));
        set_status(
            &ui,
            &format!("New {system} theme: choose a wallpaper and press Enter"),
        );
    }
    window.set_default_size(WIN_W, WIN_H);
    let initial_focus = if ui.config.borrow().carousel_mode {
        carousel_picture.clone().upcast::<gtk::Widget>()
    } else {
        grid.clone().upcast::<gtk::Widget>()
    };
    glib::idle_add_local_once(move || {
        initial_focus.grab_focus();
    });
    ui
}

#[cfg(feature = "wallhaven")]
fn setup_wallhaven(ui: &Ui, view_stack: &gtk::Stack, toolbar_stack: &gtk::Stack, btn_wh: &Button) {
    use crate::wallhaven;
    // widgets
    let wh_scroll = ScrolledWindow::new();
    wh_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    wh_scroll.set_has_frame(false);
    wh_scroll.set_vexpand(true);
    wh_scroll.set_hexpand(true);
    let wh_grid = FlowBox::new();
    wh_grid.add_css_class("bg-grid");
    wh_grid.set_homogeneous(true);
    wh_grid.set_min_children_per_line(3);
    wh_grid.set_max_children_per_line(3);
    wh_grid.set_column_spacing(12);
    wh_grid.set_row_spacing(12);
    wh_grid.set_selection_mode(gtk::SelectionMode::Single);
    wh_grid.set_activate_on_single_click(true);
    wh_grid.set_focusable(true);
    wh_scroll.set_child(Some(&wh_grid));
    *ui.wallhaven_grid.borrow_mut() = Some(wh_grid.clone());
    *ui.wallhaven_scroll.borrow_mut() = Some(wh_scroll.clone());
    {
        let current = ui.wallhaven_index.clone();
        wh_grid.connect_selected_children_changed(move |grid| {
            if let Some(child) = grid.selected_children().first() {
                current.set(child.index() as usize);
            }
        });
    }

    let wh_top = GtkBox::new(Orientation::Horizontal, 8);
    let wh_entry = Entry::new();
    wh_entry.set_placeholder_text(Some("Search Wallhaven..."));
    wh_entry.add_css_class("bg-entry");
    wh_entry.set_hexpand(true);
    let wh_sort = DropDown::from_strings(&["Hot", "Toplist", "Uploads", "Random", "Views"]);
    wh_sort.set_selected(0);
    let btn_refresh = Button::with_label("Refresh");
    btn_refresh.add_css_class("bg-btn");
    btn_refresh.set_tooltip_text(Some("Show a different random batch"));
    let btn_more = Button::with_label("More");
    btn_more.add_css_class("bg-btn");
    btn_more.set_sensitive(false);
    let btn_exit = Button::with_label("Exit");
    btn_exit.add_css_class("bg-btn");
    wh_top.append(&wh_entry);
    wh_top.append(&wh_sort);
    wh_top.append(&btn_refresh);
    wh_top.append(&btn_more);
    wh_top.append(&btn_exit);

    let wh_status = Label::new(None);
    wh_status.add_css_class("bg-status");
    wh_status.set_visible(false);
    wh_status.set_xalign(0.0);

    let wh_toolbar = GtkBox::new(Orientation::Vertical, 0);
    wh_toolbar.append(&wh_top);
    toolbar_stack.add_named(&wh_toolbar, Some("wallhaven"));

    let wh_box = GtkBox::new(Orientation::Vertical, 0);
    wh_status.set_margin_top(0);
    wh_status.set_margin_bottom(0);
    wh_box.append(&wh_status);
    wh_box.append(&wh_scroll);
    view_stack.add_named(&wh_box, Some("wallhaven"));

    let cfg0 = ui.config.borrow().clone();
    let wh_state = Rc::new(RefCell::new(WhState {
        categories: "011".to_string(),
        sort: "Hot".to_string(),
        purity: "100".to_string(),
        seed: None,
        page: 0,
        last_page: 1,
        items: Vec::new(),
        seen: HashSet::new(),
        downloading: HashSet::new(),
        thumbs: HashMap::new(),
        pictures: HashMap::new(),
        gen: 0,
        pending: Arc::new(AtomicUsize::new(0)),
        tx: None,
        rx: None,
        drainer_active: false,
        search_src: Rc::new(RefCell::new(None)),
        opened: false,
        loading_page: false,
    }));
    {
        let sort_map = ["Hot", "Toplist", "Uploads", "Random", "Views"];
        let s = sort_map
            .get(wh_sort.selected() as usize)
            .copied()
            .unwrap_or("Hot");
        wh_state.borrow_mut().sort = s.to_string();
    }

    // drainer
    let start_drainer = {
        let ui = ui.clone();
        let wh_state = wh_state.clone();
        let wh_grid = wh_grid.clone();
        let wh_status = wh_status.clone();
        let btn_more = btn_more.clone();
        Rc::new(move || {
            let mut st = wh_state.borrow_mut();
            if st.drainer_active {
                return;
            }
            st.drainer_active = true;
            drop(st);
            let ui = ui.clone();
            let wh_state = wh_state.clone();
            let wh_grid = wh_grid.clone();
            let wh_status = wh_status.clone();
            let btn_more = btn_more.clone();
            glib::timeout_add_local(Duration::from_millis(50), move || {
                let has_rx = wh_state.borrow().rx.is_some();
                if !has_rx {
                    wh_state.borrow_mut().drainer_active = false;
                    return glib::ControlFlow::Break;
                }
                loop {
                    let recv = {
                        let st = wh_state.borrow();
                        st.rx.as_ref().map(|r| r.try_recv())
                    };
                    match recv {
                        Some(Ok(ev)) => {
                            // handle ev below by binding
                            let ev = ev;
                            match ev {
                                WhEvent::Results {
                                    gen,
                                    page,
                                    last_page,
                                    items,
                                    seed,
                                } => {
                                    let mut st = wh_state.borrow_mut();
                                    if st.gen != gen {
                                        continue;
                                    }
                                    if page == 1 {
                                        st.items.clear();
                                        st.seen.clear();
                                        st.thumbs.clear();
                                        st.pictures.clear();
                                    }
                                    for it in items {
                                        if st.seen.insert(it.id.clone()) {
                                            st.items.push(it);
                                        }
                                    }
                                    st.page = page;
                                    st.last_page = last_page;
                                    if page == 1 {
                                        st.seed = seed.clone();
                                    }
                                    // rebuild grid
                                    while let Some(c) = wh_grid.first_child() {
                                        wh_grid.remove(&c);
                                    }
                                    st.pictures.clear();
                                    ui.wallhaven_index.set(0);
                                    let items_clone: Vec<_> = st.items.clone();
                                    for it in &items_clone {
                                        let tex = st.thumbs.get(&it.id);
                                        let (tile, pic) = make_tile(tex, false);
                                        tile.set_hexpand(true);
                                        wh_grid.append(&tile);
                                        st.pictures.insert(it.id.clone(), pic);
                                    }
                                    if let Some(first) = wh_grid.child_at_index(0) {
                                        wh_grid.select_child(&first);
                                    }
                                    st.loading_page = false;
                                    let can_more = can_load_more(st.page, st.last_page, false);
                                    btn_more.set_sensitive(can_more);
                                    if st.items.is_empty() {
                                        wh_status.set_text("No results");
                                    } else {
                                        wh_status.set_text(&format!("{} results", st.items.len()));
                                    }
                                    wh_status.set_visible(true);
                                }
                                WhEvent::Thumb {
                                    gen,
                                    id,
                                    width,
                                    height,
                                    stride,
                                    bytes,
                                } => {
                                    let mut st = wh_state.borrow_mut();
                                    if st.gen != gen {
                                        eprintln!("WH thumb stale gen");
                                        continue;
                                    }
                                    let gbytes = glib::Bytes::from_owned(bytes);
                                    let tex = gtk::gdk::MemoryTexture::new(
                                        width,
                                        height,
                                        gtk::gdk::MemoryFormat::R8g8b8,
                                        &gbytes,
                                        stride,
                                    );
                                    st.thumbs.insert(id.clone(), tex.clone().into());
                                    if let Some(pic) = st.pictures.get(&id) {
                                        pic.set_paintable(Some(&tex));
                                    }
                                }
                                WhEvent::Error { gen, msg } => {
                                    let mut st = wh_state.borrow_mut();
                                    if st.gen != gen {
                                        continue;
                                    }
                                    st.loading_page = false;
                                    btn_more.set_sensitive(can_load_more(
                                        st.page,
                                        st.last_page,
                                        false,
                                    ));
                                    wh_status.set_text(&msg);
                                    wh_status.set_visible(true);
                                }
                                WhEvent::DownloadDone { path } => {
                                    let cfg = ui.config.borrow().clone();
                                    let mut cfg = cfg;
                                    if let Err(e) = apply_wallpaper(&mut cfg, &path, false) {
                                        wh_status.set_text(&format!("Error: {e}"));
                                        wh_status.set_visible(true);
                                    } else {
                                        *ui.config.borrow_mut() = cfg.clone();
                                        let dir = std::path::Path::new(&path).parent().unwrap();
                                        wallhaven::trim_wallhaven_dir(
                                            dir,
                                            selected_wallpaper(&cfg),
                                            &path,
                                        );
                                        notify_wallpaper_changed();
                                        start_color_job(&ui, path.clone(), false);
                                    }
                                }
                                WhEvent::DownloadError { msg } => {
                                    wh_status.set_text(&msg);
                                    wh_status.set_visible(true);
                                }
                            }
                        }
                        _ => break,
                    }
                }
                let pending = wh_state
                    .borrow()
                    .pending
                    .load(std::sync::atomic::Ordering::Acquire);
                if pending == 0 {
                    let empty = wh_state
                        .borrow()
                        .rx
                        .as_ref()
                        .map(|r| r.try_recv().is_err())
                        .unwrap_or(true);
                    if empty {
                        let mut st = wh_state.borrow_mut();
                        st.rx = None;
                        st.tx = None;
                        st.drainer_active = false;
                        return glib::ControlFlow::Break;
                    }
                }
                glib::ControlFlow::Continue
            });
        })
    };

    // trigger download
    let trigger_download = {
        let ui = ui.clone();
        let wh_state = wh_state.clone();
        let wh_status = wh_status.clone();
        Rc::new(move |item: wallhaven::WhItem| {
            let debug = std::env::var("ASPIRE_BG_DEBUG").is_ok();
            if debug {
                eprintln!("WH tile click id={}", item.id);
            }
            // in-flight set
            {
                let mut st = wh_state.borrow_mut();
                if st.downloading.contains(&item.id) {
                    if debug {
                        eprintln!("WH ignored (already downloading) id={}", item.id);
                    }
                    return;
                }
                st.downloading.insert(item.id.clone());
            }
            wh_status.set_text(&format!("Downloading {}...", item.id));
            wh_status.set_visible(true);
            if let Some(pic) = wh_state.borrow().pictures.get(&item.id) {
                pic.add_css_class("wh-downloading");
            }

            let (tx, rx) = std::sync::mpsc::channel();
            let dest = std::path::PathBuf::from(format!(
                "{}/.config/aspire-bg/wallhaven",
                std::env::var("HOME").unwrap_or_default()
            ));
            let txw = tx.clone();
            let item_clone = item.clone();
            if debug {
                eprintln!("WH download start id={}", item.id);
            }
            std::thread::spawn(move || match wallhaven::download_full(&item_clone, &dest) {
                Ok(path) => {
                    if debug {
                        eprintln!(
                            "WH download done id={} path={}",
                            item_clone.id,
                            path.display()
                        );
                    }
                    if txw
                        .send(WhEvent::DownloadDone {
                            path: path.to_string_lossy().to_string(),
                        })
                        .is_err()
                        && debug
                    {
                        eprintln!("WH send DownloadDone failed");
                    }
                }
                Err(e) => {
                    if debug {
                        eprintln!("WH download error id={} err={}", item_clone.id, e);
                    }
                    if txw.send(WhEvent::DownloadError { msg: e }).is_err() && debug {
                        eprintln!("WH send DownloadError failed");
                    }
                }
            });

            // dedicated poll
            let ui = ui.clone();
            let wh_state = wh_state.clone();
            let wh_status = wh_status.clone();
            let item_id = item.id.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                match rx.try_recv() {
                    Ok(WhEvent::DownloadDone { path }) => {
                        if debug {
                            eprintln!("WH DownloadDone received path={}", path);
                        }
                        let mut cfg = ui.config.borrow().clone();
                        match apply_wallpaper(&mut cfg, &path, false) {
                            Ok(()) => {
                                *ui.config.borrow_mut() = cfg.clone();
                                let dir = std::path::Path::new(&path).parent().unwrap();
                                wallhaven::trim_wallhaven_dir(dir, selected_wallpaper(&cfg), &path);
                                notify_wallpaper_changed();
                                start_color_job(&ui, path.clone(), false);
                                if debug {
                                    eprintln!("WH apply_wallpaper OK path={}", path);
                                }
                            }
                            Err(e) => {
                                let msg = format!("Error: {e}");
                                wh_status.set_text(&msg);
                                wh_status.set_visible(true);
                                if debug {
                                    eprintln!("WH apply_wallpaper ERR path={} err={}", path, e);
                                }
                            }
                        }
                        if let Some(pic) = wh_state.borrow().pictures.get(&item_id) {
                            pic.remove_css_class("wh-downloading");
                        }
                        wh_state.borrow_mut().downloading.remove(&item_id);
                        glib::ControlFlow::Break
                    }
                    Ok(WhEvent::DownloadError { msg }) => {
                        if let Some(pic) = wh_state.borrow().pictures.get(&item_id) {
                            pic.remove_css_class("wh-downloading");
                        }
                        wh_status.set_text(&msg);
                        wh_status.set_visible(true);
                        if debug {
                            eprintln!("WH download error shown: {}", msg);
                        }
                        wh_state.borrow_mut().downloading.remove(&item_id);
                        glib::ControlFlow::Break
                    }
                    Ok(_) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        let msg = "Wallhaven: disconnected response channel";
                        wh_status.set_text(msg);
                        wh_status.set_visible(true);
                        if debug {
                            eprintln!("WH channel disconnected");
                        }
                        wh_state.borrow_mut().downloading.remove(&item_id);
                        glib::ControlFlow::Break
                    }
                }
            });
        })
    };

    // start search
    let start_search = {
        let ui = ui.clone();
        let wh_state = wh_state.clone();
        let wh_grid = wh_grid.clone();
        let btn_more = btn_more.clone();
        let wh_entry = wh_entry.clone();
        let start_drainer = start_drainer.clone();
        Rc::new(move |append: bool| {
            let categories = WALLHAVEN_CATEGORIES.to_string();
            let query = wh_entry.text().to_string();
            let page = {
                let mut st = wh_state.borrow_mut();
                st.categories = categories.clone();
                if !append {
                    st.last_page = 1;
                    st.seed = None;
                    st.items.clear();
                    st.seen.clear();
                    st.thumbs.clear();
                    st.pictures.clear();
                    while let Some(c) = wh_grid.first_child() {
                        wh_grid.remove(&c);
                    }
                }
                let p = if append { st.page + 1 } else { 1 };
                if append && !can_load_more(st.page, st.last_page, st.loading_page) {
                    return;
                }
                st.gen = st.gen.wrapping_add(1);
                st.loading_page = append;
                p
            };
            btn_more.set_sensitive(false);
            let gen = wh_state.borrow().gen;
            let pending = wh_state.borrow().pending.clone();
            pending.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            let (tx, rx) = std::sync::mpsc::channel();
            {
                let mut st = wh_state.borrow_mut();
                st.tx = Some(tx.clone());
                st.rx = Some(rx);
            }
            let (sort_api, purity_api, seed_api) = {
                let st = wh_state.borrow();
                let sort_api = match st.sort.as_str() {
                    "Hot" => "hot",
                    "Uploads" => "date_added",
                    "Random" => "random",
                    "Views" => "views",
                    "Toplist" => "toplist",
                    _ => "relevance",
                };
                let seed = if sort_api == "random" && page > 1 {
                    st.seed.clone()
                } else {
                    None
                };
                (sort_api.to_string(), st.purity.clone(), seed)
            };
            let mut cfg = ui.config.borrow().clone();
            cfg.wallhaven.categories = categories.clone();
            cfg.wallhaven.purity = "100".to_string();
            let txw = tx.clone();

            std::thread::spawn(move || {
                match wallhaven::search(
                    &cfg,
                    &query,
                    &categories,
                    &sort_api,
                    &cfg.wallhaven.purity,
                    seed_api.as_deref(),
                    page,
                ) {
                    Ok(sp) => {
                        if txw
                            .send(WhEvent::Results {
                                gen,
                                page,
                                last_page: sp.last_page,
                                items: sp.items.clone(),
                                seed: sp.seed.clone(),
                            })
                            .is_err()
                            && std::env::var("ASPIRE_BG_DEBUG").is_ok()
                        {
                            eprintln!("WH send Results failed");
                        }
                        for it in &sp.items {
                            if let Ok(path) = wallhaven::download_thumb(&it.thumb_url) {
                                if let Ok(px) =
                                    gdk_pixbuf::Pixbuf::from_file_at_scale(&path, 200, 200, true)
                                {
                                    let width = px.width();
                                    let height = px.height();
                                    let stride = px.rowstride() as usize;
                                    let bytes = px.read_pixel_bytes();
                                    if txw
                                        .send(WhEvent::Thumb {
                                            gen,
                                            id: it.id.clone(),
                                            width,
                                            height,
                                            stride,
                                            bytes: bytes.as_ref().to_vec(),
                                        })
                                        .is_err()
                                        && std::env::var("ASPIRE_BG_DEBUG").is_ok()
                                    {
                                        eprintln!("WH send Thumb failed");
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if txw.send(WhEvent::Error { gen, msg: e }).is_err()
                            && std::env::var("ASPIRE_BG_DEBUG").is_ok()
                        {
                            eprintln!("WH send Error failed");
                        }
                    }
                }
                pending.fetch_sub(1, std::sync::atomic::Ordering::Release);
            });
            // arrancar drainer si hace falta
            let need = !wh_state.borrow().drainer_active;
            if need {
                start_drainer();
            }
            let _ = btn_more;
        })
    };

    {
        let window = ui.window.clone();
        btn_exit.connect_clicked(move |_| window.close());
    }

    // Wallhaven view
    {
        let view_stack = view_stack.clone();
        let toolbar_stack = toolbar_stack.clone();
        let wh_state = wh_state.clone();
        let start_search = start_search.clone();
        let wh_entry = wh_entry.clone();
        btn_wh.connect_clicked(move |_| {
            view_stack.set_visible_child_name("wallhaven");
            toolbar_stack.set_visible_child_name("wallhaven");
            wh_entry.grab_focus();
            let mut st = wh_state.borrow_mut();
            if !st.opened {
                st.opened = true;
                drop(st);
                start_search(false);
            }
        });
    }

    // Debounce búsqueda
    {
        let wh_state = wh_state.clone();
        let wh_status = wh_status.clone();
        let start_search = start_search.clone();
        wh_entry.connect_changed(move |_| {
            if let Some(src) = wh_state.borrow_mut().search_src.borrow_mut().take() {
                src.remove();
            }
            wh_state.borrow_mut().items.clear();
            wh_state.borrow_mut().pictures.clear();
            wh_state.borrow_mut().thumbs.clear();
            wh_status.set_text("Searching...");
            wh_status.set_visible(true);
            let wh_state_timer = wh_state.clone();
            let start_search = start_search.clone();
            let src = glib::timeout_add_local_once(Duration::from_millis(350), move || {
                *wh_state_timer.borrow().search_src.borrow_mut() = None;
                start_search(false);
            });
            *wh_state.borrow().search_src.borrow_mut() = Some(src);
        });
    }

    // Sort
    {
        let wh_state = wh_state.clone();
        let start_search = start_search.clone();
        let sort_map = ["Hot", "Toplist", "Uploads", "Random", "Views"];
        wh_sort.connect_selected_notify(move |d| {
            let v = sort_map
                .get(d.selected() as usize)
                .copied()
                .unwrap_or("Hot");
            wh_state.borrow_mut().sort = v.to_string();
            start_search(false);
        });
    }

    {
        let start_search = start_search.clone();
        let wh_sort = wh_sort.clone();
        btn_refresh.connect_clicked(move |_| {
            if wh_sort.selected() == 3 {
                start_search(false);
            } else {
                wh_sort.set_selected(3);
            }
        });
    }

    // More
    {
        let start_search = start_search.clone();
        let btn_more = btn_more.clone();
        let wh_state = wh_state.clone();
        btn_more.connect_clicked(move |_| {
            let st = wh_state.borrow();
            if !can_load_more(st.page, st.last_page, st.loading_page) {
                return;
            }
            drop(st);
            start_search(true);
        });
    }

    // Pick en grid
    {
        let wh_grid = wh_grid.clone();
        let wh_state = wh_state.clone();
        wh_grid.connect_child_activated(move |_, child| {
            let idx = child.index() as usize;
            let item = {
                let st = wh_state.borrow();
                st.items.get(idx).cloned()
            };
            let Some(item) = item else {
                return;
            };
            trigger_download(item);
            let _ = wh_status;
            let _ = ui;
        });
    }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn print_list(images: &[String], cfg: &BgConfig) {
    let mut out = String::from("{\"engine\":\"");
    out.push_str(theme_engine::active_engine().as_str());
    out.push_str("\",\"backend\":\"");
    out.push_str(&json_escape(&cfg.backend));
    out.push_str("\",\"monitors\":\"");
    out.push_str(&json_escape(&cfg.monitors));
    out.push_str("\",\"wallpaper\":\"");
    out.push_str(&json_escape(&cfg.wallpaper));
    out.push_str("\",\"wallpapers\":[");
    for (i, p) in images.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&json_escape(p));
        out.push('"');
    }
    out.push_str("]}");
    println!("{out}");
}

fn print_help() {
    println!(
        "aspire-bg — wallpaper selector and color engine\n\
\n\
Usage: aspire-bg [OPTIONS]\n\
\n\
  (no options)               open the wallpaper grid\n\
  --restore                  restore saved wallpaper without recoloring\n\
  --random                   choose a random wallpaper\n\
  --next                     choose the next wallpaper by name\n\
  --wallpaper IMG            apply IMG\n\
  --apply-colors             reapply the active color palette\n\
  --set-engine matugen|tinty select and apply the color engine\n\
  --set-tinty-theme NAME     apply and save a Tinty scheme\n\
  --create-tinty-theme [S]   open the picker to create a Tinty theme (base16|base24)\n\
  --generate-tinty-theme IMG create and apply a Tinty theme without the GUI\n\
  --theme-system base16|base24  theme system for creation (default: base24)\n\
  --folder DIR               wallpaper folder\n\
  --backend X                select and save the wallpaper engine\n\
  --monitor X                select and save the target monitor\n\
  --fill X                   fill|stretch|fit|center|tile\n\
  --zen-mode                 start without the search and options bars\n\
  --list                     list wallpapers as JSON\n\
  --status                   show engine, wallpaper, and Tinty theme\n\
  --no-colors                do not update colors when changing wallpaper\n\
  --no-post-command          do not run post_command\n\
  -h, --help                 show this help\n\
  -V, --version              show version"
    );
}

fn print_status() {
    let config = load_config();
    println!("engine: {}", theme_engine::active_engine().as_str());
    println!("wallpaper: {}", config.wallpaper);
    if let Some(scheme) = theme_engine::tinty_scheme() {
        println!("tinty-theme: {scheme}");
    }
    println!("config: {}", ini_path());
}

enum Action {
    Gui,
    Wallpaper(String),
    Random,
    Next,
    Restore,
    List,
    ApplyColors,
    SetEngine(String),
    SetTintyTheme(String),
    Status,
    GenerateTheme(String),
    Help,
    Version,
}

fn apply_and_color(config: &mut BgConfig, path: &str, no_post: bool, no_colors: bool) -> bool {
    if let Err(e) = apply_wallpaper(config, path, no_post) {
        eprintln!("aspire-bg: {e}");
        return false;
    }
    if no_colors || is_video(path) {
        return true;
    }
    match theme_engine::on_wallpaper_change(Path::new(path)) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("aspire-bg: colores: {e:#}");
            false
        }
    }
}

fn main() -> glib::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut config = load_config();
    let mut no_post = false;
    let mut no_colors = false;
    let mut theme_system: Option<String> = None;
    let mut action = Action::Gui;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--folder" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    config.folder = shellexpand_home(v);
                }
            }
            "--wallpaper" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    action = Action::Wallpaper(shellexpand_home(v));
                }
            }
            "--backend" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    config.backend = v.clone();
                    save_config(&config);
                }
            }
            "--monitor" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    config.monitors = v.clone();
                    save_config(&config);
                }
            }
            "--fill" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    config.fill = v.clone();
                    save_config(&config);
                }
            }
            "--theme-system" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    theme_system = Some(v.clone());
                }
            }
            "--random" => action = Action::Random,
            "--next" => action = Action::Next,
            "--restore" => action = Action::Restore,
            "--list" => action = Action::List,
            "--apply-colors" => action = Action::ApplyColors,
            "--set-engine" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    action = Action::SetEngine(v.clone());
                }
            }
            "--set-tinty-theme" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    action = Action::SetTintyTheme(v.clone());
                }
            }
            "--generate-tinty-theme" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    action = Action::GenerateTheme(shellexpand_home(v));
                }
            }
            "--create-tinty-theme" => {
                let next_is_system = args
                    .get(i + 1)
                    .map(|v| matches!(v.as_str(), "base16" | "base24"))
                    .unwrap_or(false);
                if next_is_system {
                    i += 1;
                    theme_system = args.get(i).cloned();
                }
                theme_system.get_or_insert_with(|| "base24".to_string());
                action = Action::Gui;
            }
            "--status" => action = Action::Status,
            "--no-post-command" => no_post = true,
            "--no-colors" => no_colors = true,
            "--zen-mode" => {
                config.zen_mode = true;
                save_config(&config);
            }
            "-h" | "--help" => action = Action::Help,
            "-V" | "--version" => action = Action::Version,
            _ => {}
        }
        i += 1;
    }

    match action {
        Action::Help => {
            print_help();
            return glib::ExitCode::SUCCESS;
        }
        Action::Version => {
            println!("aspire-bg {}", env!("CARGO_PKG_VERSION"));
            return glib::ExitCode::SUCCESS;
        }
        Action::Status => {
            print_status();
            return glib::ExitCode::SUCCESS;
        }
        Action::Wallpaper(path) => {
            let ok = apply_and_color(&mut config, &path, no_post, no_colors);
            return if ok {
                glib::ExitCode::SUCCESS
            } else {
                glib::ExitCode::FAILURE
            };
        }
        Action::Random => {
            let images = order_images(
                walk_images(&config.folder, config.subfolders, config.show_hidden),
                "random",
            );
            if let Some(path) = pick_random(&images, selected_wallpaper(&config)) {
                if !apply_and_color(&mut config, &path, no_post, no_colors) {
                    return glib::ExitCode::FAILURE;
                }
            }
            return glib::ExitCode::SUCCESS;
        }
        Action::Next => {
            let images = order_images(
                walk_images(&config.folder, config.subfolders, config.show_hidden),
                "name",
            );
            if let Some(path) = pick_next(&images, selected_wallpaper(&config)) {
                if !apply_and_color(&mut config, &path, no_post, no_colors) {
                    return glib::ExitCode::FAILURE;
                }
            }
            return glib::ExitCode::SUCCESS;
        }
        Action::Restore => {
            // Arranque rápido: el fondo guardado ya tiene su paleta en disco.
            let wallpaper = selected_wallpaper(&config);
            if !wallpaper.is_empty() {
                if let Err(e) = set_wallpaper(&config, wallpaper) {
                    eprintln!("aspire-bg: {e}");
                    return glib::ExitCode::FAILURE;
                }
            }
            return glib::ExitCode::SUCCESS;
        }
        Action::List => {
            let images = order_images(
                walk_images(&config.folder, config.subfolders, config.show_hidden),
                &config.sort.clone(),
            );
            print_list(&images, &config);
            return glib::ExitCode::SUCCESS;
        }
        Action::ApplyColors => {
            return match theme_engine::apply_current_colors() {
                Ok(Some(_)) => glib::ExitCode::SUCCESS,
                Ok(None) => {
                    eprintln!(
                        "aspire-bg: no wallpaper or Tinty theme is saved for engine {}",
                        theme_engine::active_engine().as_str()
                    );
                    glib::ExitCode::FAILURE
                }
                Err(e) => {
                    eprintln!("aspire-bg: {e:#}");
                    glib::ExitCode::FAILURE
                }
            };
        }
        Action::SetEngine(name) => {
            let Some(engine) = Engine::parse(&name) else {
                eprintln!("aspire-bg: invalid engine: {name} (matugen|tinty)");
                return glib::ExitCode::FAILURE;
            };
            return match theme_engine::set_engine(engine) {
                Ok(_) => glib::ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("aspire-bg: {e:#}");
                    glib::ExitCode::FAILURE
                }
            };
        }
        Action::SetTintyTheme(name) => {
            return match aspire_core::themes::apply_theme_by_id(&name) {
                Ok(_) => glib::ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("aspire-bg: {e:#}");
                    glib::ExitCode::FAILURE
                }
            };
        }
        Action::GenerateTheme(image) => {
            let system = theme_system.clone().unwrap_or_else(|| "base24".to_string());
            let result =
                theme_engine::create_tinty_scheme(Path::new(&image), &system).and_then(|scheme| {
                    theme_engine::apply_tinty_scheme(&scheme)?;
                    theme_engine::set_active_engine(Engine::Tinty)?;
                    Ok(scheme)
                });
            return match result {
                Ok(scheme) => {
                    println!("{scheme}");
                    glib::ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("aspire-bg: {e:#}");
                    glib::ExitCode::FAILURE
                }
            };
        }
        Action::Gui => {}
    }

    let walked = walk_images(&config.folder, config.subfolders, config.show_hidden);
    let images = order_images(walked, &config.sort.clone());
    // Los flags ya están consumidos arriba: a GTK se le pasa argv vacío
    // porque rechaza ("Opción desconocida") todo lo que no conoce.
    RelmApp::from_app(Application::builder().build())
        .with_args(Vec::new())
        .run::<BgApp>(BgAppInit {
            config,
            images,
            theme_system,
        });
    glib::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{
        can_load_more, decode_monitor_wallpapers, encode_monitor_wallpapers,
        record_wallpaper_assignment,
    };
    use std::collections::HashMap;

    #[test]
    fn monitor_wallpaper_config_round_trips_delimiters_and_unicode() {
        let wallpapers = HashMap::from([
            (
                "DP-1".to_string(),
                "/home/user/Fondos/foto, verano=1.png".to_string(),
            ),
            (
                "HDMI-A-1".to_string(),
                "/home/user/Imágenes/雪.png".to_string(),
            ),
        ]);
        let encoded = encode_monitor_wallpapers(&wallpapers);
        assert_eq!(decode_monitor_wallpapers(&encoded), Some(wallpapers));
    }

    #[test]
    fn monitor_wallpaper_config_rejects_malformed_encoding() {
        assert_eq!(decode_monitor_wallpapers("DP-1=%GG"), None);
        assert_eq!(decode_monitor_wallpapers("DP-1"), None);
    }

    #[test]
    fn applying_to_each_monitor_preserves_existing_assignments() {
        let monitors = vec![
            "All".to_string(),
            "DP-1".to_string(),
            "HDMI-A-1".to_string(),
        ];
        let mut wallpaper = "/wallpapers/original.jpg".to_string();
        let mut assignments = HashMap::new();

        record_wallpaper_assignment(
            "DP-1",
            &mut wallpaper,
            &mut assignments,
            &monitors,
            "/wallpapers/left.jpg",
        );
        record_wallpaper_assignment(
            "HDMI-A-1",
            &mut wallpaper,
            &mut assignments,
            &monitors,
            "/wallpapers/right.jpg",
        );

        assert_eq!(assignments.get("DP-1").unwrap(), "/wallpapers/left.jpg");
        assert_eq!(
            assignments.get("HDMI-A-1").unwrap(),
            "/wallpapers/right.jpg"
        );
        assert_eq!(wallpaper, "/wallpapers/original.jpg");
    }

    #[test]
    fn wallhaven_pagination_continues_past_five_pages() {
        assert!(can_load_more(6, 12, false));
        assert!(!can_load_more(6, 12, true));
        assert!(!can_load_more(12, 12, false));
    }
}
