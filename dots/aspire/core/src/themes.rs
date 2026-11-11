//! Paquetes de tema (`aspire-themes`): descubrimiento, resolución de capas,
//! aplicación y reversión. Lógica compartida por `aspire-themes`, `aspire-bg`
//! y el menú; nadie reimplementa nada.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeFont {
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeIcons {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeManifest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub tinty_id: Option<String>,
    pub font: Option<ThemeFont>,
    pub icons: Option<ThemeIcons>,
    pub default_wallpaper: Option<String>,
    #[serde(default)]
    pub wallpaper: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ThemePackage {
    pub id: String,
    pub root: PathBuf,
    pub manifest: ThemeManifest,
}

fn home() -> Result<PathBuf> {
    dirs::home_dir().context("no se pudo obtener HOME")
}

pub fn themes_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(h) = home() {
        roots.push(h.join(".dots/assets/themes"));
        roots.push(h.join(".local/share/aspire/themes"));
    }
    roots
}

/// Descubre paquetes en ambas raíces. Si un id está repetido, gana el del
/// usuario (la segunda raíz).
pub fn discover_themes() -> Vec<ThemePackage> {
    let roots = themes_roots();
    let mut by_id: std::collections::BTreeMap<String, ThemePackage> =
        std::collections::BTreeMap::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(id) = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            let manifest_path = path.join("theme.toml");
            let Ok(text) = fs::read_to_string(&manifest_path) else {
                continue;
            };
            let Ok(manifest): Result<ThemeManifest, _> = toml::from_str(&text) else {
                continue;
            };
            by_id.insert(
                id.clone(),
                ThemePackage {
                    id,
                    root: path,
                    manifest,
                },
            );
        }
    }
    by_id.into_values().collect()
}

pub fn find_theme(id: &str) -> Option<ThemePackage> {
    discover_themes().into_iter().find(|p| p.id == id)
}

