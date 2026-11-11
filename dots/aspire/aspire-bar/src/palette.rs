use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const COLOR_NAMES: [&str; 7] = [
    "background",
    "background-alt",
    "foreground",
    "selected",
    "selectedtext",
    "active",
    "urgent",
];

pub const DYNAMIC_ID: &str = "dynamic";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default = "default_author")]
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub colors: BTreeMap<String, String>,
}

fn default_author() -> String {
    "Mangowm_dots".to_owned()
}

pub fn list() -> Result<Vec<Palette>> {
    let mut palettes_by_id = BTreeMap::new();
    for root in [bundled_root()?, user_root()?] {
        if !root.is_dir() {
            continue;
        }
        for entry in
            fs::read_dir(&root).with_context(|| format!("no se pudo listar {}", root.display()))?
        {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let path = entry.path();
            if entry.file_type()?.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "toml")
            {
                let palette = read(&path)?;
                palettes_by_id.insert(palette.id.clone(), palette);
            } else if entry.file_type()?.is_dir() {
                let legacy_path = path.join("palette.toml");
                if legacy_path.is_file() {
                    let mut palette = read(&legacy_path)?;
                    if palette.id.is_empty() {
                        palette.id = entry.file_name().to_string_lossy().into_owned();
                    }
                    palettes_by_id.insert(palette.id.clone(), palette);
                }
            }
        }
    }
    let mut palettes = palettes_by_id.into_values().collect::<Vec<_>>();
    palettes.sort_by_key(|palette| palette.name.to_lowercase());
    Ok(palettes)
}

pub fn read(path: &Path) -> Result<Palette> {
    let source =
        fs::read_to_string(path).with_context(|| format!("no se pudo leer {}", path.display()))?;
    let mut palette: Palette = toml::from_str(&source)
        .with_context(|| format!("paleta TOML inválida en {}", path.display()))?;
    if palette.id.is_empty() {
        palette.id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .context("el archivo de paleta no tiene un nombre UTF-8")?
            .to_owned();
    }
    validate(&palette)?;
    Ok(palette)
}

pub fn find(name: &str) -> Result<Palette> {
    list()?
        .into_iter()
        .find(|palette| palette.id == name || palette.name == name)
        .with_context(|| format!("no existe la paleta «{name}»"))
}

pub fn find_by_id(id: &str) -> Result<Palette> {
    find(id)
}

pub fn save_custom(palette: &Palette) -> Result<PathBuf> {
    validate(palette)?;
    let slug = slug(&palette.name)?;
    if find(&slug).is_ok() {
        bail!("ya existe una paleta con el identificador «{slug}»");
    }
    let target = user_root()?.join(format!("{slug}.toml"));
    let mut palette = palette.clone();
    palette.id = slug;
    let text = toml::to_string_pretty(&palette).context("no se pudo serializar la paleta")?;
    let parent = target
        .parent()
        .context("la ruta de paleta no tiene directorio")?;
    fs::create_dir_all(parent).with_context(|| format!("no se pudo crear {}", parent.display()))?;
    fs::write(&target, text).with_context(|| format!("no se pudo guardar {}", target.display()))?;
    Ok(target)
}

pub fn validate(palette: &Palette) -> Result<()> {
    if palette.name.trim().is_empty() {
        bail!("la paleta necesita un nombre");
    }
    for name in COLOR_NAMES {
        let color = palette
            .colors
            .get(name)
            .with_context(|| format!("falta el color canónico «{name}»"))?;
        if !valid_hex(color) {
            bail!("el color «{name}» debe ser #RRGGBB");
        }
    }
    Ok(())
}

fn valid_hex(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn bundled_root() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no se pudo determinar HOME")?;
    Ok(home.join(".dots/assets/waybar-palettes/palettes"))
}

fn user_root() -> Result<PathBuf> {
    let config = dirs::config_dir().context("no se pudo determinar XDG_CONFIG_HOME")?;
    Ok(config.join("aspire-bar/palettes"))
}

fn slug(name: &str) -> Result<String> {
    let slug = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() || slug.len() > 64 {
        bail!("nombre de paleta inválido");
    }
    Ok(slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> Palette {
        Palette {
            id: "test".to_owned(),
            name: "Test".to_owned(),
            author: "Test".to_owned(),
            description: String::new(),
            colors: COLOR_NAMES
                .into_iter()
                .map(|name| (name.to_owned(), "#112233".to_owned()))
                .collect(),
        }
    }

    #[test]
    fn requires_all_canonical_colors_and_hex_values() {
        validate(&example()).unwrap();
        let mut invalid = example();
        invalid.colors.insert("urgent".to_owned(), "red".to_owned());
        assert!(validate(&invalid).is_err());
        invalid.colors.remove("urgent");
        assert!(validate(&invalid).is_err());
    }

    #[test]
    fn slug_is_safe_and_stable() {
        assert_eq!(slug("Tokyo Night").unwrap(), "tokyo-night");
        assert!(slug("...").is_err());
    }

    #[test]
    fn canonical_palette_files_load_by_id_and_expose_all_roles() {
        for id in [
            "catppuccin-mocha",
            "catppuccin-latte",
            "tokyo-night",
            "gruvbox-dark",
            "dracula",
            "nord",
            "rose-pine",
            "everforest",
        ] {
            let palette = find_by_id(id).unwrap();
            assert_eq!(palette.id, id);
            validate(&palette).unwrap();
        }
    }

    #[test]
    fn bundled_palettes_keep_text_and_status_states_legible() {
        for id in [
            "catppuccin-mocha",
            "catppuccin-latte",
            "tokyo-night",
            "gruvbox-dark",
            "dracula",
            "nord",
            "rose-pine",
            "everforest",
        ] {
            let palette = find_by_id(id).unwrap();
            let color = |name: &str| palette.colors.get(name).unwrap();
            assert!(
                contrast_ratio(color("foreground"), color("background")) >= 4.5,
                "{id}: texto sobre fondo"
            );
            assert!(
                contrast_ratio(color("selected"), color("background")) >= 4.5,
                "{id}: estado warning"
            );
            assert!(
                contrast_ratio(color("foreground"), color("background-alt")) >= 4.5,
                "{id}: texto de estados críticos"
            );
            assert!(
                contrast_ratio(color("selectedtext"), color("active")) >= 4.5,
                "{id}: texto del estado activo"
            );
            assert!(
                contrast_ratio(color("urgent"), color("background")) >= 3.0,
                "{id}: borde de estado urgente"
            );
        }
    }

    fn contrast_ratio(left: &str, right: &str) -> f64 {
        let left = relative_luminance(left);
        let right = relative_luminance(right);
        let (lighter, darker) = if left >= right {
            (left, right)
        } else {
            (right, left)
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(color: &str) -> f64 {
        let channels = (1..=5)
            .step_by(2)
            .map(|index| u8::from_str_radix(&color[index..index + 2], 16).unwrap() as f64 / 255.0)
            .map(|channel| {
                if channel <= 0.04045 {
                    channel / 12.92
                } else {
                    ((channel + 0.055) / 1.055).powf(2.4)
                }
            })
            .collect::<Vec<_>>();
        0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
    }
}
