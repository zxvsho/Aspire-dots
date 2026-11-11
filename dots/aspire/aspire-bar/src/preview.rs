use crate::document::BarDocument;
use crate::palette::COLOR_NAMES;
use anyhow::{bail, Result};
use gtk::prelude::*;
use gtk::{Align, Box as GtkBox, CssProvider, Label, Orientation};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

static PREVIEW_ID: AtomicUsize = AtomicUsize::new(0);

type ModuleSelectionHandler = Rc<dyn Fn(usize, String)>;

pub struct Preview {
    pub page: GtkBox,
    bar: gtk::CenterBox,
    zones: [GtkBox; 3],
    status: Label,
    provider: CssProvider,
    id: usize,
    on_module_selected: Rc<RefCell<Option<ModuleSelectionHandler>>>,
}

impl Preview {
    pub fn new() -> Self {
        let id = PREVIEW_ID.fetch_add(1, Ordering::Relaxed);
        let page = GtkBox::new(Orientation::Vertical, 4);
        page.set_margin_top(6);
        page.set_margin_bottom(6);
        page.set_margin_start(6);
        page.set_margin_end(6);

        let bar = gtk::CenterBox::new();
        bar.set_widget_name(&format!("aspire-preview-bar-{id}"));
        bar.set_halign(Align::Fill);
        bar.set_valign(Align::Start);
        let zones = std::array::from_fn(|_| GtkBox::new(Orientation::Horizontal, 6));
        for (index, zone) in zones.iter().enumerate() {
            zone.set_widget_name(&format!("aspire-preview-zone-{id}-{index}"));
            if index == 1 {
                zone.set_halign(Align::Center);
            }
        }
        bar.set_start_widget(Some(&zones[0]));
        bar.set_center_widget(Some(&zones[1]));
        bar.set_end_widget(Some(&zones[2]));
        page.append(&bar);
        let status = Label::new(Some("Sin cambios aplicados"));
        status.add_css_class("dim-label");
        status.set_halign(Align::Start);
        page.append(&status);

        let provider = CssProvider::new();
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }

        Self {
            page,
            bar,
            zones,
            status,
            provider,
            id,
            on_module_selected: Rc::new(RefCell::new(None)),
        }
    }

    pub fn render(&self, document: &BarDocument) {
        self.render_selected(document, None);
    }

    pub fn set_on_module_selected(&self, callback: impl Fn(usize, String) + 'static) {
        *self.on_module_selected.borrow_mut() = Some(Rc::new(callback));
    }

    pub fn zone_widgets(&self) -> [GtkBox; 3] {
        self.zones.clone()
    }

    pub fn render_selected(&self, document: &BarDocument, selected: Option<(usize, &str)>) {
        let colors = match colors_for(&document.style_settings) {
            Ok(colors) => colors,
            Err(error) => {
                self.status
                    .set_text(&format!("No se pudo resolver la paleta: {error:#}"));
                return;
            }
        };
        let position = document
            .config
            .bars()
            .first()
            .and_then(|bar| bar.position.as_ref())
            .and_then(serde_json::Value::as_str)
            .unwrap_or("top");
        let bar_config = document.config.bars().first();
        let is_vertical = matches!(position, "left" | "right");
        self.bar.set_orientation(if is_vertical {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        });
        for (zone_index, zone) in self.zones.iter().enumerate() {
            clear_box(zone);
            zone.set_orientation(if is_vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            });
            zone.set_size_request(if is_vertical { 76 } else { 96 }, -1);
            zone.set_hexpand(!is_vertical && zone_index != 1);
            zone.set_vexpand(is_vertical);
        }
        self.bar.set_halign(Align::Fill);
        self.bar.set_valign(if position == "bottom" {
            Align::End
        } else {
            Align::Start
        });
        self.bar.set_hexpand(true);
        self.bar.set_vexpand(false);
        let height = bar_config
            .and_then(|bar| bar.height.as_ref())
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(36)
            .clamp(24, 96) as i32;
        self.bar.set_size_request(-1, height);

        let modules = bar_config.map(|bar| bar.zones()).unwrap_or_default();
        for (zone_index, names) in modules.iter().enumerate() {
            if names.is_empty() {
                let placeholder = Label::new(Some(
                    [
                        "Izquierda · soltar aquí",
                        "Centro · soltar aquí",
                        "Derecha · soltar aquí",
                    ][zone_index],
                ));
                placeholder.set_wrap(true);
                placeholder.set_halign(Align::Center);
                placeholder.set_valign(Align::Center);
                placeholder.add_css_class("dim-label");
                self.zones[zone_index].append(&placeholder);
            }
            for (module_index, module) in names.iter().enumerate() {
                let button = gtk::Button::with_label(&display_name(module));
                button.set_widget_name(&format!(
                    "aspire-preview-chip-{id}-{zone_index}-{module_index}",
                    id = self.id
                ));
                button.add_css_class(&format!("aspire-preview-chip-{}", self.id));
                if selected == Some((zone_index, module.as_str()))
                    || (selected.is_none() && module_index == 0 && zone_index == 0)
                {
                    button.add_css_class(&format!("aspire-preview-active-{}", self.id));
                }
                button.set_tooltip_text(Some(module));
                let module_name = module.clone();
                let on_selected = Rc::clone(&self.on_module_selected);
                button.connect_clicked(move |_| {
                    if let Some(callback) = on_selected.borrow().as_ref() {
                        callback(zone_index, module_name.clone());
                    }
                });
                let drag = gtk::DragSource::new();
                drag.set_actions(gtk::gdk::DragAction::MOVE);
                let dragged = module.clone();
                drag.connect_prepare(move |_, _, _| {
                    Some(gtk::gdk::ContentProvider::for_value(
                        &dragged.to_value(),
                    ))
                });
                button.add_controller(drag);
                self.zones[zone_index].append(&button);
            }
        }
        let radius = document
            .style_settings
            .border_radius
            .map_or(16, |value| value.min(48));
        self.provider.load_from_string(&format!(
            "#aspire-preview-bar-{id} {{ background-color: alpha({bg}, 0.94); color: {fg}; border: 1px solid alpha({fg}, 0.2); border-radius: {radius}px; padding: 5px 8px; margin-top: 8px; margin-bottom: 8px; }}\n\
             .aspire-preview-chip-{id} {{ background-color: {surface}; color: {fg}; border-radius: 999px; padding: 4px 9px; font-size: 11px; }}\n\
             .aspire-preview-active-{id} {{ background-color: {selected}; color: {selectedtext}; }}\n\
             .aspire-drop-hover {{ background-color: alpha({selected}, 0.20); border-radius: 8px; }}\n\
             .aspire-preview-insert {{ background-color: {selected}; min-width: 3px; min-height: 18px; border-radius: 2px; }}",
            id = self.id,
            bg = colors["background"],
            surface = colors["background-alt"],
            fg = colors["foreground"],
            selected = colors["selected"],
            selectedtext = colors["selectedtext"],
        ));

        let count = document.config.module_count();
        self.status.set_text(&format!(
            "{} · {} módulo(s) · {}",
            position,
            count,
            document
                .style_settings
                .palette_name
                .as_deref()
                .unwrap_or("colores dinámicos")
        ));
    }
}

