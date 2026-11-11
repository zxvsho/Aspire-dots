use crate::document::BarDocument;
use crate::model::WaybarConfig;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub config: WaybarConfig,
}

#[derive(Deserialize)]
struct TemplateCatalog {
    template: Vec<TemplateEntry>,
}

#[derive(Deserialize)]
struct TemplateEntry {
    id: String,
    name: String,
    category: String,
    description: String,
    source: PathBuf,
}

pub fn list() -> Result<Vec<Template>> {
    let root = asset_root()?;
    let catalog_path = root.join("templates.toml");
    let source = fs::read_to_string(&catalog_path)
        .with_context(|| format!("no se pudo leer {}", catalog_path.display()))?;
    let catalog: TemplateCatalog = toml::from_str(&source).with_context(|| {
        format!(
            "catálogo de plantillas inválido: {}",
            catalog_path.display()
        )
    })?;
    catalog
        .template
        .into_iter()
        .map(|entry| {
            let config_path = root.join(&entry.source).join("config.jsonc");
            let config_source = fs::read_to_string(&config_path)
                .with_context(|| format!("no se pudo leer {}", config_path.display()))?;
            let config = json5::from_str(&config_source)
                .with_context(|| format!("JSONC inválido en {}", config_path.display()))?;
            Ok(Template {
                id: entry.id,
                name: entry.name,
                category: entry.category,
                description: entry.description,
                config,
            })
        })
        .collect()
}

pub fn find(id: &str) -> Result<Template> {
    list()?
        .into_iter()
        .find(|template| template.id == id)
        .with_context(|| format!("no existe la plantilla «{id}»"))
}

pub fn apply_layout(
    document: &mut BarDocument,
    template: &Template,
    bar_index: usize,
) -> Result<()> {
    let Some(source) = template.config.bars().first() else {
        bail!("la plantilla no contiene una barra");
    };
    let target = document
        .config
        .bar_mut(bar_index)
        .context("no existe la barra seleccionada")?;
    target.position = source.position.clone();
    target.height = source.height.clone();
    target.modules_left = source.modules_left.clone();
    target.modules_center = source.modules_center.clone();
    target.modules_right = source.modules_right.clone();
    for module in target.zones().into_iter().flatten() {
        if let Some(definition) = source.extra.get(&module) {
            target
                .extra
                .entry(module)
                .or_insert_with(|| definition.clone());
        }
    }
    Ok(())
}

fn asset_root() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no se pudo determinar HOME")?;
    Ok(home.join(".dots/assets/waybar-templates"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_templates_are_layout_only_and_use_mango_modules() {
        let templates = list().unwrap();
        assert_eq!(templates.len(), 6);
        assert_eq!(
            templates
                .iter()
                .map(|template| template.category.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            ["capsule", "full", "gaming", "minimal", "productivity"]
                .into_iter()
                .collect()
        );
        for template in templates {
            let modules = template
                .config
                .bars()
                .iter()
                .flat_map(|bar| bar.zones())
                .flatten()
                .collect::<Vec<_>>();
            assert!(!modules.is_empty());
            if template.id == "dynamic-island" {
                assert!(modules.iter().any(|module| module == "group/island"));
            } else {
                assert!(modules.iter().any(|module| module == "ext/workspaces"));
            }
            assert!(!modules.iter().any(|module| module.contains("hyprland")));
        }
        let island = find("dynamic-island").unwrap();
        assert!(island.config.bars()[0]
            .module_definition("group/island")
            .is_some());
    }

    #[test]
    fn applying_template_changes_only_layout_and_preserves_user_module_options() {
        let mut document = BarDocument::from_config_source(
            r#"{
                "position": "bottom",
                "height": 60,
                "modules-left": ["custom/cava"],
                "custom/cava": {"exec": "my-cava", "private-option": true},
                "future-option": {"preserved": true}
            }"#,
            "/* user CSS */".to_owned(),
        )
        .unwrap();
        document.style_settings.border_radius = Some(17);
        document.style_settings.palette_id = Some("dracula".to_owned());
        let template = Template {
            id: "test".to_owned(),
            name: "Test".to_owned(),
            category: "minimal".to_owned(),
            description: String::new(),
            config: json5::from_str(
                r#"{
                    "position": "top",
                    "height": 30,
                    "modules-left": ["ext/workspaces"],
                    "modules-center": ["clock"],
                    "ext/workspaces": {"format": "{icon}"},
                    "clock": {"format": "{:%H:%M}"}
                }"#,
            )
            .unwrap(),
        };
        let style = document.style.clone();
        let settings = document.style_settings.clone();
        apply_layout(&mut document, &template, 0).unwrap();
        let bar = &document.config.bars()[0];
        assert_eq!(bar.position.as_ref().unwrap(), "top");
        assert_eq!(bar.height.as_ref().unwrap(), 30);
        assert_eq!(
            bar.modules_left.as_ref().unwrap(),
            &vec!["ext/workspaces".to_owned()]
        );
        assert_eq!(
            bar.modules_center.as_ref().unwrap(),
            &vec!["clock".to_owned()]
        );
        assert_eq!(
            bar.module_definition("custom/cava").unwrap()["private-option"],
            true
        );
        assert_eq!(bar.extra["future-option"]["preserved"], true);
        assert!(bar.module_definition("ext/workspaces").is_some());
        assert_eq!(document.style, style);
        assert_eq!(document.style_settings, settings);
    }
}
