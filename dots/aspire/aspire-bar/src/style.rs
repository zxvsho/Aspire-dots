use crate::model::WaybarConfig;
use anyhow::{bail, Context, Result};
use gtk::gio::prelude::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const SETTINGS_START: &str = "/* aspire-bar:settings:start\n";
const SETTINGS_END: &str = "\naspire-bar:settings:end */\n";
const IMPORT_START: &str = "/* aspire-bar:import:start */\n";
const IMPORT_END: &str = "\n/* aspire-bar:import:end */\n";
const PALETTE_START: &str = "/* aspire-bar:palette:start */\n";
const PALETTE_END: &str = "\n/* aspire-bar:palette:end */\n";
const RULES_START: &str = "\n/* aspire-bar:rules:start */";
const RULES_END: &str = "/* aspire-bar:rules:end */\n";

pub const COLOR_TOKENS: &[&str] = &[
    "background",
    "background-alt",
    "foreground",
    "selected",
    "selectedtext",
    "active",
    "urgent",
];

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_aspire_palette: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub palette_id: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub palette_colors: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_padding_vertical: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_padding_horizontal: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_margin_horizontal: Option<u16>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub modules: BTreeMap<String, ModuleOverrides>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleOverrides {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub states: BTreeMap<String, StateOverride>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StateOverride {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

pub fn parse(source: &str) -> Result<(String, StyleSettings)> {
    let (source, settings_block) = strip_block(source, SETTINGS_START, SETTINGS_END)?;
    let (source, _) = strip_block(&source, IMPORT_START, IMPORT_END)?;
    let (source, _) = strip_block(&source, PALETTE_START, PALETTE_END)?;
    let (source, _) = strip_block(&source, RULES_START, RULES_END)?;
    let settings = match settings_block {
        Some(contents) => serde_json::from_str(&contents)
            .context("no se pudo leer la configuración de estilo de aspire-bar")?,
        None => StyleSettings::default(),
    };
    validate_settings(&settings)?;
    Ok((source, settings))
}

pub fn render(source: &str, settings: &StyleSettings, config: &WaybarConfig) -> Result<String> {
    validate_settings(settings)?;
    let generated = generated_rules(settings, config);
    if settings == &StyleSettings::default() && generated.is_empty() {
        return Ok(source.to_owned());
    }

    let mut output = String::new();
    if uses_palette_tokens(settings) {
        output.push_str(IMPORT_START);
        output.push_str(&palette_import_rule()?);
        output.push_str(IMPORT_END);
    }
    if uses_fixed_palette(settings) {
        output.push_str(PALETTE_START);
        output.push_str(&fixed_palette_rules(settings)?);
        output.push_str(PALETTE_END);
    }
    if !settings.is_empty() {
        let json = serde_json::to_string_pretty(settings)
            .context("no se pudo serializar la configuración de estilo")?;
        output.push_str(SETTINGS_START);
        output.push_str(&json.replace("*/", "*\\/"));
        output.push_str(SETTINGS_END);
    }
    output.push_str(source);
    if !generated.is_empty() {
        output.push_str(RULES_START);
        output.push('\n');
        output.push_str(&generated);
        output.push_str(RULES_END);
    }
    Ok(output)
}

pub fn uses_palette_tokens(settings: &StyleSettings) -> bool {
    settings.use_aspire_palette != Some(false)
        && (settings.use_aspire_palette == Some(true)
            || settings.foreground.is_some()
            || settings.background.is_some()
            || settings.border_color.is_some()
            || settings.shadow == Some(true)
            || settings.modules.values().any(|module| {
                module
                    .states
                    .values()
                    .any(|state| state.foreground.is_some() || state.background.is_some())
            }))
}

pub fn uses_fixed_palette(settings: &StyleSettings) -> bool {
    settings.use_aspire_palette == Some(false) && !settings.palette_colors.is_empty()
}

fn fixed_palette_rules(settings: &StyleSettings) -> Result<String> {
    let mut rules = String::new();
    for token in COLOR_TOKENS {
        let color = settings
            .palette_colors
            .get(*token)
            .with_context(|| format!("falta el color de paleta «{token}»"))?;
        if !valid_hex_color(color) {
            bail!("valor hexadecimal no válido para «{token}»: {color}");
        }
        rules.push_str(&format!("@define-color {token} {color};\n"));
    }
    Ok(format!(
        "/* Paleta fija: {} */\n{rules}",
        settings.palette_name.as_deref().unwrap_or("Personalizada")
    ))
}

pub fn palette_import_rule() -> Result<String> {
    let colors = dirs::config_dir()
        .context("no se pudo determinar XDG_CONFIG_HOME")?
        .join("theme-engine/current/colors.css");
    let uri = gtk::gio::File::for_path(colors).uri();
    Ok(format!("@import url(\"{uri}\");"))
}

pub fn generated_rules(settings: &StyleSettings, config: &WaybarConfig) -> String {
    let mut rules = Vec::new();
    if settings.use_aspire_palette == Some(true) || uses_fixed_palette(settings) {
        rules.push(format!(
            "* {{\n    color: @{};\n}}\n\nwindow#waybar {{\n    background-color: @{};\n}}",
            settings.foreground.as_deref().unwrap_or("foreground"),
            settings.background.as_deref().unwrap_or("background")
        ));
        let selectors = configured_module_selectors(config);
        if !selectors.is_empty() {
            let modules = selectors.join(",\n");
            let hover = selectors_for_state(&selectors, ":hover");
            let warning = selectors_for_state(&selectors, ".warning");
            let critical = ["critical", "urgent", "disconnected"]
                .into_iter()
                .map(|state| selectors_for_state(&selectors, &format!(".{state}")))
                .collect::<Vec<_>>()
                .join(",\n");
            let active = selectors_for_state(&selectors, ".active");
            let muted = selectors_for_state(&selectors, ".muted");
            let disabled = selectors_for_state(&selectors, ".disabled");
            rules.push(format!(
                "{modules} {{\n    color: @foreground;\n    background-color: transparent;\n}}\n\n\
                 {hover} {{\n    color: @foreground;\n    background-color: @background-alt;\n}}\n\n\
                 {warning} {{\n    color: @selected;\n}}\n\n\
                 {critical} {{\n    color: @foreground;\n    background-color: @background;\n    border: 1px solid @urgent;\n}}\n\n\
                 {active} {{\n    color: @selectedtext;\n    background-color: @active;\n}}\n\n\
                 {muted},\n{disabled} {{\n    color: @background-alt;\n}}"
            ));
        }
        rules.push(
            "#workspaces button { color: @foreground; }\n\
             #workspaces button:hover { color: @foreground; background-color: @background-alt; }\n\
             #workspaces button.active { color: @selectedtext; background-color: @selected; animation: none; }\n\
             #workspaces button.urgent { color: @foreground; background-color: @background; border: 1px solid @urgent; }"
                .to_owned(),
        );
    }

    let mut global = Vec::new();
    if let Some(font) = &settings.font_family {
        global.push(format!("font-family: {};", css_font_family(font)));
    }
    if let Some(size) = settings.font_size {
        global.push(format!("font-size: {size}px;"));
    }
    if let Some(color) = &settings.foreground {
        global.push(format!("color: @{color};"));
    }
    if !global.is_empty() {
        rules.push(format!("* {{\n    {}\n}}", global.join("\n    ")));
    }

    let mut bar = Vec::new();
    if let Some(color) = &settings.background {
        bar.push(format!("background-color: @{color};"));
    }
    if let Some(opacity) = settings.opacity {
        bar.push(format!("opacity: {opacity:.2};"));
    }
    if let Some(radius) = settings.border_radius {
        bar.push(format!("border-radius: {radius}px;"));
    }
    if let Some(style) = &settings.border_style {
        let width = settings.border_width.unwrap_or_default();
        if style == "none" || width == 0 {
            bar.push("border: none;".to_owned());
        } else if let Some(color) = &settings.border_color {
            bar.push(format!("border: {width}px {style} @{color};"));
        }
    } else if let (Some(width), Some(color)) = (settings.border_width, &settings.border_color) {
        if width == 0 {
            bar.push("border: none;".to_owned());
        } else {
            bar.push(format!("border: {width}px solid @{color};"));
        }
    }
    if settings.shadow == Some(true) {
        bar.push("box-shadow: 0 2px 8px alpha(@background, 0.35);".to_owned());
    } else if settings.shadow == Some(false) {
        bar.push("box-shadow: none;".to_owned());
    }
    if !bar.is_empty() {
        rules.push(format!("window#waybar {{\n    {}\n}}", bar.join("\n    ")));
    }

    let mut module = Vec::new();
    if let Some(vertical) = settings.module_padding_vertical {
        module.push(format!("padding-top: {vertical}px;"));
        module.push(format!("padding-bottom: {vertical}px;"));
    }
    if let Some(horizontal) = settings.module_padding_horizontal {
        module.push(format!("padding-left: {horizontal}px;"));
        module.push(format!("padding-right: {horizontal}px;"));
    }
    if let Some(horizontal) = settings.module_margin_horizontal {
        module.push(format!("margin-left: {horizontal}px;"));
        module.push(format!("margin-right: {horizontal}px;"));
    }
    if !module.is_empty() {
        let selectors = configured_module_selectors(config);
        if !selectors.is_empty() {
            rules.push(format!(
                "{} {{\n    {}\n}}",
                selectors.join(",\n"),
                module.join("\n    ")
            ));
        }
    }

    for (name, overrides) in &settings.modules {
        let selector = module_selector(name);
        for (state, style) in &overrides.states {
            let Some(selector) = selector.as_ref() else {
                continue;
            };
            let selector = match state.as_str() {
                "hover" => format!("{selector}:hover"),
                "warning" | "critical" | "urgent" | "active" | "disabled" => {
                    format!("{selector}.{state}")
                }
                _ => selector.clone(),
            };
            let mut declarations = Vec::new();
            if let Some(color) = &style.foreground {
                declarations.push(format!("color: @{color};"));
            }
            if let Some(color) = &style.background {
                declarations.push(format!("background-color: @{color};"));
            }
            if !declarations.is_empty() {
                rules.push(format!(
                    "{selector} {{\n    {}\n}}",
                    declarations.join("\n    ")
                ));
            }
        }
    }

    rules.join("\n\n")
}

pub fn validate_settings(settings: &StyleSettings) -> Result<()> {
    if let Some(font) = &settings.font_family {
        if font.trim().is_empty()
            || font.split(',').any(|family| family.trim().is_empty())
            || !font
                .chars()
                .all(|character| character.is_alphanumeric() || " -_,.".contains(character))
        {
            bail!("la fuente solo puede contener letras, números, espacios, coma, punto, guion y guion bajo");
        }
    }
    if settings
        .font_size
        .is_some_and(|size| !(6..=96).contains(&size))
    {
        bail!("el tamaño de fuente debe estar entre 6 y 96 px");
    }
    for color in [
        settings.foreground.as_deref(),
        settings.background.as_deref(),
        settings.border_color.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_color(color)?;
    }
    for (name, color) in &settings.palette_colors {
        if !COLOR_TOKENS.contains(&name.as_str()) || !valid_hex_color(color) {
            bail!("color hexadecimal o nombre canónico no válido: {name}={color}");
        }
    }
    if settings
        .opacity
        .is_some_and(|opacity| !(0.1..=1.0).contains(&opacity))
    {
        bail!("la opacidad debe estar entre 0,10 y 1,00");
    }
    if settings
        .border_style
        .as_deref()
        .is_some_and(|style| !["none", "solid", "dashed", "dotted", "double"].contains(&style))
    {
        bail!("estilo de borde no válido");
    }
    for module in settings.modules.values() {
        for (state, override_style) in &module.states {
            if ![
                "normal", "hover", "warning", "critical", "urgent", "active", "disabled",
            ]
            .contains(&state.as_str())
            {
                bail!("estado de estilo no válido: {state}");
            }
            for color in [
                override_style.foreground.as_deref(),
                override_style.background.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                validate_color(color)?;
            }
        }
    }
    Ok(())
}

impl StyleSettings {
    pub fn is_empty(&self) -> bool {
        self.use_aspire_palette.is_none()
            && self.palette_name.is_none()
            && self.palette_colors.is_empty()
            && self.font_family.is_none()
            && self.font_size.is_none()
            && self.foreground.is_none()
            && self.background.is_none()
            && self.opacity.is_none()
            && self.border_radius.is_none()
            && self.border_width.is_none()
            && self.border_color.is_none()
            && self.border_style.is_none()
            && self.shadow.is_none()
            && self.module_padding_vertical.is_none()
            && self.module_padding_horizontal.is_none()
            && self.module_margin_horizontal.is_none()
            && self.modules.is_empty()
    }
}

fn validate_color(color: &str) -> Result<()> {
    if !COLOR_TOKENS.contains(&color) {
        bail!("color no reconocido por aspire-core: {color}");
    }
    Ok(())
}

fn valid_hex_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn strip_block(source: &str, start: &str, end: &str) -> Result<(String, Option<String>)> {
    let Some(start_index) = source.find(start) else {
        if source.contains(end) {
            bail!("bloque de estilo de aspire-bar incompleto");
        }
        return Ok((source.to_owned(), None));
    };
    if source[start_index + start.len()..].contains(start) {
        bail!("el bloque de estilo de aspire-bar está duplicado");
    }
    let content_start = start_index + start.len();
    let content_end = source[content_start..]
        .find(end)
        .map(|index| content_start + index)
        .context("bloque de estilo de aspire-bar incompleto")?;
    let block_end = content_end + end.len();
    let block = source[content_start..content_end].to_owned();
    let mut stripped = String::with_capacity(source.len() - (block_end - start_index));
    stripped.push_str(&source[..start_index]);
    stripped.push_str(&source[block_end..]);
    Ok((stripped, Some(block)))
}

fn css_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn css_font_family(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .map(|family| {
            if [
                "serif",
                "sans-serif",
                "monospace",
                "cursive",
                "fantasy",
                "system-ui",
            ]
            .contains(&family)
            {
                family.to_owned()
            } else {
                format!("\"{}\"", css_string(family))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn configured_module_selectors(config: &WaybarConfig) -> Vec<String> {
    let mut modules = config
        .bars()
        .iter()
        .flat_map(|bar| bar.zones())
        .flatten()
        .filter_map(|module| module_selector(&module))
        .collect::<Vec<_>>();
    modules.sort();
    modules.dedup();
    modules
}

fn selectors_for_state(selectors: &[String], state: &str) -> String {
    selectors
        .iter()
        .map(|selector| format!("{selector}{state}"))
        .collect::<Vec<_>>()
        .join(",\n")
}

fn module_selector(module: &str) -> Option<String> {
    let (module, instance) = module
        .split_once('#')
        .map_or((module, None), |(module, instance)| {
            (module, Some(instance))
        });
    let identifier = module
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else if character == '/' {
                '-'
            } else {
                '_'
            }
        })
        .collect::<String>();
    if identifier.is_empty() {
        return None;
    }
    let instance = instance
        .filter(|instance| !instance.is_empty())
        .map(|instance| {
            let instance = instance
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                        character
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            format!(".{instance}")
        })
        .unwrap_or_default();
    Some(format!("#{identifier}{instance}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WaybarConfig;

    #[test]
    fn settings_round_trip_and_generated_css_use_engine_tokens() {
        let config: WaybarConfig =
            json5::from_str(r#"{"modules-left":["custom/cava","clock"]}"#).unwrap();
        let mut settings = StyleSettings {
            use_aspire_palette: Some(true),
            font_family: Some("JetBrainsMono Nerd Font".to_owned()),
            font_size: Some(12),
            background: Some("background".to_owned()),
            border_radius: Some(0),
            ..StyleSettings::default()
        };
        settings.modules.insert(
            "custom/cava".to_owned(),
            ModuleOverrides {
                states: BTreeMap::from([(
                    "warning".to_owned(),
                    StateOverride {
                        foreground: Some("urgent".to_owned()),
                        background: None,
                    },
                )]),
            },
        );
        let css = render("/* base */\n", &settings, &config).unwrap();
        let (base, recovered) = parse(&css).unwrap();

        assert_eq!(base, "/* base */\n");
        assert_eq!(recovered, settings);
        assert!(css.contains("@background"));
        assert!(css.contains("#custom-cava.warning"));
        assert!(!css.contains("#282a36"));
        assert_eq!(render(&base, &recovered, &config).unwrap(), css);
    }

    #[test]
    fn retro_rules_keep_palette_driven_colors_and_square_edges() {
        let config: WaybarConfig = json5::from_str(r#"{"modules-left":["clock"]}"#).unwrap();
        let settings = StyleSettings {
            font_family: Some("JetBrainsMono Nerd Font, monospace".to_owned()),
            foreground: Some("foreground".to_owned()),
            background: Some("background".to_owned()),
            border_radius: Some(0),
            border_color: Some("active".to_owned()),
            border_width: Some(1),
            border_style: Some("solid".to_owned()),
            opacity: Some(0.95),
            shadow: Some(false),
            module_padding_vertical: Some(3),
            module_padding_horizontal: Some(6),
            ..StyleSettings::default()
        };
        let css = generated_rules(&settings, &config);

        assert!(css.contains("font-family: \"JetBrainsMono Nerd Font\", monospace"));
        assert!(css.contains("border-radius: 0px"));
        assert!(css.contains("border: 1px solid @active"));
    }

    #[test]
    fn palette_mode_recolors_existing_modules_and_states_from_the_engine() {
        let config: WaybarConfig =
            json5::from_str(r#"{"modules-left":["custom/cava","battery","clock"]}"#).unwrap();
        let settings = StyleSettings {
            use_aspire_palette: Some(true),
            ..StyleSettings::default()
        };
        let css = generated_rules(&settings, &config);

        assert!(css.contains("@background"));
        assert!(css.contains("@foreground"));
        assert!(css.contains("#custom-cava.critical"));
        assert!(css.contains("#battery.critical"));
        assert!(css.contains("#custom-cava.warning"));
        assert!(css.contains("border: 1px solid @urgent"));
        assert!(css.contains("color: @selected"));
        assert!(css.contains("#clock:hover"));
        assert!(css.contains("#workspaces button.active"));
        assert!(!css.contains("#282a36"));
    }

    #[test]
    fn rejects_css_injection_in_font_and_noncanonical_colors() {
        let settings = StyleSettings {
            font_family: Some("monospace; color: red".to_owned()),
            ..StyleSettings::default()
        };
        assert!(validate_settings(&settings).is_err());
        assert!(validate_color("#fff").is_err());
    }

    #[test]
    fn removes_managed_sections_without_losing_custom_css() {
        let source = format!(
            "before\n{SETTINGS_START}{{\"font_size\":12}}{SETTINGS_END}{RULES_START}\n* {{}}\n{RULES_END}after"
        );
        let (base, settings) = parse(&source).unwrap();
        assert_eq!(base, "before\nafter");
        assert_eq!(settings.font_size, Some(12));
    }

    #[test]
    fn retains_module_state_settings() {
        let settings = StyleSettings {
            modules: BTreeMap::from([(
                "clock".to_owned(),
                ModuleOverrides {
                    states: BTreeMap::from([(
                        "hover".to_owned(),
                        StateOverride {
                            foreground: Some("selected".to_owned()),
                            background: Some("background-alt".to_owned()),
                        },
                    )]),
                },
            )]),
            ..StyleSettings::default()
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        let decoded: StyleSettings = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, settings);
    }

    #[test]
    fn fixed_palette_generates_color_definitions_without_dynamic_import() {
        let config: WaybarConfig = json5::from_str(r#"{"modules-left":["clock"]}"#).unwrap();
        let settings = StyleSettings {
            use_aspire_palette: Some(false),
            palette_name: Some("Dracula".to_owned()),
            palette_colors: COLOR_TOKENS
                .iter()
                .map(|name| ((*name).to_owned(), "#112233".to_owned()))
                .collect(),
            ..StyleSettings::default()
        };
        let css = render("", &settings, &config).unwrap();
        assert!(css.contains("@define-color background #112233;"));
        assert!(css.contains("/* Paleta fija: Dracula */"));
        assert!(!css.contains("theme-engine/current/colors.css"));
        assert!(css.contains("#clock:hover"));
        let (_, decoded) = parse(&css).unwrap();
        assert_eq!(decoded, settings);
        assert_eq!(render("", &decoded, &config).unwrap(), css);
    }
}