impl Default for Preview {
    fn default() -> Self {
        Self::new()
    }
}

fn colors_for(settings: &crate::style::StyleSettings) -> Result<BTreeMap<&'static str, String>> {
    let theme = aspire_core::colors::load_theme_colors();
    let fallback = [
        ("background", "#1e1e2e"),
        ("background-alt", "#313244"),
        ("foreground", "#cdd6f4"),
        ("selected", "#cba6f7"),
        ("selectedtext", "#1e1e2e"),
        ("active", "#89b4fa"),
        ("urgent", "#f38ba8"),
    ];
    let dynamic = [
        theme.background,
        theme.background_alt,
        theme.foreground,
        theme.selected,
        theme.selectedtext,
        theme.active,
        theme.urgent,
    ];
    let fixed_colors = if settings.use_aspire_palette == Some(false) {
        if let Some(id) = settings.palette_id.as_deref() {
            Some(crate::palette::find_by_id(id)?.colors)
        } else if !settings.palette_colors.is_empty() {
            Some(settings.palette_colors.clone())
        } else {
            bail!("el tema fijo no tiene una paleta asociada")
        }
    } else {
        None
    };
    let mut colors = BTreeMap::new();
    for (index, name) in COLOR_NAMES.iter().enumerate() {
        let color = fixed_colors
            .as_ref()
            .and_then(|fixed| fixed.get(*name).cloned())
            .or_else(|| {
                dynamic[index]
                    .map(|color| format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b))
            });
        colors.insert(*name, color.unwrap_or_else(|| fallback[index].1.to_owned()));
    }
    Ok(colors)
}

fn display_name(name: &str) -> String {
    name.rsplit_once('/')
        .map_or(name, |(_, leaf)| leaf)
        .replace('-', " ")
}

fn clear_box(container: &GtkBox) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_resolves_fixed_colors_from_the_palette_reference() {
        let settings = crate::style::StyleSettings {
            use_aspire_palette: Some(false),
            palette_id: Some("catppuccin-latte".to_owned()),
            ..crate::style::StyleSettings::default()
        };
        let colors = colors_for(&settings).unwrap();
        assert_eq!(colors["background"], "#eff1f5");
        assert_eq!(colors["urgent"], "#d20f39");
    }
}
