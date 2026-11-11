#![allow(deprecated, dead_code, unused_variables, unused_mut)]
use aspire_core::colors::{self, RgbColor};
use aspire_core::theme_engine;
use aspire_core::themes::{self, ThemePackage};
use gtk::gdk::{Key, ModifierType};
use gtk::glib::{self, Propagation};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, CssProvider, Label, ListBox, ListBoxRow,
    Orientation, ScrolledWindow,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use relm4::{ComponentParts, ComponentSender, RelmApp, SimpleComponent};
use std::cell::RefCell;
use std::rc::Rc;

const WIN_W: i32 = 520;
const WIN_H: i32 = 350;
const LEFT_W: i32 = 140;
const PREVIEW_W: i32 = 380;

#[derive(Clone)]
struct Ui {
    window: ApplicationWindow,
    list: ListBox,
    list_scroll: ScrolledWindow,
    preview: GtkBox,
    preview_provider: Rc<CssProvider>,
    themes: Rc<Vec<ThemePackage>>,
    shown: Rc<RefCell<Vec<usize>>>,
    current: Rc<RefCell<usize>>,
    engine_matugen: bool,
}

#[derive(Debug)]
enum AppMsg {
    Select(usize),
    MoveSelection(isize),
    Apply,
    Close,
}

struct ThemesApp {
    ui: Ui,
}

impl SimpleComponent for ThemesApp {
    type Init = Vec<ThemePackage>;
    type Input = AppMsg;
    type Output = ();
    type Widgets = ();
    type Root = ApplicationWindow;

    fn init_root() -> Self::Root {
        ApplicationWindow::builder().title("Aspire Themes").build()
    }

    fn init(
        packages: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let ui = build_ui(root, packages, sender);
        ComponentParts {
            model: Self { ui },
            widgets: (),
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Select(index) => {
                *self.ui.current.borrow_mut() = index;
                update_preview(&self.ui);
            }
            AppMsg::MoveSelection(delta) => move_selection(&self.ui, delta),
            AppMsg::Apply => apply_current(&self.ui),
            AppMsg::Close => self.ui.window.close(),
        }
    }
}

fn hex(c: Option<RgbColor>, fallback: &str) -> String {
    c.map(|v| format!("#{:02x}{:02x}{:02x}", v.r, v.g, v.b))
        .unwrap_or_else(|| fallback.to_string())
}

fn load_theme_colors_hex() -> (String, String, String, String, String, String, String) {
    let c = colors::load_theme_colors();
    (
        hex(c.background, "#1e1e2e"),
        hex(c.background_alt, "#313244"),
        hex(c.foreground, "#cdd6f4"),
        hex(c.selected, "#cba6f7"),
        hex(c.selectedtext, "#000000"),
        hex(c.active, "#89b4fa"),
        hex(c.urgent, "#f38ba8"),
    )
}

fn build_css() -> String {
    let (bg, alt, fg, sel, seltext, _active, _urgent) = load_theme_colors_hex();
    format!(
        r#"
.themes-window {{ background-color: transparent; border: none; font-family: "JetBrainsMono Nerd Font"; font-size: 12px; }}
.themes-card {{ background-color: {bg}; border: 1px solid {sel}; border-radius: 0; }}
.themes-list {{ background: transparent; padding: 4px; }}
.themes-list row {{ min-height: 22px; background: transparent; border-radius: 0; outline: none; padding: 0 6px; }}
.themes-list row:selected {{ background-color: {sel}; }}
.themes-list row:selected label {{ color: {seltext}; }}
.themes-item {{ color: {fg}; font-size: 12px; }}
.themes-item-active {{ color: {fg}; font-size: 12px; opacity: 0.6; }}
.themes-preview {{ padding: 10px 12px; }}
.tp-name {{ font-size: 14px; font-weight: bold; }}
.tp-badge {{ font-size: 11px; opacity: 0.7; }}
.tp-rowlabel {{ font-size: 11px; opacity: 0.6; }}
.tp-rowval {{ font-size: 11px; }}
.tp-footer {{ font-size: 10px; opacity: 0.6; margin-top: 6px; }}
.tp-notfound {{ font-size: 11px; opacity: 0.7; margin-top: 8px; }}
"#,
        bg = bg,
        fg = fg,
        sel = sel,
        seltext = seltext
    )
}

fn parse_hex(s: &str) -> (u8, u8, u8) {
    let s = s.trim().trim_start_matches('#');
    if s.len() < 6 {
        return (128, 128, 128);
    }
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(128);
    (r, g, b)
}

fn to_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

