use anyhow::{bail, Result};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;

static CSS_VALIDATION_LOCK: Mutex<()> = Mutex::new(());

pub fn validate_css(css: &str) -> Result<()> {
    let _guard = CSS_VALIDATION_LOCK
        .lock()
        .map_err(|error| anyhow::anyhow!("no se pudo bloquear la validación CSS: {error}"))?;
    let errors = Rc::new(RefCell::new(Vec::new()));
    // CssProvider no necesita display; el constructor Rust impone gtk::init().
    let provider: gtk::CssProvider =
        unsafe { gtk::glib::translate::from_glib_full(gtk::ffi::gtk_css_provider_new()) };
    let errors_for_signal = Rc::clone(&errors);
    provider.connect_parsing_error(move |_, section, error| {
        errors_for_signal.borrow_mut().push(format!(
            "línea {}: {}",
            section.start_location().lines() + 1,
            error.message()
        ));
    });
    provider.load_from_string(css);

    let errors = errors.borrow();
    if !errors.is_empty() {
        bail!("CSS inválido:\n{}", errors.join("\n"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_css_without_a_display() {
        validate_css("window#waybar { background: transparent; }\n").unwrap();
        assert!(validate_css("window#waybar { color: ; }\n").is_err());

        let config: crate::model::WaybarConfig =
            json5::from_str(r#"{"modules-left":["custom/cava","clock"]}"#).unwrap();
        let settings = crate::style::StyleSettings {
            use_aspire_palette: Some(true),
            font_family: Some("JetBrainsMono Nerd Font, monospace".to_owned()),
            opacity: Some(0.95),
            border_radius: Some(0),
            border_width: Some(1),
            border_color: Some("active".to_owned()),
            border_style: Some("solid".to_owned()),
            shadow: Some(true),
            module_padding_vertical: Some(3),
            module_padding_horizontal: Some(6),
            ..crate::style::StyleSettings::default()
        };
        let css = crate::style::render("", &settings, &config).unwrap();
        validate_css(&css).unwrap();
    }
}