/// Resuelve la ruta del esquema de color para un paquete.
/// Prioridad: `scheme.yaml` del paquete; si no existe, `tinty_id` en
/// `~/.local/share/tinted-theming/tinty/scheme-repos/schemes/<sistema>/<nombre>.yaml`.
pub fn resolve_scheme_path(pkg: &ThemePackage) -> Option<PathBuf> {
    let local = pkg.root.join("scheme.yaml");
    if local.is_file() {
        return Some(local);
    }
    let tinty_id = pkg.manifest.tinty_id.as_ref()?;
    let system = if pkg.manifest.system.is_empty() {
        "base24"
    } else {
        pkg.manifest.system.as_str()
    };
    let name = tinty_id
        .strip_prefix(&format!("{system}-"))
        .unwrap_or(tinty_id);
    let home = home().ok()?;
    let path = home
        .join(".local/share/tinted-theming/tinty/scheme-repos/schemes")
        .join(system)
        .join(format!("{name}.yaml"));
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Estado nuevo: font-global / icons-global (un tema nunca los sobrescribe)
// ---------------------------------------------------------------------------

pub fn font_global() -> Option<String> {
    crate::state::font_global()
}

pub fn save_font_global(value: &str) -> Result<()> {
    crate::state::save_font_global(value)
}

pub fn icons_global() -> Option<String> {
    crate::state::icons_global()
}

pub fn save_icons_global(value: &str) -> Result<()> {
    crate::state::save_icons_global(value)
}

// ---------------------------------------------------------------------------
// Aplicar / revertir
// ---------------------------------------------------------------------------

pub fn apply_package(pkg: &ThemePackage) -> Result<()> {
    use crate::theme_engine::{self, Engine};

    // 0. Revertir overrides del tema anterior (idempotente).
    revert_theme_assets();

    // 1. Si el motor es matugen, cambiarlo a tinty.
    if theme_engine::active_engine() == Engine::Matugen {
        theme_engine::set_active_engine(Engine::Tinty)?;
    }

    let system = if pkg.manifest.system.is_empty() {
        "base24".to_string()
    } else {
        pkg.manifest.system.clone()
    };

    let scheme_src = resolve_scheme_path(pkg);
    let scheme_name = format!("{system}-aspire-{}", pkg.id);

    // Enlazar scheme.yaml si existe (compatibilidad).
    if let Some(src) = &scheme_src {
        let custom_dir = home()?
            .join(".local/share/tinted-theming/tinty/custom-schemes")
            .join(&system);
        fs::create_dir_all(&custom_dir)
            .with_context(|| format!("no se pudo crear {}", custom_dir.display()))?;
        let dest = custom_dir.join(format!("aspire-{}.yaml", pkg.id));
        let _ = fs::remove_file(&dest);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(src, &dest)
                .with_context(|| format!("no se pudo enlazar {}", dest.display()))?;
        }
    }

    // Paleta efectiva: colors.toml manda; esquema solo rellena lo que falte.
    let mut palette: Option<theme_engine::Palette> = None;
    if let Some(src) = &scheme_src {
        if let Ok(text) = fs::read_to_string(src) {
            let slots = theme_engine::parse_tinty_yaml_slots(&text);
            if !slots.is_empty() {
                palette = theme_engine::Palette::from_tinty_slots(&slots);
            }
        }
    }
    let colors_toml = pkg.root.join("colors.toml");
    if colors_toml.is_file() {
        let ct = theme_engine::palette_from_colors_toml(&colors_toml)?;
        let merged = if let Some(base) = palette {
            let mut ansi = base.ansi;
            for (i, color) in ansi.iter_mut().enumerate() {
                if ct.ansi[i].is_some() {
                    *color = ct.ansi[i];
                }
            }
            theme_engine::Palette {
                background: ct.background,
                background_alt: ct.background_alt,
                foreground: ct.foreground,
                selected: ct.selected,
                selectedtext: ct.selectedtext,
                active: ct.active,
                urgent: ct.urgent,
                ansi,
            }
        } else {
            ct
        };
        palette = Some(merged);
    }
    let palette =
        palette.ok_or_else(|| anyhow::anyhow!("no se pudo resolver la paleta del tema"))?;
    palette.write(theme_engine::Engine::Tinty)?;
    theme_engine::write_tinty_terminals_from_palette(&palette)?;
    palette.apply_system()?;
    theme_engine::save_tinty_scheme(&scheme_name)?;

    // 3. Fuente.
    crate::fonts::apply(pkg)?;
    // global/system: solo referencia; no se toca font-global.

    // 4. Iconos.
    crate::icons::apply(pkg)?;

    // 5. Fondos: aplicar un fondo del tema con aspire-bg, sin recolorear.
    let wallpapers_dir = pkg.root.join("wallpapers");
    let exts = [
        "png", "jpg", "jpeg", "webp", "bmp", "gif", "mp4", "m4v", "webm", "mkv", "mov", "avi",
        "ogv",
    ];
    let files: Vec<PathBuf> = fs::read_dir(&wallpapers_dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .filter(|p| {
                    // ignorar ocultos
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with('.') {
                        return false;
                    }
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| exts.contains(&e.to_lowercase().as_str()))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    if !files.is_empty() {
        let pick_random = || {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as usize)
                .unwrap_or(0);
            files[nanos % files.len()].clone()
        };
        let chosen = match pkg.manifest.wallpaper.as_deref() {
            Some("random") => pick_random(),
            Some(name) => {
                let p = wallpapers_dir.join(name);
                if p.is_file() {
                    p
                } else {
                    pick_random()
                }
            }
            None => {
                if let Some(dw) = &pkg.manifest.default_wallpaper {
                    let p = wallpapers_dir.join(dw);
                    if p.is_file() {
                        p
                    } else {
                        pick_random()
                    }
                } else {
                    pick_random()
                }
            }
        };
        // Aplicar con swaybg (módulo wallpaper de core, sin proceso aspire-bg).
        let _ = crate::wallpaper::set_wallpaper_swaybg(&chosen, "fill");
        let _ = crate::state::save_last_wallpaper(&chosen);
    }

    Ok(())
}

/// Limpia los overrides del tema (fuentes/iconos enlazados) sin tocar
/// font-global/icons-global.
pub fn revert_theme_assets() {
    crate::fonts::revert();
    crate::icons::revert();
}

/// Normaliza un id de tema a partir del valor guardado en tinty-theme.
/// Acepta "base24-dracula" y "base24-aspire-dracula"; devuelve "dracula".
pub fn normalize_theme_id(value: &str) -> String {
    let after_system = value.split_once('-').map(|x| x.1).unwrap_or(value);
    after_system
        .strip_prefix("aspire-")
        .unwrap_or(after_system)
        .to_string()
}

/// Aplica un tema por su id (normalizado). Si no hay paquete, cae a esquema tinty suelto.
pub fn apply_theme_by_id(id: &str) -> Result<()> {
    let normalized = normalize_theme_id(id);
    if let Some(pkg) = find_theme(&normalized) {
        apply_package(&pkg)
    } else {
        crate::theme_engine::apply_tinty_scheme(id).map(|_| ())
    }
}

/// Compatibilidad: aplica un paquete ya cargado.
pub fn apply_theme(pkg: &ThemePackage) -> Result<()> {
    apply_package(pkg)
}

/// Reaplica las elecciones globales (fuentes/iconos) tras pasar a matugen.
/// Limpia overrides y restaura el tema de iconos GTK a icons-global (o Papirus).
pub fn revert_to_global() {
    crate::fonts::revert();
    crate::icons::revert_to_global();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_nothing_on_missing_roots() {
        // No debe panicar aunque no existan las raíces.
        let _ = discover_themes();
    }

    #[test]
    fn font_global_roundtrip() {
        // Usar un HOME temporal para no tocar el real.
        let tmp = std::env::temp_dir().join("aspire-themes-test");
        let _ = std::fs::create_dir_all(&tmp);
        std::env::set_var("HOME", &tmp);
        save_font_global("JetBrainsMono Nerd Font").unwrap();
        assert_eq!(font_global().as_deref(), Some("JetBrainsMono Nerd Font"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

