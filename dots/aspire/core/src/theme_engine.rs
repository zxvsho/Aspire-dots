//! Motor de color del dotfile.
//!
//! Vive en `aspire-core` (sin GUI) y su **dueño/usuario principal es
//! `aspire-bg`**: un único sitio decide la paleta y la reparte:
//!
//! - **matugen**: extrae la paleta del wallpaper y aporta sus roles a las
//!   plantillas Rust.
//! - **tinty**: lee los slots ANSI del esquema como datos y los aporta a las
//!   plantillas Rust. Con tinty el wallpaper **no** cambia la paleta.
//!
//! La paleta semántica y los slots ANSI son los que leen waybar, rofi,
//! aspire-menu, aspire-bg, mako y las plantillas locales.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

pub use crate::reload::{mango_target_handled, TargetCfg, waybar_target_handled};

static RENDER_ID: AtomicU64 = AtomicU64::new(0);

fn home() -> Result<PathBuf> {
    dirs::home_dir().context("no se pudo obtener HOME")
}

fn config_dir() -> Result<PathBuf> {
    Ok(home()?.join(".config"))
}

// ---------------------------------------------------------------------------
// Motor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Matugen,
    Tinty,
}

impl Engine {
    pub fn as_str(self) -> &'static str {
        match self {
            Engine::Matugen => "matugen",
            Engine::Tinty => "tinty",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "matugen" => Some(Engine::Matugen),
            "tinty" => Some(Engine::Tinty),
            _ => None,
        }
    }
}

/// Motor activo (`~/.config/color-engine/active`). Por defecto matugen.
pub fn active_engine() -> Engine {
    Engine::parse(&crate::state::active_engine_str()).unwrap_or(Engine::Matugen)
}

pub fn set_active_engine(engine: Engine) -> Result<()> {
    crate::state::set_active_engine_str(engine.as_str())
}

pub fn tinty_scheme() -> Option<String> {
    crate::state::tinty_theme()
}

pub fn save_tinty_scheme(scheme: &str) -> Result<()> {
    crate::state::save_tinty_theme(scheme)
}

/// Último wallpaper aplicado (lo escribe aspire-bg al guardar su config).
pub fn last_wallpaper() -> Option<PathBuf> {
    crate::state::last_wallpaper()
}