fn mix(c1: &str, c2: &str, pct: f32) -> String {
    // pct = weight of c1
    let (r1, g1, b1) = parse_hex(c1);
    let (r2, g2, b2) = parse_hex(c2);
    let r = (r1 as f32 * pct + r2 as f32 * (1.0 - pct)) as u8;
    let g = (g1 as f32 * pct + g2 as f32 * (1.0 - pct)) as u8;
    let b = (b1 as f32 * pct + b2 as f32 * (1.0 - pct)) as u8;
    to_hex(r, g, b)
}

fn lighten(c: &str, amt: f32) -> String {
    mix(c, "#ffffff", 1.0 - amt)
}

fn variant_of(text: &str) -> String {
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("variant:") {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    "dark".to_string()
}

fn scheme_text(pkg: &ThemePackage) -> Option<String> {
    let path = themes::resolve_scheme_path(pkg)?;
    std::fs::read_to_string(&path).ok()
}

fn scheme_colors(pkg: &ThemePackage) -> Option<(String, String, Vec<String>, Vec<String>, String)> {
    // (bg, fg, accents8, brights8, variant)
    let text = scheme_text(pkg)?;
    let slots = theme_engine::parse_tinty_yaml_slots(&text);
    if slots.is_empty() {
        eprintln!("aspire-themes: no colors parsed for {}", pkg.id);
        return None;
    }
    let get = |key: &str| -> String {
        slots
            .get(key)
            .map(|s| format!("#{}", s))
            .unwrap_or_else(|| "#888888".to_string())
    };
    let bg = get("base00");
    let fg = get("base05");
    let mut accents = Vec::new();
    for key in [
        "base08", "base09", "base0a", "base0b", "base0c", "base0d", "base0e", "base0f",
    ] {
        accents.push(get(key));
    }
    let mut brights = Vec::new();
    for key in ["base12", "base13", "base14", "base15", "base16", "base17"] {
        brights.push(get(key));
    }
    Some((bg, fg, accents, brights, variant_of(&text)))
}

fn active_theme_id() -> Option<String> {
    if theme_engine::active_engine() != theme_engine::Engine::Tinty {
        return None;
    }
    let scheme = theme_engine::tinty_scheme()?;
    let after = scheme.split_once('-')?.1;
    let id = after.strip_prefix("aspire-")?;
    Some(id.to_string())
}

fn bright_for(
    pkg: &ThemePackage,
    accent_idx: usize,
    accent_hex: &str,
    brights: &[String],
) -> String {
    let system = if pkg.manifest.system.is_empty() {
        "base24"
    } else {
        pkg.manifest.system.as_str()
    };
    if system == "base24" {
        // mapping accent index -> bright slot
        let bright_idx = match accent_idx {
            0 => Some(0), // base08 red -> base12
            2 => Some(1), // base0A yellow -> base13
            3 => Some(2), // base0B green -> base14
            4 => Some(3), // base0C cyan -> base15
            5 => Some(4), // base0D blue -> base16
            6 => Some(5), // base0E magenta -> base17
            _ => None,
        };
        if let Some(bi) = bright_idx {
            if bi < brights.len() {
                return brights[bi].clone();
            }
        }
    }
    lighten(accent_hex, 0.18)
}

fn count_wallpapers(pkg: &ThemePackage) -> usize {
    let dir = pkg.root.join("wallpapers");
    let exts = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];
    std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_file())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| exts.contains(&x.to_lowercase().as_str()))
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn include_names(pkg: &ThemePackage) -> Vec<String> {
    let mut names = Vec::new();
    for dir in [pkg.root.join("include"), pkg.root.join("templates")] {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let path = e.path();
                if !path.is_file() {
                    continue;
                }
                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let base = fname.strip_suffix(".tpl").unwrap_or(fname);
                // text before first "-" or "."
                let mut cut = base.len();
                for (i, c) in base.char_indices() {
                    if c == '-' || c == '.' {
                        cut = i;
                        break;
                    }
                }
                let name = &base[..cut];
                if !name.is_empty() && !names.contains(&name.to_string()) {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

fn make_row(name: &str, is_active: bool) -> ListBoxRow {
    let row_box = GtkBox::new(Orientation::Horizontal, 4);
    let label = Label::new(Some(name));
    label.set_xalign(0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_hexpand(true);
    label.add_css_class("themes-item");
    row_box.append(&label);
    if is_active {
        let dot = Label::new(Some("●"));
        dot.add_css_class("themes-item-active");
        row_box.append(&dot);
    }
    let row = ListBoxRow::new();
    row.set_child(Some(&row_box));
    row
}

fn rebuild_list(ui: &Ui) {
    ui.list.remove_all();
    let shown = ui.shown.borrow();
    if shown.is_empty() {
        return;
    }
    let active = active_theme_id();
    for &idx in shown.iter() {
        let pkg = &ui.themes[idx];
        let is_active = active.as_deref() == Some(pkg.id.as_str());
        ui.list.append(&make_row(&pkg.manifest.name, is_active));
    }
    let cur = *ui.current.borrow();
    if let Some(row) = ui.list.row_at_index(cur as i32) {
        ui.list.select_row(Some(&row));
    }
}

fn apply_preview_css(ui: &Ui, css: &str) {
    let display = gtk::gdk::Display::default().unwrap();
    gtk::style_context_remove_provider_for_display(&display, ui.preview_provider.as_ref());
    ui.preview_provider.load_from_string(css);
    gtk::style_context_add_provider_for_display(
        &display,
        ui.preview_provider.as_ref(),
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
    ui.preview.queue_draw();
    ui.window.queue_draw();
}

fn update_preview(ui: &Ui) {
    while let Some(child) = ui.preview.first_child() {
        ui.preview.remove(&child);
    }
    let shown = ui.shown.borrow();
    let cur = *ui.current.borrow();
    let Some(&idx) = shown.get(cur) else {
        return;
    };
    let pkg = &ui.themes[idx].clone();
    drop(shown);

    let colors = scheme_colors(&pkg);
    if colors.is_none() {
        // header still
        let header = GtkBox::new(Orientation::Horizontal, 8);
        let name_lbl = Label::new(Some(&pkg.manifest.name));
        name_lbl.add_css_class("tp-name");
        name_lbl.set_halign(gtk::Align::Start);
        name_lbl.set_hexpand(true);
        let badge = Label::new(Some(&format!("{} · dark", pkg.manifest.system)));
        badge.add_css_class("tp-badge");
        badge.set_halign(gtk::Align::End);
        header.append(&name_lbl);
        header.append(&badge);
        ui.preview.append(&header);

        let lbl = Label::new(Some("Scheme not found"));
        lbl.add_css_class("tp-notfound");
        lbl.set_halign(gtk::Align::Start);
        ui.preview.append(&lbl);

        let hint = Label::new(Some("Enter apply · Esc close"));
        hint.add_css_class("tp-footer");
        hint.set_halign(gtk::Align::Start);
        ui.preview.append(&hint);
        return;
    }
    let (bg, fg, accents, brights, variant) = colors.unwrap();

    // css
    let mut css = String::new();
    css.push_str(&format!(
        ".themes-preview {{ background-color: {bg}; color: {fg}; }}\n",
        bg = bg,
        fg = fg
    ));
    // bars container bands
    for (i, acc) in accents.iter().enumerate() {
        let bright = bright_for(&pkg, i, acc, &brights);
        let b1 = mix(acc, &bright, 0.75);
        let b2 = mix(acc, &bright, 0.50);
        let b3 = mix(acc, &bright, 0.25);
        // bottom to top: acc, 75/25, 50/50, 25/75, bright
        let bands = [acc.clone(), b1, b2, b3, bright];
        for (bi, col) in bands.iter().enumerate() {
            css.push_str(&format!(
                ".tp-bar-{i}-band-{bi} {{ background-color: {col}; }}\n",
                i = i,
                bi = bi,
                col = col
            ));
        }
    }
    apply_preview_css(ui, &css);

    // header
    let header = GtkBox::new(Orientation::Horizontal, 8);
    let name_lbl = Label::new(Some(&pkg.manifest.name));
    name_lbl.add_css_class("tp-name");
    name_lbl.set_halign(gtk::Align::Start);
    name_lbl.set_hexpand(true);
    let badge = Label::new(Some(&format!("{} · {}", pkg.manifest.system, variant)));
    badge.add_css_class("tp-badge");
    badge.set_halign(gtk::Align::End);
    header.append(&name_lbl);
    header.append(&badge);
    ui.preview.append(&header);

    // bars
    let bars = GtkBox::new(Orientation::Horizontal, 4);
    bars.set_margin_top(8);
    bars.set_halign(gtk::Align::Start);
    for i in 0..8usize {
        let acc = accents
            .get(i)
            .cloned()
            .unwrap_or_else(|| "#888888".to_string());
        let bar = GtkBox::new(Orientation::Vertical, 0);
        bar.set_size_request(40, 150);
        // bands bottom to top: we append bottom first? GtkBox vertical appends top-to-bottom.
        // We want bottom-to-top bands: bottom=base, top=bright. So append bright first then ... base?
        // Easier: create vertical box and append bands from top to bottom: bright, 25/75, 50/50, 75/25, base.
        let bright = bright_for(&pkg, i, &acc, &brights);
        let b1 = mix(&acc, &bright, 0.75);
        let b2 = mix(&acc, &bright, 0.50);
        let b3 = mix(&acc, &bright, 0.25);
        let bands_top_to_bottom = [bright.clone(), b3, b2, b1, acc.clone()];
        for (bi_rev, col) in bands_top_to_bottom.iter().enumerate() {
            // map to band index 4..0
            let band_idx = 4 - bi_rev;
            let band = GtkBox::new(Orientation::Vertical, 0);
            band.add_css_class(&format!("tp-bar-{}-band-{}", i, band_idx));
            band.set_size_request(40, 30);
            band.set_vexpand(false);
            // tooltip on each band? We'll set on whole bar instead.
            bar.append(&band);
        }
        // tooltip on bar
        bar.set_tooltip_text(Some(&acc));
        bars.append(&bar);
    }
    ui.preview.append(&bars);

    // info rows
    let mk_row = |label: &str, value: String, muted: bool| -> GtkBox {
        let row = GtkBox::new(Orientation::Horizontal, 6);
        row.set_margin_top(4);
        let lab = Label::new(Some(label));
        lab.add_css_class("tp-rowlabel");
        lab.set_size_request(96, -1);
        lab.set_halign(gtk::Align::Start);
        let val = Label::new(Some(&value));
        val.add_css_class("tp-rowval");
        if muted {
            val.set_opacity(0.5);
        }
        val.set_halign(gtk::Align::Start);
        val.set_hexpand(true);
        row.append(&lab);
        row.append(&val);
        row
    };

    // Font
    let (font_val, font_muted) = if let Some(f) = &pkg.manifest.font {
        (f.family.clone(), false)
    } else {
        ("Global".to_string(), true)
    };
    ui.preview.append(&mk_row("Font:", font_val, font_muted));

    // Icons
    let (icon_val, icon_muted) = if let Some(ic) = &pkg.manifest.icons {
        (ic.name.clone(), false)
    } else {
        ("Global".to_string(), true)
    };
    ui.preview.append(&mk_row("Icons:", icon_val, icon_muted));

    // Wallpapers
    ui.preview.append(&mk_row(
        "Wallpapers:",
        count_wallpapers(&pkg).to_string(),
        false,
    ));

    // Includes
    let inames = include_names(&pkg);
    let (inc_val, inc_muted) = if inames.is_empty() {
        ("none".to_string(), true)
    } else {
        (inames.join(" · "), false)
    };
    ui.preview.append(&mk_row("Includes:", inc_val, inc_muted));

    let hint = Label::new(Some("Enter apply · Esc close"));
    hint.add_css_class("tp-footer");
    hint.set_halign(gtk::Align::Start);
    hint.set_margin_top(6);
    ui.preview.append(&hint);
}

fn move_selection(ui: &Ui, delta: isize) {
    let shown_len = ui.shown.borrow().len();
    if shown_len == 0 {
        return;
    }
    let mut cur = *ui.current.borrow();
    let new = if delta > 0 {
        (cur + delta as usize).min(shown_len - 1)
    } else {
        cur.saturating_sub((-delta) as usize)
    };
    if new == cur {
        return;
    }
    *ui.current.borrow_mut() = new;
    if let Some(row) = ui.list.row_at_index(new as i32) {
        ui.list.select_row(Some(&row));
        let alloc = row.allocation();
        let view_h = ui.list_scroll.allocation().height();
        if alloc.height() > 0 && view_h > 0 {
            let adj = ui.list_scroll.vadjustment();
            let target = (alloc.y() + alloc.height() / 2 - view_h / 2) as f64;
            let max = (adj.upper() - adj.page_size()).max(0.0);
            adj.set_value(target.clamp(0.0, max));
        }
    }
    update_preview(ui);
}

fn apply_current(ui: &Ui) {
    let shown = ui.shown.borrow();
    let cur = *ui.current.borrow();
    let Some(&idx) = shown.get(cur) else {
        return;
    };
    let pkg = ui.themes[idx].clone();
    drop(shown);
    if let Err(error) = themes::apply_theme(&pkg) {
        eprintln!("aspire-themes: apply failed: {error:#}");
    }
    ui.window.close();
}

fn connect_signals(ui: &Ui, sender: ComponentSender<ThemesApp>) {
    let key = gtk::EventControllerKey::new();
    key.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let sender = sender.clone();
        key.connect_key_pressed(move |_, keyval, _, state| {
            let alt = state.contains(ModifierType::ALT_MASK);
            let ctrl = state.contains(ModifierType::CONTROL_MASK);
            if alt || ctrl {
                if keyval == Key::j {
                    sender.input(AppMsg::MoveSelection(1));
                    return Propagation::Stop;
                }
                if keyval == Key::k {
                    sender.input(AppMsg::MoveSelection(-1));
                    return Propagation::Stop;
                }
                if keyval == Key::l {
                    sender.input(AppMsg::Apply);
                    return Propagation::Stop;
                }
                if alt && keyval == Key::h {
                    sender.input(AppMsg::Close);
                    return Propagation::Stop;
                }
                return Propagation::Proceed;
            }
            if keyval == Key::Escape {
                sender.input(AppMsg::Close);
                Propagation::Stop
            } else if keyval == Key::Return || keyval == Key::KP_Enter {
                sender.input(AppMsg::Apply);
                Propagation::Stop
            } else if keyval == Key::Up {
                sender.input(AppMsg::MoveSelection(-1));
                Propagation::Stop
            } else if keyval == Key::Down {
                sender.input(AppMsg::MoveSelection(1));
                Propagation::Stop
            } else {
                Propagation::Proceed
            }
        });
    }
    {
        let ui_s = ui.clone();
        let sender = sender.clone();
        ui.list.connect_selected_rows_changed(move |_| {
            let idx = {
                let st = ui_s.shown.borrow();
                (0..st.len() as i32).find_map(|i| {
                    ui_s.list
                        .row_at_index(i)
                        .filter(|r| r.is_selected())
                        .map(|_| i as usize)
                })
            };
            if let Some(i) = idx {
                sender.input(AppMsg::Select(i));
            }
        });
    }
    ui.window.add_controller(key);
}

fn build_ui(
    window: ApplicationWindow,
    packages: Vec<ThemePackage>,
    sender: ComponentSender<ThemesApp>,
) -> Ui {
    window.add_css_class("themes-window");
    window.set_decorated(false);

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("aspire-themes"));
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_exclusive_zone(0);

    let card = GtkBox::new(Orientation::Horizontal, 0);
    card.add_css_class("themes-card");

    // left list
    let left = GtkBox::new(Orientation::Vertical, 0);
    left.set_size_request(LEFT_W, -1);
    left.set_hexpand(false);
    let list_scroll = ScrolledWindow::new();
    list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    list_scroll.set_vexpand(true);
    list_scroll.set_hexpand(false);
    list_scroll.set_size_request(LEFT_W, -1);
    let list = ListBox::new();
    list.add_css_class("themes-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    list_scroll.set_child(Some(&list));
    left.append(&list_scroll);

    let sep = gtk::Separator::new(Orientation::Vertical);

    // right preview
    let preview_scroll = ScrolledWindow::new();
    preview_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);
    preview_scroll.set_hexpand(true);
    preview_scroll.set_vexpand(true);
    preview_scroll.set_size_request(PREVIEW_W, -1);
    let preview = GtkBox::new(Orientation::Vertical, 0);
    preview.add_css_class("themes-preview");
    preview_scroll.set_child(Some(&preview));

    card.append(&left);
    card.append(&sep);
    card.append(&preview_scroll);
    window.set_child(Some(&card));

    let provider = CssProvider::new();
    provider.load_from_string(&build_css());
    let display = gtk::gdk::Display::default().expect("no display");
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let preview_provider = Rc::new(CssProvider::new());
    gtk::style_context_add_provider_for_display(
        &display,
        preview_provider.as_ref(),
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let engine_matugen = theme_engine::active_engine() == theme_engine::Engine::Matugen;

    let themes_vec = packages;
    let shown: Vec<usize> = (0..themes_vec.len()).collect();
    let ui = Ui {
        window: window.clone(),
        list: list.clone(),
        list_scroll: list_scroll.clone(),
        preview: preview.clone(),
        preview_provider,
        themes: Rc::new(themes_vec),
        shown: Rc::new(RefCell::new(shown)),
        current: Rc::new(RefCell::new(0)),
        engine_matugen,
    };

    connect_signals(&ui, sender);
    rebuild_list(&ui);
    update_preview(&ui);

    window.set_default_size(WIN_W, WIN_H);
    ui
}

fn main() -> glib::ExitCode {
    unsafe {
        std::env::set_var("GSK_RENDERER", "cairo");
    }
    let packages = themes::discover_themes();
    RelmApp::from_app(Application::builder().build())
        .with_args(Vec::new())
        .run::<ThemesApp>(packages);
    glib::ExitCode::SUCCESS
}
