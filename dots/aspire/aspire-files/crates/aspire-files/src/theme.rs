//! Tema de aspire-files: lee la paleta de `theme-engine` y aplica el CSS de
//! la interfaz. La recarga en caliente se hace desde `app.rs` con un
//! `gio::FileMonitor` sobre el directorio de la paleta.

use std::cell::RefCell;
use std::path::Path;

use gtk::{gdk, CssProvider};

use aspire_config::Engine;

/// Paleta de emergencia si no existe `colors.css`.
const FALLBACK: &str = r#"
@define-color background #1e1e2e;
@define-color background-alt #181825;
@define-color foreground #cdd6f4;
@define-color selected #89b4fa;
@define-color selectedtext #1e1e2e;
@define-color active #89b4fa;
@define-color urgent #f38ba8;
@define-color color0 #45475a;
@define-color color1 #f38ba8;
@define-color color2 #a6e3a1;
@define-color color3 #f9e2af;
@define-color color4 #89b4fa;
@define-color color5 #f5c2e7;
@define-color color6 #94e2d5;
@define-color color7 #bac2de;
@define-color color8 #585b70;
@define-color color9 #f38ba8;
@define-color color10 #a6e3a1;
@define-color color11 #f9e2af;
@define-color color12 #89b4fa;
@define-color color13 #f5c2e7;
@define-color color14 #94e2d5;
@define-color color15 #a6adc8;
"#;

/// CSS propio de la interfaz. Solo usa colores con nombre para que Matugen y
/// Tinty lo controlen por completo.
const APP_CSS: &str = include_str!("../assets/aspire-base.css");


/// Aplica (o reaplica) la paleta y el CSS de la aplicación.
pub fn apply(
    display: &gdk::Display,
    previous: &RefCell<Option<CssProvider>>,
    colors_path: &Path,
    engine: Engine,
) {
    let palette = if engine == Engine::None {
        FALLBACK.to_string()
    } else {
        std::fs::read_to_string(colors_path).unwrap_or_else(|_| FALLBACK.to_string())
    };
    let css = format!("{palette}\n{APP_CSS}");
    let provider = CssProvider::new();
    provider.load_from_string(&css);

    if let Some(old) = previous.borrow_mut().take() {
        gtk::style_context_remove_provider_for_display(display, &old);
    }
    gtk::style_context_add_provider_for_display(
        display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    *previous.borrow_mut() = Some(provider);
}