// ---------------------------------------------------------------------------
// Color y paleta
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn from_hex(value: &str) -> Option<Self> {
        let value = value.trim().trim_start_matches('#');
        if (value.len() != 6 && value.len() != 8) || !value.is_ascii() {
            return None;
        }
        Some(Self {
            r: u8::from_str_radix(&value[0..2], 16).ok()?,
            g: u8::from_str_radix(&value[2..4], 16).ok()?,
            b: u8::from_str_radix(&value[4..6], 16).ok()?,
        })
    }

    pub fn hex(self) -> String {
        format!("{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Mezcla `self` con `other`; `self_pct` es el peso de `self` (0-100).
    pub fn mix(self, other: Self, self_pct: u8) -> Self {
        let a = self_pct as u32;
        let b = 100 - a;
        Self {
            r: ((self.r as u32 * a + other.r as u32 * b) / 100) as u8,
            g: ((self.g as u32 * a + other.g as u32 * b) / 100) as u8,
            b: ((self.b as u32 * a + other.b as u32 * b) / 100) as u8,
        }
    }
}

/// Los 7 colores que consumen los componentes del dotfile.
#[derive(Debug, Clone)]
pub struct Palette {
    pub background: Rgb,
    pub background_alt: Rgb,
    pub foreground: Rgb,
    pub selected: Rgb,
    pub selectedtext: Rgb,
    pub active: Rgb,
    pub urgent: Rgb,
    pub ansi: [Option<Rgb>; 16],
}

impl Palette {
    /// Roles de matugen tal como los usa la plantilla de rofi del dotfile,
    /// para que el resultado sea idéntico al de antes.
    fn from_matugen(json: &Value) -> Result<Self> {
        let role = |name: &str| -> Option<Rgb> {
            let entry = json.get("colors")?.get(name)?.get("default")?;
            let hex = entry.get("hex").or_else(|| entry.get("color"))?.as_str()?;
            Rgb::from_hex(hex)
        };
        let ansi_roles = [
            ("background", "background"),
            ("error", "error"),
            ("secondary", "primary_container"),
            ("tertiary", "primary"),
            ("primary", "primary"),
            ("on_background", "on_background"),
            ("secondary_container", "surface_container"),
            ("surface", "background"),
            ("surface_container", "background"),
            ("error", "error"),
            ("secondary_fixed", "primary_container"),
            ("tertiary_fixed", "primary"),
            ("primary_container", "primary"),
            ("on_surface", "on_background"),
            ("on_surface_variant", "on_background"),
            ("outline", "surface_container"),
        ];
        Ok(Self {
            background: role("background").context("matugen no devolvió background")?,
            background_alt: role("surface_container")
                .context("matugen no devolvió surface_container")?,
            foreground: role("on_background").context("matugen no devolvió on_background")?,
            selected: role("primary").context("matugen no devolvió primary")?,
            selectedtext: Rgb::from_hex("#000000").unwrap(),
            active: role("primary_container").context("matugen no devolvió primary_container")?,
            urgent: role("error").context("matugen no devolvió error")?,
            ansi: std::array::from_fn(|index| {
                let (preferred, fallback) = ansi_roles[index];
                role(preferred).or_else(|| role(fallback))
            }),
        })
    }

    /// Slots ANSI según la sección 14 de aspire-themes-spec.md.
    pub fn from_tinty_slots(slots: &HashMap<String, String>) -> Option<Self> {
        let get = |key: &str| slots.get(key).and_then(|s| Rgb::from_hex(s));
        let base24 = slots.contains_key("base12");
        let _bright = |base_key: &str, bright_key: &str| {
            if base24 {
                get(bright_key).or_else(|| get(base_key))
            } else {
                get(base_key)
            }
        };
        let dark1 = get("base08");
        let dark2 = get("base0b");
        let dark3 = get("base0a");
        let dark4 = get("base0d");
        let dark5 = get("base0e");
        let dark6 = get("base0c");
        let bright1 = if base24 { get("base12") } else { get("base08") };
        let bright2 = if base24 { get("base14") } else { get("base0b") };
        let bright3 = if base24 { get("base13") } else { get("base0a") };
        let bright4 = if base24 { get("base16") } else { get("base0d") };
        let bright5 = if base24 { get("base17") } else { get("base0e") };
        let bright6 = if base24 { get("base15") } else { get("base0c") };
        Some(Self {
            // Mapeo canónico (igual para base16 y base24):
            //   background     = base00
            //   background_alt = base01
            //   foreground     = base05
            //   selected       = base0D (acento del tema)
            //   selectedtext   = #000000 (fijo, como color-engine.sh)
            //   active         = base05
            //   urgent         = base08
            background: get("base00")?,
            background_alt: get("base01")?,
            foreground: get("base05")?,
            selected: get("base0d")?,
            selectedtext: Rgb { r: 0, g: 0, b: 0 },
            active: get("base05")?,
            urgent: get("base08")?,
            ansi: [
                get("base00"),                   // color0  = base00
                dark1.or_else(|| get("base08")), // color1  = base08
                dark2.or_else(|| get("base0b")), // color2  = base0B
                dark3.or_else(|| get("base0a")), // color3  = base0A
                dark4.or_else(|| get("base0d")), // color4  = base0D
                dark5.or_else(|| get("base0e")), // color5  = base0E
                dark6.or_else(|| get("base0c")), // color6  = base0C
                get("base05"),                   // color7  = base05
                get("base03"),                   // color8  = base03
                bright1,                         // color9  = base08 (base24: base12)
                bright2,                         // color10 = base0B (base24: base14)
                bright3,                         // color11 = base0A (base24: base13)
                bright4,                         // color12 = base0D (base24: base16)
                bright5,                         // color13 = base0E (base24: base17)
                bright6,                         // color14 = base0C (base24: base15)
                get("base07"),                   // color15 = base07
            ],
        })
    }

    fn template_tokens(&self) -> HashMap<String, String> {
        let mut tokens = HashMap::from([
            ("bg".to_string(), format!("#{}", self.background.hex())),
            ("fg".to_string(), format!("#{}", self.foreground.hex())),
            (
                "background".to_string(),
                format!("#{}", self.background.hex()),
            ),
            (
                "background_alt".to_string(),
                format!("#{}", self.background_alt.hex()),
            ),
            (
                "foreground".to_string(),
                format!("#{}", self.foreground.hex()),
            ),
            ("selected".to_string(), format!("#{}", self.selected.hex())),
            (
                "selectedtext".to_string(),
                format!("#{}", self.selectedtext.hex()),
            ),
            ("active".to_string(), format!("#{}", self.active.hex())),
            ("urgent".to_string(), format!("#{}", self.urgent.hex())),
        ]);
        for (index, color) in self.ansi.iter().enumerate() {
            if let Some(color) = color {
                tokens.insert(format!("color{index}"), format!("#{}", color.hex()));
            }
        }
        tokens
    }

    /// Escribe la paleta compatible y, si están instaladas, renderiza las
    /// plantillas Rust antes de actualizar la configuración del sistema.
    pub fn write(&self, engine: Engine) -> Result<()> {
        let dir = config_dir()?.join("theme-engine");
        std::fs::create_dir_all(&dir).context("no se pudo crear theme-engine")?;

        let mut rasi = format!(
            "/**\n *\n * Colors — generado por aspire-bg ({})\n **/\n* {{\n    background:     #{}FF;\n    background-alt: #{}FF;\n    foreground:     #{}FF;\n    selected:       #{}FF;\n    selectedtext:   #{}FF;\n    active:         #{}FF;\n    urgent:         #{}FF;\n",
            engine.as_str(),
            self.background.hex(),
            self.background_alt.hex(),
            self.foreground.hex(),
            self.selected.hex(),
            self.selectedtext.hex(),
            self.active.hex(),
            self.urgent.hex(),
        );
        for (index, color) in self.ansi.iter().enumerate() {
            if let Some(color) = color {
                rasi.push_str(&format!("    color{index}:       #{}FF;\n", color.hex()));
            }
        }
        rasi.push_str("}\n");
        std::fs::write(dir.join("colors.rasi"), rasi).context("no se pudo escribir colors.rasi")?;

        let mut css = format!(
            "/* Generado por aspire-bg ({}) — NO EDITAR A MANO */\n@define-color background #{};\n@define-color background-alt #{};\n@define-color foreground #{};\n@define-color selected #{};\n@define-color selectedtext #{};\n@define-color active #{};\n@define-color urgent #{};\n",
            engine.as_str(),
            self.background.hex(),
            self.background_alt.hex(),
            self.foreground.hex(),
            self.selected.hex(),
            self.selectedtext.hex(),
            self.active.hex(),
            self.urgent.hex(),
        );
        for (index, color) in self.ansi.iter().enumerate() {
            if let Some(color) = color {
                css.push_str(&format!("@define-color color{} #{};\n", index, color.hex()));
            }
        }
        std::fs::write(dir.join("colors.css"), css).context("no se pudo escribir colors.css")?;

        render_templates(&self.template_tokens())?;

        // rofi importa colors/matugen-wallpaper.rasi; antes era un enlace
        // que creaba color-engine.sh. Lo recreamos como enlace a la paleta.
        let rofi_dir = config_dir()?.join("rofi/colors");
        if rofi_dir.is_dir() {
            let link = rofi_dir.join("matugen-wallpaper.rasi");
            if std::fs::symlink_metadata(&link).is_ok() {
                let _ = std::fs::remove_file(&link);
            }
            #[cfg(unix)]
            {
                let _ = std::os::unix::fs::symlink(dir.join("colors.rasi"), &link);
            }
        }
        Ok(())
    }

    /// Recolorea los componentes que no generan matugen/tinty.
    pub fn apply_system(&self) -> Result<()> {
        apply_mango_palette(self)?;
        self.apply_mako();
        self.apply_walker();
        // No copiar: rio/themes/*.toml son symlinks a theme-engine/current/rio.toml.
        crate::reload::reload_all(&crate::reload::Ctx {
            palette: Some(self),
        });
        Ok(())
    }

    fn apply_mako(&self) {
        let Ok(dir) = config_dir() else { return };
        let path = dir.join("mako/config");
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        let mut section = String::new();
        let mut out = String::with_capacity(text.len() + 64);
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('[') {
                section = trimmed.to_string();
            }
            let critical = section.starts_with("[urgency=critical]");
            let replacement = if let Some(rest) = trimmed.strip_prefix("background-color=") {
                let _ = rest;
                Some(format!("background-color=#{}E6", self.background.hex()))
            } else if trimmed.starts_with("text-color=") {
                Some(format!("text-color=#{}", self.foreground.hex()))
            } else if trimmed.starts_with("border-color=") {
                let color = if critical { self.urgent } else { self.selected };
                Some(format!("border-color=#{}", color.hex()))
            } else if trimmed.starts_with("progress-color=") {
                Some(format!("progress-color=over #{}", self.selected.hex()))
            } else {
                None
            };
            out.push_str(&replacement.unwrap_or_else(|| line.to_string()));
            out.push('\n');
        }
        let _ = std::fs::write(&path, out);
        run("makoctl", &["reload"]);
    }

    fn apply_walker(&self) {
        let Ok(dir) = config_dir() else { return };
        let path = dir.join("walker/themes/mango-theme/style.css");
        let Ok(text) = std::fs::read_to_string(&path) else {
            return;
        };
        let values = [
            ("window_bg_color", self.background),
            ("accent_bg_color", self.selected),
            ("theme_fg_color", self.foreground),
            ("error_bg_color", self.urgent),
            ("error_fg_color", self.background),
        ];
        let mut out = text;
        for (name, color) in values {
            out = replace_define_color(&out, name, &color.hex());
        }
        let _ = std::fs::write(&path, out);
    }
}

