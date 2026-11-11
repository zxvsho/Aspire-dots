use crate::model::WaybarConfig;
use crate::style::{self, StyleSettings};
use anyhow::{Context, Result};

pub fn generate_config(config: &WaybarConfig) -> Result<String> {
    let mut output =
        serde_json::to_string_pretty(config).context("no se pudo serializar la configuración")?;
    output.push('\n');
    Ok(output)
}

pub fn generate_style(
    source: &str,
    settings: &StyleSettings,
    config: &WaybarConfig,
) -> Result<String> {
    style::render(source, settings, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::BarDocument;

    #[test]
    fn css_generation_keeps_existing_style_byte_for_byte() {
        let css = "@import url(\"colors.css\");\n#clock { color: @foreground; }\n";
        assert_eq!(
            generate_style(
                css,
                &StyleSettings::default(),
                &WaybarConfig::Single(Box::default())
            )
            .unwrap(),
            css
        );
    }

    #[test]
    fn jsonc_generation_preserves_config_semantics() {
        let source = r#"
        {
          // A string containing comment markers must remain intact.
          "modules-left": ["custom/example",],
          "custom/example": {"exec": "printf '// text'", "enabled": true,},
          "unknown": [1, 2,],
        }
        "#;
        let document = BarDocument::from_config_source(source, String::new()).unwrap();
        let output = document.generated_config().unwrap();
        let reread: WaybarConfig = serde_json::from_str(&output).unwrap();

        assert_eq!(reread, document.config);
        assert!(output.contains("printf '// text'"));
    }
}
