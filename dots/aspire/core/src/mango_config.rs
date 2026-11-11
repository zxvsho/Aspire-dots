use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
pub struct ColorHex {
    pub value: u32,
}

impl ColorHex {
    pub fn from_rgba_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim().trim_start_matches('#');
        let value = if hex.len() == 6 {
            format!("{hex}ff")
        } else if hex.len() == 8 {
            hex.to_string()
        } else {
            return None;
        };
        let value = u32::from_str_radix(&value, 16).ok()?;
        Some(Self { value })
    }

    pub fn from_rgb(rgb: [u8; 3]) -> Self {
        Self {
            value: u32::from_be_bytes([0x00, rgb[0], rgb[1], rgb[2]]) << 8 | 0xff,
        }
    }
}

pub fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no se pudo obtener HOME")?;
    Ok(home.join(".config").join("mango"))
}

fn active_path() -> Result<PathBuf> {
    let dir = config_dir()?;
    if dir.join("config.toml").is_file() {
        return Ok(dir.join("config.toml"));
    }
    if dir.join("config.conf").is_file() {
        return Ok(dir.join("config.conf"));
    }
    Ok(dir.join("config.toml"))
}

fn backup_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    path.with_file_name(format!("{filename}.bak-{stamp}"))
}

fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let dir = path.parent().context("la ruta no tiene directorio padre")?;
    let temp = dir.join(format!(
        ".{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    fs::write(&temp, content).with_context(|| format!("no se pudo escribir {}", temp.display()))?;
    fs::rename(&temp, path).with_context(|| format!("no se pudo reemplazar {}", path.display()))?;
    Ok(())
}

fn replace_line(text: &str, key: &str, value: &str) -> String {
    let needle = format!("{key}=");
    let mut out = String::with_capacity(text.len() + 64);
    for line in text.lines() {
        if line.trim_start().starts_with(&needle) {
            out.push_str(&format!("{key}={value}"));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn write_conf(path: &Path, values: &[(&str, u32)]) -> Result<()> {
    let original = fs::read_to_string(path).unwrap_or_default();
    let backup = backup_path(path);
    if path.exists() && !backup.exists() {
        fs::copy(path, &backup)
            .with_context(|| format!("no se pudo hacer backup de {}", path.display()))?;
    }

    let mut out = original;
    for (key, raw) in values {
        let value = format!("0x{:08x}", raw);
        out = replace_line(&out, key, &value);
    }
    atomic_write(path, &out)?;
    Ok(())
}

fn write_toml(path: &Path, values: &[(&str, u32)]) -> Result<()> {
    let original =
        fs::read_to_string(path).with_context(|| format!("no se pudo leer {}", path.display()))?;
    let backup = backup_path(path);
    if !backup.exists() {
        fs::copy(path, &backup)
            .with_context(|| format!("no se pudo hacer backup de {}", path.display()))?;
    }

    let mut doc = original
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("no se pudo parsear TOML de {}", path.display()))?;

    for (key, raw) in values {
        let mut value = format!("0x{raw:08x}")
            .parse::<toml_edit::Value>()
            .with_context(|| format!("no se pudo formatear el color {key}"))?;

        if let Some(existing) = doc.get(*key).and_then(toml_edit::Item::as_value) {
            *value.decor_mut() = existing.decor().clone();
        }

        let explanation = match *key {
            "focus_color" => Some(
                " # Color dinámico del borde enfocado: acento Tinty (base0D) o Matugen (primary).",
            ),
            "urgent_color" => {
                Some(" # Color dinámico para ventanas urgentes: Tinty (base08) o Matugen (error).")
            }
            _ => None,
        };
        if let Some(comment) = explanation {
            let has_inline_comment = value
                .decor()
                .suffix()
                .and_then(toml_edit::RawString::as_str)
                .is_some_and(|suffix| suffix.contains('#'));
            if !has_inline_comment {
                value.decor_mut().set_suffix(comment);
            }
        }

        doc[*key] = value.clone().into();

        // Actualizar también la sección [global] si existe (MangoWM la usa para bordes).
        if let Some(global) = doc.get_mut("global").and_then(|item| item.as_table_mut()) {
            global[*key] = value.into();
        }
    }

    atomic_write(path, &doc.to_string())?;
    Ok(())
}

pub fn apply_mango_colors(values: &[(&str, u32)]) -> Result<()> {
    let path = active_path()?;
    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    if !directory.exists() {
        fs::create_dir_all(directory)
            .with_context(|| format!("no se pudo crear {}", directory.display()))?;
    }

    if path
        .extension()
        .map(|s| s.eq_ignore_ascii_case("toml"))
        .unwrap_or(false)
    {
        write_toml(&path, values)
    } else {
        write_conf(&path, values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static HOME_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn writes_and_preserves_toml_comments() {
        let _guard = HOME_LOCK.lock().unwrap();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("aspire-mango-config-{stamp}"));
        let _ = fs::remove_dir_all(&base);
        let home = base.join("home");
        fs::create_dir_all(home.join(".config/mango")).unwrap();

        let old_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }

        let path = home.join(".config/mango/config.toml");
        fs::write(
            &path,
            "# comentario global\n[global]\nroot_color = 0x010203ff\nborder_px = 2\nborder_color = 0x010203ff\n# deja esto intacto\nfocus_color = 0x040506ff\nsource = [\"./env.toml\"]\n",
        )
        .unwrap();

        apply_mango_colors(&[
            ("root_color", 0x112233ff),
            ("border_color", 0x223344ff),
            ("focus_color", 0x99aabbff),
            ("urgent_color", 0xff5555ff),
        ])
        .unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let parsed: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(
            parsed["root_color"].as_integer().unwrap(),
            287454207
        );
        assert_eq!(
            parsed["focus_color"].as_integer().unwrap(),
            2578103295
        );
        assert_eq!(
            parsed["urgent_color"].as_integer().unwrap(),
            4283782655
        );
        assert_eq!(
            parsed["border_color"].as_integer().unwrap(),
            573785343
        );
        assert_eq!(parsed["global"]["border_px"].as_integer().unwrap(), 2);
        assert!(parsed.get("borderpx").is_none());
        assert_eq!(
            parsed["global"]["source"].as_array().unwrap()[0].as_str(),
            Some("./env.toml")
        );
        assert!(text.contains("# comentario global"));
        assert!(text.contains(
            "focus_color = 0x99aabbff # Color dinámico del borde enfocado: acento Tinty (base0D) o Matugen (primary)."
        ));
        assert!(text.contains(
            "urgent_color = 0xff5555ff # Color dinámico para ventanas urgentes: Tinty (base08) o Matugen (error)."
        ));

        match old_home {
            Some(value) => unsafe {
                std::env::set_var("HOME", value);
            },
            None => unsafe {
                std::env::remove_var("HOME");
            },
        }
    }

    #[test]
    fn writes_legacy_conf_without_losing_keys() {
        let _guard = HOME_LOCK.lock().unwrap();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("aspire-mango-config-conf-{stamp}"));
        let _ = fs::remove_dir_all(&base);
        let home = base.join("home");
        fs::create_dir_all(home.join(".config/mango")).unwrap();

        let old_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &home);
        }

        let path = home.join(".config/mango/config.conf");
        fs::write(
            &path,
            "root_color=0x010203ff\nfocus_color=0x040506ff\nsource=./env.conf\n",
        )
        .unwrap();

        apply_mango_colors(&[("root_color", 0xabcdef00), ("focus_color", 0x012345ff)]).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("root_color=0xabcdef00"));
        assert!(text.contains("focus_color=0x012345ff"));
        assert!(text.contains("source=./env.conf"));

        match old_home {
            Some(value) => unsafe {
                std::env::set_var("HOME", value);
            },
            None => unsafe {
                std::env::remove_var("HOME");
            },
        }
    }
}
