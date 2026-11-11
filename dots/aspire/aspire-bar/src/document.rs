use crate::generator;
use crate::model::WaybarConfig;
use crate::style::{self, StyleSettings};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct BarDocument {
    pub config: WaybarConfig,
    pub style: String,
    pub style_settings: StyleSettings,
}

impl BarDocument {
    pub fn use_dynamic_palette_when_unspecified(&mut self) {
        if self.style_settings.use_aspire_palette.is_none() {
            self.style_settings.use_aspire_palette = Some(true);
        }
    }

    pub fn read(config_path: &Path, style_path: &Path) -> Result<Self> {
        let config_source = fs::read_to_string(config_path)
            .with_context(|| format!("no se pudo leer {}", config_path.display()))?;
        let config = json5::from_str(&config_source)
            .with_context(|| format!("JSONC inválido en {}", config_path.display()))?;
        let style_source = fs::read_to_string(style_path)
            .with_context(|| format!("no se pudo leer {}", style_path.display()))?;
        let (style, style_settings) = style::parse(&style_source)?;
        Ok(Self {
            config,
            style,
            style_settings,
        })
    }

    pub fn from_config_source(config_source: &str, style: String) -> Result<Self> {
        let config = json5::from_str(config_source).context("configuración JSONC inválida")?;
        let (style, style_settings) = style::parse(&style)?;
        Ok(Self {
            config,
            style,
            style_settings,
        })
    }

    pub fn generated_config(&self) -> Result<String> {
        generator::generate_config(&self.config)
    }

    pub fn generated_style(&self) -> Result<String> {
        let mut settings = self.style_settings.clone();
        if settings.use_aspire_palette == Some(false) {
            if let Some(palette_id) = settings.palette_id.as_deref() {
                let palette = crate::palette::find_by_id(palette_id)?;
                settings.palette_name = Some(palette.name);
                settings.palette_colors = palette.colors;
            }
        }
        generator::generate_style(&self.style, &settings, &self.config)
    }
}