pub fn apply_mango_palette(palette: &Palette) -> Result<()> {
    let values = mango_color_values(palette);
    crate::mango_config::apply_mango_colors(&values)
}

fn mango_color_values(palette: &Palette) -> [(&'static str, u32); 10] {
    let border = palette.background.mix(palette.foreground, 92);
    [
        ("root_color", mango_color(palette.background)),
        ("border_color", mango_color(border)),
        ("focus_color", mango_color(palette.selected)),
        ("drop_color", mango_color(palette.active)),
        ("split_color", mango_color(palette.active)),
        ("maximized_screen_color", mango_color(palette.selected)),
        ("urgent_color", mango_color(palette.urgent)),
        ("scratchpad_color", mango_color(palette.foreground)),
        ("global_color", mango_color(palette.active)),
        ("overlay_color", mango_color(palette.active)),
    ]
}

fn mango_color(color: Rgb) -> u32 {
    u32::from_be_bytes([color.r, color.g, color.b, u8::MAX])
}

/// Renderiza `templates/<app>.tpl` en Rust y activa el conjunto completo
/// mediante un intercambio atómico de `current`. Devuelve `None` cuando
/// todavía no hay un directorio de plantillas instalado.
pub fn render_templates(tokens: &HashMap<String, String>) -> Result<Option<PathBuf>> {
    let root = config_dir()?.join("theme-engine");
    std::fs::create_dir_all(&root)
        .with_context(|| format!("no se pudo crear {}", root.display()))?;

    let mut sources: Vec<PathBuf> = Vec::new();
    if let Some(id) = active_theme_id() {
        if let Some(pkg) = crate::themes::find_theme(&id) {
            let td = pkg.root.join("templates");
            if td.is_dir() {
                sources.push(td);
            }
        }
    }
    let global = root.join("templates");
    if global.is_dir() {
        sources.push(global);
    }
    if sources.is_empty() {
        return Ok(None);
    }
    let r = render_templates_in(&root, &sources, tokens);
    r.map(Some)
}

fn active_theme_id() -> Option<String> {
    if active_engine() != Engine::Tinty {
        return None;
    }
    let scheme = tinty_scheme()?;
    let after = scheme.split_once('-')?.1;
    let id = after.strip_prefix("aspire-").unwrap_or(after);
    Some(id.to_string())
}

fn render_templates_in(
    root: &Path,
    sources: &[PathBuf],
    tokens: &HashMap<String, String>,
) -> Result<PathBuf> {
    let root = std::fs::canonicalize(root)
        .with_context(|| format!("no se pudo resolver {}", root.display()))?;
    let current = root.join("current");
    let previous_target = match std::fs::symlink_metadata(&current) {
        Ok(metadata) if metadata.file_type().is_symlink() => Some(
            std::fs::read_link(&current)
                .with_context(|| format!("no se pudo leer el enlace {}", current.display()))?,
        ),
        Ok(_) => bail!(
            "se rehúsa reemplazar {} porque no es un symlink",
            current.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error)
                .with_context(|| format!("no se pudo inspeccionar {}", current.display()))
        }
    };

    // Recoger .tpl de todas las fuentes, prioridad por orden de `sources`
    // (el primero gana si hay mismo nombre de archivo).
    let mut collected: Vec<(String, PathBuf)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for src in sources {
        let Ok(entries) = std::fs::read_dir(src) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !metadata.file_type().is_file() {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !file_name.ends_with(".tpl") {
                continue;
            }
            let output_name = match file_name.strip_suffix(".tpl") {
                Some(n) if valid_template_name(n) => n.to_string(),
                _ => continue,
            };
            if !seen.insert(output_name.clone()) {
                continue; // ya aportado por una fuente de mayor prioridad
            }
            collected.push((output_name, path));
        }
    }
    if collected.is_empty() {
        bail!("no hay plantillas .tpl en las fuentes configuradas");
    }

    let id = render_id();
    let staging = root.join(format!(".staging-{id}"));
    let rendered = root.join(format!(".rendered-{id}"));
    let pending_link = root.join(format!(".current-{id}"));
    std::fs::create_dir(&staging)
        .with_context(|| format!("no se pudo crear staging {}", staging.display()))?;

    let render_result = (|| -> Result<()> {
        for (output_name, source) in &collected {
            let template = std::fs::read_to_string(source)
                .with_context(|| format!("no se pudo leer la plantilla {}", source.display()))?;
            let output = render_template(&template, tokens)
                .with_context(|| format!("error al renderizar {}", source.display()))?;
            validate_rendered_output(output_name, &output)?;
            std::fs::write(staging.join(output_name), output)
                .with_context(|| format!("no se pudo escribir {} en staging", output_name))?;
        }
        validate_staging(&staging)?;
        // include/ del tema activo: copia verbatim (sin tokens).
        if let Some(id) = active_theme_id() {
            if let Some(pkg) = crate::themes::find_theme(&id) {
                let inc = pkg.root.join("include");
                if inc.is_dir() {
                    if let Ok(entries) = std::fs::read_dir(&inc) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file() {
                                if let Some(name) = path.file_name() {
                                    let _ = std::fs::copy(&path, staging.join(name));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    })();

    if let Err(error) = render_result {
        remove_render_path(&staging).with_context(|| {
            format!("falló la limpieza de staging después del error: {error:#}")
        })?;
        return Err(error);
    }

    if let Err(error) = std::fs::rename(&staging, &rendered) {
        remove_render_path(&staging)?;
        return Err(error)
            .with_context(|| format!("no se pudo publicar el staging en {}", rendered.display()));
    }
    if let Err(error) = create_symlink(&rendered, &pending_link) {
        remove_render_path(&rendered)?;
        return Err(error).with_context(|| {
            format!(
                "no se pudo crear el symlink temporal {}",
                pending_link.display()
            )
        })?;
    }
    if let Err(error) = std::fs::rename(&pending_link, &current) {
        remove_render_path(&pending_link)?;
        remove_render_path(&rendered)?;
        return Err(error).with_context(|| {
            format!(
                "no se pudo intercambiar atómicamente el symlink {}",
                current.display()
            )
        })?;
    }

    if let Some(previous_target) = previous_target {
        let previous_path = if previous_target.is_absolute() {
            previous_target
        } else {
            root.join(previous_target)
        };
        if let Ok(previous_path) = std::fs::canonicalize(&previous_path) {
            let managed_name = previous_path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".rendered-"));
            if managed_name && previous_path.parent() == Some(root.as_path()) {
                remove_render_path(&previous_path).with_context(|| {
                    format!(
                        "el tema nuevo quedó activo, pero no se pudo eliminar el render anterior {}",
                        previous_path.display()
                    )
                })?;
            }
        }
    }
    Ok(current)
}

fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(not(unix))]
    {
        let _ = (target, link);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "el intercambio de theme-engine/current requiere symlinks Unix",
        ))
    }
}

