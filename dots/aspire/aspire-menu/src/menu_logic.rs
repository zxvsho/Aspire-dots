//! Lógica del menú independiente de la interfaz.
//!
//! Todo lo que hay aquí no depende de ningún toolkit: lee colores de
//! `colors.rasi`, resuelve el motor de color activo, genera los ítems
//! virtuales de Engine, calcula los ítems visibles y la ruta relativa.

use crate::model::{ActionType, AppState, MenuItem};

pub use aspire_core::colors::{load_theme_colors, parse_color, RgbColor, ThemeColors};

/// Motor activo, delegado en `aspire-core` (el motor vive allí).
pub fn active_engine() -> String {
    aspire_core::theme_engine::active_engine()
        .as_str()
        .to_string()
}

/// `level` es el nivel desde el que se está buscando. La ruta se construye sin
/// su prefijo para que `parent_path` la recorte igual que las del árbol.
pub fn engine_items(level: &str) -> Vec<MenuItem> {
    let active = active_engine();
    let base = if level == "style" {
        "Engine".to_string()
    } else {
        "Style › Engine".to_string()
    };
    let mk = |id: &str, label: &str, icon: &str| MenuItem {
        id: id.into(),
        label: label.into(),
        icon: icon.into(),
        path: format!("{} › {}", base, label),
        action_type: ActionType::Action,
        script_path: None,
        is_submenu: false,
        is_active: active == id,
        level: "engine".into(),
    };
    vec![
        mk("matugen", "Dynamic Color", " "),
        mk("tinty", "Theme Managers", " "),
    ]
}

pub fn visible(state: &mut AppState, query: &str, engine: bool) -> Vec<MenuItem> {
    let q = query.to_lowercase();
    let level = state.current_level.clone();
    if engine {
        return engine_items(&level)
            .into_iter()
            .filter(|i| q.is_empty() || i.label.to_lowercase().contains(&q))
            .collect();
    }
    if query.is_empty() {
        state.search_mode = false;
        state.search_query.clear();
        state.update_filtered_items();
        return state.get_display_items();
    }
    state.search_mode = true;
    state.search_query = query.to_string();
    state.update_filtered_items();
    let mut items = state.get_display_items();
    // Las opciones de Engine solo se añaden desde root o style.
    if level == "root" || level == "style" {
        for e in engine_items(&level) {
            if e.label.to_lowercase().contains(&q) && !items.iter().any(|i| i.label == e.label) {
                items.push(e);
            }
        }
    }
    items.truncate(9);
    items
}

/// Ruta del padre relativa al nivel actual: quita el prompt de ese nivel y
/// el nombre del propio ítem. Vacía si el resultado está en el nivel actual.
pub fn parent_path(state: &AppState, item: &MenuItem) -> String {
    let prefix = format!("{} › ", state.current_prompt());
    let p = item
        .path
        .strip_prefix(prefix.as_str())
        .unwrap_or(&item.path);
    let p = p.strip_suffix(item.label.as_str()).unwrap_or(p);
    p.trim_end_matches([' ', '›']).to_string()
}

/// Aplica un texto tecleado a la búsqueda (`\u{8}` borra, el resto se
/// añade si es texto imprimible). Devuelve `true` si la búsqueda cambió
/// y hay que refrescar.
pub fn apply_typed(query: &mut String, s: &str) -> bool {
    if s == "\u{8}" {
        query.pop();
        true
    } else if !s.is_empty()
        && s.chars()
            .all(|c| c >= ' ' && c != '\u{7f}' && !('\u{f700}'..='\u{f8ff}').contains(&c))
    {
        query.push_str(s);
        true
    } else {
        false
    }
}