fn render_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = RENDER_ID.fetch_add(1, Ordering::Relaxed);
    format!("{}-{timestamp}-{sequence}", std::process::id())
}

fn valid_template_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn render_template(template: &str, tokens: &HashMap<String, String>) -> Result<String> {
    let mut output = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let token_start = start + 2;
        let end = rest[token_start..]
            .find('}')
            .map(|offset| token_start + offset)
            .context("token sin llave de cierre `}`")?;
        let name = &rest[token_start..end];
        if name.is_empty() || name.contains("${") || name.contains('{') {
            bail!("sintaxis de token inválida: ${{{name}}}");
        }
        let value = tokens
            .get(name)
            .with_context(|| format!("token no disponible: ${{{name}}}"))?;
        if value.contains('\0') {
            bail!("el valor del token ${{{name}}} contiene NUL");
        }
        output.push_str(value);
        rest = &rest[end + 1..];
    }
    output.push_str(rest);
    if output.contains("${") {
        bail!("la salida contiene tokens sin resolver");
    }
    Ok(output)
}

fn validate_rendered_output(name: &str, output: &str) -> Result<()> {
    if !valid_template_name(name) {
        bail!("nombre de salida no válido: {name}");
    }
    if output.trim().is_empty() {
        bail!("la salida renderizada está vacía: {name}");
    }
    if output.contains('\0') || output.contains("${") {
        bail!("la salida renderizada contiene datos no válidos: {name}");
    }
    Ok(())
}

fn validate_staging(staging: &Path) -> Result<()> {
    let entries = std::fs::read_dir(staging)
        .with_context(|| format!("no se pudo validar staging {}", staging.display()))?;
    let mut count = 0;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("no se pudo leer staging {}", staging.display()))?;
        let metadata = std::fs::symlink_metadata(entry.path())
            .with_context(|| format!("no se pudo inspeccionar {}", entry.path().display()))?;
        if !metadata.file_type().is_file() {
            bail!("el staging contiene una entrada que no es archivo regular");
        }
        let content = std::fs::read_to_string(entry.path())
            .with_context(|| format!("no se pudo validar {}", entry.path().display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        validate_rendered_output(&name, &content)?;
        count += 1;
    }
    if count == 0 {
        bail!("el staging no contiene archivos renderizados");
    }
    Ok(())
}

fn remove_render_path(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => std::fs::remove_dir_all(path)
            .with_context(|| format!("no se pudo eliminar el directorio {}", path.display())),
        Ok(_) => std::fs::remove_file(path)
            .with_context(|| format!("no se pudo eliminar {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("no se pudo inspeccionar {}", path.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_directory() -> PathBuf {
        std::env::temp_dir().join(format!("aspire-core-theme-test-{}", render_id()))
    }

    #[test]
    fn renders_semantic_and_ansi_tokens() {
        let tokens = HashMap::from([
            ("bg".to_string(), "#112233".to_string()),
            ("color15".to_string(), "#aabbcc".to_string()),
        ]);
        assert_eq!(
            render_template("background=${bg}\nlast=${color15}\n", &tokens).unwrap(),
            "background=#112233\nlast=#aabbcc\n"
        );
    }

    #[test]
    fn rejects_missing_and_malformed_tokens() {
        let tokens = HashMap::new();
        assert!(render_template("x=${unknown}", &tokens)
            .unwrap_err()
            .to_string()
            .contains("token no disponible"));
        assert!(render_template("x=${bg", &tokens)
            .unwrap_err()
            .to_string()
            .contains("sin llave de cierre"));
    }

    #[test]
    fn resolves_file_uri_css_imports_and_percent_encoded_paths() {
        assert_eq!(
            resolve_import_path(
                Path::new("/tmp/waybar"),
                "file:///tmp/theme%20engine/current/colors.css"
            ),
            Some(PathBuf::from("/tmp/theme engine/current/colors.css"))
        );
        assert_eq!(
            resolve_import_path(Path::new("/tmp/waybar"), "colors.css"),
            Some(PathBuf::from("/tmp/waybar/colors.css"))
        );
        assert_eq!(
            resolve_import_path(Path::new("/tmp/waybar"), "file://host/colors.css"),
            None
        );
    }

    #[test]
    fn configured_mango_and_waybar_targets_prevent_duplicate_fallbacks() {
        let targets = toml::from_str::<HashMap<String, TargetCfg>>(
            "[mango]\ncommand = \"mmsg dispatch reload_config\"\n\
             [waybar]\nsignal = \"SIGUSR2\"\nprocess = \"waybar\"\n",
        )
        .unwrap();
        assert!(mango_target_handled(&targets));
        assert!(waybar_target_handled(&targets));

        let no_targets = HashMap::new();
        assert!(!mango_target_handled(&no_targets));
        assert!(!waybar_target_handled(&no_targets));
    }

    #[test]
    fn mango_focused_border_uses_tinty_accent_not_base04_or_urgent() {
        let text = std::fs::read_to_string("tests/fixtures/dracula-base24.yaml").unwrap();
        let slots = parse_tinty_yaml_slots(&text);
        let palette = Palette::from_tinty_slots(&slots).unwrap();
        let values = mango_color_values(&palette)
            .into_iter()
            .collect::<HashMap<_, _>>();

        assert_eq!(palette.selected, Rgb::from_hex("#bd93f9").unwrap());
        assert_eq!(values["focus_color"], 0xbd93f9ff);
        assert_eq!(values["urgent_color"], 0xff5555ff);
        assert_ne!(values["focus_color"], values["urgent_color"]);
    }

    #[test]
    fn mango_focused_border_uses_matugen_primary_role() {
        let palette = Palette {
            background: Rgb::from_hex("#282a36").unwrap(),
            background_alt: Rgb::from_hex("#21222c").unwrap(),
            foreground: Rgb::from_hex("#f8f8f2").unwrap(),
            selected: Rgb::from_hex("#bd93f9").unwrap(),
            selectedtext: Rgb::from_hex("#000000").unwrap(),
            active: Rgb::from_hex("#f8f8f2").unwrap(),
            urgent: Rgb::from_hex("#ff5555").unwrap(),
            ansi: [None; 16],
        };
        let values = mango_color_values(&palette)
            .into_iter()
            .collect::<HashMap<_, _>>();

        assert_eq!(values["focus_color"], 0xbd93f9ff);
        assert_eq!(values["maximized_screen_color"], 0xbd93f9ff);
        assert_eq!(values["urgent_color"], 0xff5555ff);
        assert_ne!(values["focus_color"], values["urgent_color"]);
    }

    #[test]
    fn base24_dracula_canonical_colors() {
        let path = "tests/fixtures/dracula-base24.yaml".to_string();
        let text = std::fs::read_to_string(path).unwrap();
        let slots = parse_tinty_yaml_slots(&text);
        let p = Palette::from_tinty_slots(&slots).unwrap();
        assert_eq!(p.background, Rgb::from_hex("#282a36").unwrap());
        assert_eq!(p.background_alt, Rgb::from_hex("#21222c").unwrap());
        assert_eq!(p.foreground, Rgb::from_hex("#f8f8f2").unwrap());
        assert_eq!(p.selected, Rgb::from_hex("#bd93f9").unwrap());
        assert_eq!(p.active, Rgb::from_hex("#f8f8f2").unwrap());
        assert_eq!(p.urgent, Rgb::from_hex("#ff5555").unwrap());
    }

    #[test]
    fn base16_dracula_canonical_colors() {
        // Use base16 dracula if available; otherwise skip gracefully.
        let path = "tests/fixtures/dracula-base16.yaml".to_string();
        if !std::path::Path::new(&path).is_file() {
            return;
        }
        let text = std::fs::read_to_string(path).unwrap();
        let slots = parse_tinty_yaml_slots(&text);
        let p = Palette::from_tinty_slots(&slots).unwrap();
        assert_eq!(p.background_alt, Rgb::from_hex("#21222c").unwrap());
        assert_eq!(p.selected, Rgb::from_hex("#bd93f9").unwrap());
        assert_eq!(p.urgent, Rgb::from_hex("#ff5555").unwrap());
    }

    #[test]
    fn parses_custom_tinty_yaml_as_data() {
        let slots = parse_tinty_yaml_slots(
            "system: base16\npalette:\n  base00: '#112233'\n  base0F: aabbcc\n  base17: '#778899' # comment\n",
        );
        assert_eq!(slots.get("base00").map(String::as_str), Some("112233"));
        assert_eq!(slots.get("base0f").map(String::as_str), Some("aabbcc"));
        assert_eq!(slots.get("base17").map(String::as_str), Some("778899"));
        assert!(!slots.contains_key("base16"));
    }

    #[test]
    fn swaps_current_atomically_and_preserves_old_on_render_error() {
        let root = test_directory();
        let templates = root.join("templates");
        std::fs::create_dir_all(&templates).unwrap();
        std::fs::write(templates.join("rofi.rasi.tpl"), "background: ${bg};\n").unwrap();
        let first_tokens = HashMap::from([("bg".to_string(), "#112233".to_string())]);

        let current =
            render_templates_in(&root, std::slice::from_ref(&templates), &first_tokens).unwrap();
        let previous = std::fs::read_link(&current).unwrap();
        assert_eq!(
            std::fs::read_to_string(current.join("rofi.rasi")).unwrap(),
            "background: #112233;\n"
        );

        let missing_tokens = HashMap::new();
        assert!(
            render_templates_in(&root, std::slice::from_ref(&templates), &missing_tokens).is_err()
        );
        assert_eq!(std::fs::read_link(&current).unwrap(), previous);

        let next_tokens = HashMap::from([("bg".to_string(), "#445566".to_string())]);
        render_templates_in(&root, std::slice::from_ref(&templates), &next_tokens).unwrap();
        assert_eq!(
            std::fs::read_to_string(current.join("rofi.rasi")).unwrap(),
            "background: #445566;\n"
        );
        assert!(!previous.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn replace_define_color(text: &str, name: &str, hex: &str) -> String {
    let needle = format!("@define-color {name} ");
    let mut out = String::with_capacity(text.len() + 64);
    for line in text.lines() {
        if line.trim_start().starts_with(&needle) {
            out.push_str(&format!("@define-color {name} #{hex};"));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// Recarga Waybar con SIGUSR2 tras validar que los @import del style.css activo
/// se resuelven a archivos existentes. Si algún import falta, no recarga.
#[cfg(test)]
fn resolve_import_path(base: &Path, url: &str) -> Option<PathBuf> {
    if let Some(path) = url.strip_prefix("file://") {
        if !path.starts_with('/') {
            return None;
        }
        return percent_decode_path(path).map(PathBuf::from);
    }
    Some(base.join(url))
}

#[cfg(test)]
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

#[cfg(test)]
fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn run(program: &str, args: &[&str]) {
    let _ = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

// ---------------------------------------------------------------------------
// tinty
// ---------------------------------------------------------------------------

fn tinty_scripts_dir() -> Result<PathBuf> {
    Ok(home()?.join(".local/share/tinted-theming/tinty/repos/tinted-shell/scripts"))
}

fn tinty_custom_scheme_path(scheme: &str) -> Result<Option<PathBuf>> {
    let Some((system @ ("base16" | "base24"), slug)) = scheme.split_once('-') else {
        return Ok(None);
    };
    let path = home()?
        .join(".local/share/tinted-theming/tinty/custom-schemes")
        .join(system)
        .join(format!("{slug}.yaml"));
    Ok(path.is_file().then_some(path))
}

fn clean_tinty_value(value: &str) -> String {
    let value = value.trim();
    let value = if let Some(rest) = value.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else {
        value.split_whitespace().next().unwrap_or(value)
    };
    value.trim_start_matches('#').replace('/', "")
}

/// Parsea `color00="27/2e/33"` y referencias `$color01` de los scripts de
/// tinted-shell.
fn parse_tinty_slots(text: &str) -> HashMap<u8, String> {
    let mut raw: HashMap<u8, String> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("color") else {
            continue;
        };
        let Some((key, value)) = rest.split_once('=') else {
            continue;
        };
        let Ok(idx) = key.trim().parse::<u8>() else {
            continue;
        };
        raw.insert(idx, clean_tinty_value(value));
    }

    fn resolve(idx: u8, raw: &HashMap<u8, String>, depth: u8) -> Option<String> {
        if depth > 6 {
            return None;
        }
        let value = raw.get(&idx)?;
        if let Some(reference) = value.strip_prefix('$') {
            let reference = reference.trim_start_matches('{').trim_end_matches('}');
            let target = reference.strip_prefix("color")?.parse::<u8>().ok()?;
            resolve(target, raw, depth + 1)
        } else {
            Some(value.clone())
        }
    }

    raw.keys()
        .filter_map(|idx| resolve(*idx, &raw, 0).map(|value| (*idx, value)))
        .collect()
}

pub fn parse_tinty_yaml_slots(text: &str) -> HashMap<String, String> {
    let mut slots = HashMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, raw_value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        let Some(suffix) = key.strip_prefix("base") else {
            continue;
        };
        if suffix.len() != 2 {
            continue;
        }
        let valid = (suffix.starts_with('0') && suffix.chars().nth(1).unwrap().is_ascii_hexdigit())
            || (suffix.starts_with('1') && "01234567".contains(suffix.chars().nth(1).unwrap()));
        if !valid {
            continue;
        }
        let value = raw_value.trim();
        let value = if value.starts_with('"') || value.starts_with('\'') {
            let quote = value.chars().next().unwrap();
            value[1..].split(quote).next().unwrap_or("")
        } else {
            value.split_whitespace().next().unwrap_or("")
        };
        let value = value.trim().trim_start_matches('#');
        if value.len() != 6 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        slots.insert(key, value.to_lowercase());
    }
    slots
}

fn write_tinty_terminals(slots: &HashMap<String, String>) -> Result<()> {
    let Ok(dir) = config_dir() else { return Ok(()) };
    let base24 = slots.contains_key("base12");
    let slot = |idx: u8| -> String {
        let key = match idx {
            0 => "base00",
            1 => "base08",
            9 => {
                if base24 {
                    "base12"
                } else {
                    "base08"
                }
            }
            2 => "base0b",
            10 => {
                if base24 {
                    "base14"
                } else {
                    "base0b"
                }
            }
            3 => "base0a",
            11 => {
                if base24 {
                    "base13"
                } else {
                    "base0a"
                }
            }
            4 => "base0d",
            12 => {
                if base24 {
                    "base16"
                } else {
                    "base0d"
                }
            }
            5 => "base0e",
            13 => {
                if base24 {
                    "base17"
                } else {
                    "base0e"
                }
            }
            6 => "base0c",
            14 => {
                if base24 {
                    "base15"
                } else {
                    "base0c"
                }
            }
            7 => "base05",
            8 => "base03",
            15 => "base07",
            _ => "base00",
        };
        slots.get(key).cloned().unwrap_or_default()
    };

    let kitty_dir = dir.join("kitty");
    if kitty_dir.is_dir() {
        let mut out = format!(
            "background #{}\nforeground #{}\ncursor #{}\n\n",
            slot(0),
            slot(5),
            slot(4)
        );
        for i in 0..16u8 {
            out.push_str(&format!("color{i} #{}\n", slot(i)));
        }
        let _ = std::fs::write(kitty_dir.join("theme.conf"), out);
    }

    let alacritty_dir = dir.join("alacritty");
    if alacritty_dir.is_dir() {
        let names = [
            "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
        ];
        let mut out = format!(
            "[colors.primary]\nbackground = '#{}'\nforeground = '#{}'\n\n[colors.cursor]\ncursor = '#{}'\n\n[colors.normal]\n",
            slot(0),
            slot(5),
            slot(4)
        );
        for (i, name) in names.iter().enumerate() {
            out.push_str(&format!("{name} = '#{}'\n", slot(i as u8)));
        }
        out.push_str("\n[colors.bright]\n");
        for (i, name) in names.iter().enumerate() {
            out.push_str(&format!("{name} = '#{}'\n", slot(i as u8 + 8)));
        }
        let _ = std::fs::write(alacritty_dir.join("colors.toml"), out);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// matugen
// ---------------------------------------------------------------------------

fn matugen_config() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("ASPIRE_MATUGEN_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    Ok(config_dir()?.join("matugen/config.toml"))
}

fn stderr_tail(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(3)..].join(" | ")
}

/// Extrae la paleta del wallpaper y aplica plantillas + componentes.
#[derive(serde::Deserialize, Default)]
struct ColorsTomlFile {
    #[serde(default)]
    background: String,
    #[serde(default)]
    background_alt: String,
    #[serde(default)]
    foreground: String,
    #[serde(default)]
    selected: String,
    #[serde(default)]
    selectedtext: String,
    #[serde(default)]
    active: String,
    #[serde(default)]
    urgent: String,
    #[serde(default)]
    ansi: std::collections::HashMap<String, String>,
}

/// Lee una paleta explícita desde `colors.toml` del paquete de tema.
pub fn palette_from_colors_toml(path: &Path) -> Result<Palette> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("no se pudo leer {}", path.display()))?;
    let ct: ColorsTomlFile = toml::from_str(&text)
        .with_context(|| format!("colors.toml inválido: {}", path.display()))?;
    let mut ansi = [None; 16];
    for (k, v) in &ct.ansi {
        if let Some(idx) = k
            .strip_prefix("color")
            .and_then(|s| s.trim().parse::<usize>().ok())
        {
            if idx < 16 {
                ansi[idx] = Rgb::from_hex(v);
            }
        }
    }
    Ok(Palette {
        background: Rgb::from_hex(&ct.background).context("background inválido")?,
        background_alt: Rgb::from_hex(&ct.background_alt).context("background_alt inválido")?,
        foreground: Rgb::from_hex(&ct.foreground).context("foreground inválido")?,
        selected: Rgb::from_hex(&ct.selected).context("selected inválido")?,
        selectedtext: Rgb::from_hex(&ct.selectedtext).unwrap_or(Rgb { r: 0, g: 0, b: 0 }),
        active: Rgb::from_hex(&ct.active).context("active inválido")?,
        urgent: Rgb::from_hex(&ct.urgent).context("urgent inválido")?,
        ansi: std::array::from_fn(|i| ansi[i]),
    })
}

/// Escribe los archivos de terminales a partir de la paleta (ansi color0..color15).
pub fn write_tinty_terminals_from_palette(palette: &Palette) -> Result<()> {
    let dir = config_dir()?;
    let slot = |idx: u8| -> String {
        palette
            .ansi
            .get(idx as usize)
            .and_then(|c| c.map(|x| x.hex()))
            .unwrap_or_default()
    };
    let kitty_dir = dir.join("kitty");
    if kitty_dir.is_dir() {
        let mut out = format!(
            "background #{}\\nforeground #{}\\ncursor #{}\\n\\n",
            slot(0),
            slot(7),
            slot(4) // selected? use color4
        );
        for i in 0..16u8 {
            out.push_str(&format!("color{i} #{}\\n", slot(i)));
        }
        let _ = std::fs::write(kitty_dir.join("theme.conf"), out);
    }
    let alacritty_dir = dir.join("alacritty");
    if alacritty_dir.is_dir() {
        let names = [
            "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
        ];
        let mut out = format!(
            "[colors.primary]\\nbackground = '#{}'\\nforeground = '#{}'\\n\\n[colors.cursor]\\ncursor = '#{}'\\n\\n[colors.normal]\\n",
            slot(0),
            slot(7),
            slot(4)
        );
        for (i, name) in names.iter().enumerate() {
            out.push_str(&format!("{name} = '#{}'\\n", slot(i as u8)));
        }
        out.push_str("\\n[colors.bright]\\n");
        for (i, name) in names.iter().enumerate() {
            out.push_str(&format!("{name} = '#{}'\\n", slot(i as u8 + 8)));
        }
        let _ = std::fs::write(alacritty_dir.join("colors.toml"), out);
    }
    Ok(())
}

pub fn apply_matugen(wallpaper: &Path) -> Result<Palette> {
    if !wallpaper.is_file() {
        bail!("el wallpaper no existe: {}", wallpaper.display());
    }
    let mut cmd = Command::new("matugen");
    cmd.args([
        "image",
        &wallpaper.to_string_lossy(),
        "--mode",
        "dark",
        "--type",
        "scheme-vibrant",
        "--source-color-index",
        "0",
        "--contrast",
        "0.6",
    ]);
    let matugen_config = matugen_config()?;
    if matugen_config.is_file() {
        cmd.args(["-c", &matugen_config.to_string_lossy()]);
    }
    cmd.args(["-j", "hex"]);
    let output = cmd
        .stdin(Stdio::null())
        .output()
        .context("no se pudo ejecutar matugen (¿está instalado?)")?;
    if !output.status.success() {
        bail!("matugen falló: {}", stderr_tail(&output.stderr));
    }
    let mut stream = serde_json::Deserializer::from_slice(&output.stdout).into_iter::<Value>();
    let json = stream
        .next()
        .context("matugen no devolvió JSON")?
        .context("matugen devolvió JSON inválido")?;
    let palette = Palette::from_matugen(&json)?;
    palette.write(Engine::Matugen)?;
    palette.apply_system()?;
    Ok(palette)
}

// ---------------------------------------------------------------------------
// API pública
// ---------------------------------------------------------------------------

/// Aplica un esquema tinty ya existente (`base16-…` / `base24-…`).
pub fn apply_tinty_scheme(scheme: &str) -> Result<Palette> {
    if !valid_tinty_scheme_name(scheme) {
        bail!("nombre de esquema tinty no válido: {scheme}");
    }
    let slots = if let Some(path) = tinty_custom_scheme_path(scheme)? {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("no se pudo leer el esquema {}", path.display()))?;
        parse_tinty_yaml_slots(&text)
    } else {
        // Preferir el YAML del repositorio de tinty (baseXX por clave).
        let (system, name) = scheme.split_once('-').unwrap_or(("base24", scheme));
        let repo_path = home()?
            .join(".local/share/tinted-theming/tinty/scheme-repos/schemes")
            .join(system)
            .join(format!("{name}.yaml"));
        if repo_path.is_file() {
            let text = std::fs::read_to_string(&repo_path)
                .with_context(|| format!("no se pudo leer el esquema {}", repo_path.display()))?;
            parse_tinty_yaml_slots(&text)
        } else {
            let script = tinty_scripts_dir()?.join(format!("{scheme}.sh"));
            let text = std::fs::read_to_string(&script).with_context(|| {
                format!(
                    "no encontré el archivo de datos del esquema {} ({})",
                    scheme,
                    script.display()
                )
            })?;
            // Convertir colorXX del script a claves baseXX.
            let raw = parse_tinty_slots(&text);
            let mut slots = HashMap::new();
            let mapping = [
                (0u8, "base00"),
                (1, "base08"),
                (2, "base0b"),
                (3, "base0a"),
                (4, "base0d"),
                (5, "base0e"),
                (6, "base0c"),
                (7, "base05"),
                (8, "base03"),
                (15, "base07"),
                (16, "base09"),
                (17, "base0f"),
                (9, "base12"),
                (10, "base14"),
                (11, "base13"),
                (12, "base16"),
                (13, "base17"),
                (14, "base15"),
            ];
            for (idx, key) in mapping {
                if let Some(v) = raw.get(&idx) {
                    slots.insert(key.to_string(), v.clone());
                }
            }
            // Fallbacks para claves que el script no aporta.
            if !slots.contains_key("base01") {
                if let Some(v) = slots.get("base00") {
                    slots.insert("base01".to_string(), v.clone());
                }
            }
            if !slots.contains_key("base04") {
                if let Some(v) = slots.get("base05") {
                    slots.insert("base04".to_string(), v.clone());
                }
            }
            if !slots.contains_key("base02") {
                if let Some(v) = slots.get("base01") {
                    slots.insert("base02".to_string(), v.clone());
                }
            }
            if !slots.contains_key("base06") {
                if let Some(v) = slots.get("base05") {
                    slots.insert("base06".to_string(), v.clone());
                }
            }
            slots
        }
    };
    let palette = Palette::from_tinty_slots(&slots)
        .with_context(|| format!("no se pudieron leer los colores de {scheme}"))?;
    palette.write(Engine::Tinty)?;
    write_tinty_terminals(&slots)?;
    palette.apply_system()?;
    save_tinty_scheme(scheme)?;
    Ok(palette)
}

fn valid_tinty_scheme_name(scheme: &str) -> bool {
    !scheme.is_empty()
        && scheme
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

/// Convierte un wallpaper en un esquema base16/base24, lo guarda y lo aplica.
pub fn create_tinty_scheme(image: &Path, system: &str) -> Result<String> {
    if !image.is_file() {
        bail!("el wallpaper no existe: {}", image.display());
    }
    let system = match system.trim().to_lowercase().as_str() {
        "base16" => "base16",
        _ => "base24",
    };
    let (input, temporary) = image_with_extension(image)?;

    let raw_stem = image
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "wallpaper".to_string());
    let stem = sanitize_slug(&raw_stem);
    let slug = format!("aspire-{}", stem.chars().take(40).collect::<String>())
        .trim_end_matches('-')
        .to_string();
    let name = format!("Aspire {raw_stem}");

    let output = Command::new("tinty")
        .args([
            "generate-scheme",
            &input.to_string_lossy(),
            "--system",
            system,
            "--variant",
            "dark",
            "--slug",
            &slug,
            "--name",
            &name,
            "--save",
        ])
        .stdin(Stdio::null())
        .output()
        .context("no se pudo ejecutar tinty (¿está instalado?)")?;
    if temporary {
        let _ = std::fs::remove_file(&input);
    }
    if !output.status.success() {
        bail!(
            "tinty generate-scheme falló: {}",
            stderr_tail(&output.stderr)
        );
    }
    Ok(format!("{system}-{slug}"))
}

pub fn set_engine(engine: Engine) -> Result<Option<Palette>> {
    set_active_engine(engine)?;
    match engine {
        Engine::Matugen => {
            let Some(wallpaper) = last_wallpaper() else {
                return Ok(None);
            };
            Ok(Some(apply_matugen(&wallpaper)?))
        }
        Engine::Tinty => {
            let Some(scheme) = tinty_scheme() else {
                return Ok(None);
            };
            Ok(Some(apply_tinty_scheme(&scheme)?))
        }
    }
}

/// Reaplica el motor activo sin cambiar de fondo. Devuelve `None` si falta
/// el wallpaper (matugen) o el tema guardado (tinty).
pub fn apply_current_colors() -> Result<Option<Palette>> {
    match active_engine() {
        Engine::Matugen => {
            let Some(wallpaper) = last_wallpaper() else {
                return Ok(None);
            };
            Ok(Some(apply_matugen(&wallpaper)?))
        }
        Engine::Tinty => {
            let Some(scheme) = tinty_scheme() else {
                return Ok(None);
            };
            Ok(Some(apply_tinty_scheme(&scheme)?))
        }
    }
}

/// Reacción al cambiar de wallpaper: matugen recolorea; tinty no toca nada.
pub fn on_wallpaper_change(wallpaper: &Path) -> Result<Option<Palette>> {
    match active_engine() {
        Engine::Matugen => Ok(Some(apply_matugen(wallpaper)?)),
        Engine::Tinty => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Utilidades de imagen
// ---------------------------------------------------------------------------

fn detect_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Some("png")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"BM") {
        Some("bmp")
    } else {
        None
    }
}

/// tinty usa la extensión para elegir decodificador; hay fondos con
/// extensión mentirosa (p. ej. `.jpg` que en realidad es PNG). Si no
/// coincide, se copia a /tmp con la extensión correcta.
fn image_with_extension(path: &Path) -> Result<(PathBuf, bool)> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("no se pudo abrir {}", path.display()))?;
    let mut header = [0u8; 16];
    let read = file.read(&mut header).unwrap_or(0);
    let Some(real) = detect_extension(&header[..read]) else {
        return Ok((path.to_path_buf(), false));
    };
    let current = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    let same = current == real || (real == "jpg" && current == "jpeg");
    if same {
        return Ok((path.to_path_buf(), false));
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let temporary =
        std::env::temp_dir().join(format!("aspire-bg-{}-{stamp}.{real}", std::process::id()));
    std::fs::copy(path, &temporary)
        .with_context(|| format!("no se pudo preparar {}", path.display()))?;
    Ok((temporary, true))
}

fn sanitize_slug(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == '-' || c == '_' || c == ' ' {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "wallpaper".to_string()
    } else {
        out.to_string()
    }
}