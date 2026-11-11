#![allow(dead_code, unused_variables, unused_mut, deprecated)]
use aspire_core::colors::{self, RgbColor};
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Box as GtkBox, ContentFit, CssProvider, Entry, Label, ListBox, ListBoxRow,
    Orientation, Picture, PolicyType, ScrolledWindow, Separator, Stack, TextView, WrapMode,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use relm4::prelude::*;
use std::collections::HashMap;

/// Tamaño fijo del panel y ancho de la lista.
const WIN_W: i32 = 880;
const WIN_H: i32 = 520;
const LEFT_W: i32 = 438;
const MAX_ITEMS: usize = 40;

#[derive(Clone)]
struct Item {
    id: u64,
    summary: String,
    is_image: bool,
    mime: String,
}

#[derive(Clone)]
enum Decoded {
    Text(String),
    Image(gdk_pixbuf::Pixbuf),
}

struct Clip {
    all: Vec<Item>,
    shown: Vec<Item>,
    current: usize,
    cache: HashMap<u64, Decoded>,
}

struct App {
    clip: Clip,
    query: String,
    window: ApplicationWindow,
    entry: Entry,
    list: ListBox,
    list_scroll: ScrolledWindow,
    stack: Stack,
    text: TextView,
    picture: Picture,
    text_scroll: ScrolledWindow,
    image_scroll: ScrolledWindow,
    css_provider: CssProvider,
}

#[derive(Debug)]
enum AppMsg {
    SearchChanged(String),
    MoveUp,
    MoveDown,
    CopyCurrent,
    Close,
    RowSelected(usize),
}

// ---------- helpers (unchanged from original) ----------

fn hex(c: Option<RgbColor>, fallback: &str) -> String {
    c.map(|v| format!("#{:02x}{:02x}{:02x}", v.r, v.g, v.b))
        .unwrap_or_else(|| fallback.to_string())
}

fn build_css() -> String {
    let colors = colors::load_theme_colors();
    let bg = hex(colors.background, "#1e1e2e");
    let alt = hex(colors.background_alt, "#313244");
    let fg = hex(colors.foreground, "#cdd6f4");
    let sel = hex(colors.selected, "#cba6f7");
    let seltext = hex(colors.selectedtext, "#000000");
    format!(
        r#"
.clip-window {{ background-color: transparent; border: none; font-family: "JetBrainsMono Nerd Font"; }}
.clip-card {{ background-color: {bg}; border: 1px solid {sel}; border-radius: 0; }}
.clip-search {{ min-height: 50px; border-bottom: 1px solid {alt}; padding-left: 20px; padding-right: 20px; background: transparent; }}
.clip-entry {{ background: transparent; border: none; box-shadow: none; outline: none; color: {fg}; font-size: 17px; padding: 0px; }}
.clip-entry placeholder {{ color: {fg}; opacity: 0.9; }}
.clip-list {{ background: transparent; padding: 10px 14px 16px 14px; }}
.clip-list row {{ min-height: 40px; background: transparent; border-radius: 0px; outline: none; }}
.clip-list row:selected {{ background-color: {sel}; }}
.clip-list row:selected label {{ color: {seltext}; }}
.clip-item {{ color: {fg}; font-size: 15px; }}
.clip-sep {{ background-color: {alt}; }}
.clip-preview {{ color: {fg}; font-size: 15px; background: transparent; }}
.clip-preview text {{ background: transparent; color: {fg}; }}
.clip-hint {{ color: {fg}; font-size: 12px; opacity: 0.6; padding: 6px 20px 10px 20px; }}
"#
    )
}

fn parse_line(line: &str) -> Option<Item> {
    let (id, preview) = line.split_once('\t')?;
    let id: u64 = id.trim().parse().ok()?;
    let preview = preview.trim();
    if preview.starts_with("[[ binary data") {
        let parts: Vec<&str> = preview.split_whitespace().collect();
        let (mime, dims) = match parts.as_slice() {
            [.., mime, dims, _] => (*mime, *dims),
            _ => ("img", ""),
        };
        let summary = if dims.is_empty() {
            "imagen".to_string()
        } else {
            format!("imagen {mime} {dims}")
        };
        Some(Item {
            id,
            summary: format!("{} {summary}", '\u{f03e}'),
            is_image: true,
            mime: mime.to_string(),
        })
    } else {
        let first = preview.lines().next().unwrap_or("").trim();
        let mut s: String = first.chars().take(40).collect();
        if first.chars().count() > 40 {
            s.push('…');
        }
        if s.is_empty() {
            s = "(vacío)".to_string();
        }
        Some(Item {
            id,
            summary: s,
            is_image: false,
            mime: "text/plain".to_string(),
        })
    }
}

fn load_history() -> Vec<Item> {
    let out = std::process::Command::new("cliphist").arg("list").output();
    let Ok(out) = out else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .take(MAX_ITEMS)
        .filter_map(parse_line)
        .collect()
}

fn decode_item(item: &Item) -> Option<Decoded> {
    let out = std::process::Command::new("cliphist")
        .args(["decode", &item.id.to_string()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    if item.is_image {
        let loader = gdk_pixbuf::PixbufLoader::new();
        loader.write(&out.stdout).ok()?;
        loader.close().ok()?;
        let px = loader.pixbuf()?;
        let (w, h) = (px.width() as f64, px.height() as f64);
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let scale = (1280.0 / w).min(720.0 / h).min(1.0);
        let scaled = px.scale_simple(
            (w * scale) as i32,
            (h * scale) as i32,
            gdk_pixbuf::InterpType::Bilinear,
        )?;
        Some(Decoded::Image(scaled))
    } else {
        Some(Decoded::Text(String::from_utf8_lossy(&out.stdout).into_owned()))
    }
}

fn make_row(summary: &str) -> ListBoxRow {
    let label = Label::new(Some(summary));
    label.set_xalign(0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_max_width_chars(40);
    label.set_margin_start(12);
    label.set_margin_end(12);
    label.add_css_class("clip-item");
    let row = ListBoxRow::new();
    row.set_child(Some(&label));
    row
}

fn refresh_list(app: &mut App) {
    while let Some(child) = app.list.first_child() {
        app.list.remove(&child);
    }
    let shown = app.clip.shown.clone();
    if shown.is_empty() {
        let row = make_row("Sin resultados");
        row.set_activatable(false);
        row.set_selectable(false);
        app.list.append(&row);
    } else {
        for it in &shown {
            app.list.append(&make_row(&it.summary));
        }
        if let Some(row) = app.list.row_at_index(0) {
            app.list.select_row(Some(&row));
        }
    }
}

fn update_preview(app: &mut App) {
    let item = app.clip.shown.get(app.clip.current).cloned();
    let Some(item) = item else {
        app.stack.set_visible_child(&app.text_scroll);
        app.text.buffer().set_text("");
        return;
    };
    // check cache
    if let Some(cached) = app.clip.cache.get(&item.id) {
        match cached {
            Decoded::Image(px) => {
                let tex = gtk::gdk::Texture::for_pixbuf(px);
                app.picture.set_paintable(Some(&tex));
                app.stack.set_visible_child(&app.image_scroll);
            }
            Decoded::Text(t) => {
                app.text.buffer().set_text(t);
                app.stack.set_visible_child(&app.text_scroll);
            }
        }
        return;
    }
    let out = std::process::Command::new("cliphist")
        .args(["decode", &item.id.to_string()])
        .output();
    let Ok(out) = out else {
        app.stack.set_visible_child(&app.text_scroll);
        app.text.buffer().set_text("(no se pudo leer)");
        return;
    };
    if !out.status.success() || out.stdout.is_empty() {
        app.stack.set_visible_child(&app.text_scroll);
        app.text.buffer().set_text("(no se pudo leer)");
        return;
    }
    if item.is_image {
        let loader = gdk_pixbuf::PixbufLoader::new();
        if loader.write(&out.stdout).is_ok() && loader.close().is_ok() {
            if let Some(px) = loader.pixbuf() {
                let (w, h) = (px.width() as f64, px.height() as f64);
                if w > 0.0 && h > 0.0 {
                    let scale = (1280.0 / w).min(720.0 / h).min(1.0);
                    if let Some(scaled) = px.scale_simple(
                        (w * scale) as i32,
                        (h * scale) as i32,
                        gdk_pixbuf::InterpType::Bilinear,
                    ) {
                        let tex = gtk::gdk::Texture::for_pixbuf(&scaled);
                        app.picture.set_paintable(Some(&tex));
                        app.stack.set_visible_child(&app.image_scroll);
                        app.clip.cache.insert(item.id, Decoded::Image(scaled));
                        return;
                    }
                }
            }
        }
        app.stack.set_visible_child(&app.text_scroll);
        app.text.buffer().set_text("(no se pudo leer imagen)");
    } else {
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        app.text.buffer().set_text(&text);
        app.stack.set_visible_child(&app.text_scroll);
        app.clip.cache.insert(item.id, Decoded::Text(text));
    }
}

fn copy_current(app: &mut App) {
    let item = app.clip.shown.get(app.clip.current).cloned();
    let Some(item) = item else { return; };
    let out = std::process::Command::new("cliphist")
        .args(["decode", &item.id.to_string()])
        .output();
    let Ok(out) = out else { return; };
    if !out.status.success() || out.stdout.is_empty() {
        return;
    }
    let mut cmd = std::process::Command::new("wl-copy");
    if item.is_image {
        let mime = if item.mime.contains('/') {
            item.mime.clone()
        } else {
            format!("image/{}", item.mime)
        };
        cmd.args(["--type", &mime]);
    }
    if let Ok(mut child) = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(&out.stdout);
        }
        let _ = child.wait();
    }
    app.window.close();
}

fn scroll_to_row(ui_list: &ListBox, ui_scroll: &ScrolledWindow, row: &ListBoxRow) {
    let alloc = row.allocation();
    let view_h = ui_scroll.allocation().height();
    if alloc.height() <= 0 || view_h <= 0 {
        return;
    }
    let adj = ui_scroll.vadjustment();
    let target = (alloc.y() + alloc.height() / 2 - view_h / 2) as f64;
    let max = (adj.upper() - adj.page_size()).max(0.0);
    adj.set_value(target.clamp(0.0, max));
}

// ---------- relm4 component ----------

#[relm4::component]
impl SimpleComponent for App {
    type Init = Vec<Item>;
    type Input = AppMsg;
    type Output = ();
    type Widgets = AppWidgets;

    view! {
        gtk::ApplicationWindow {
            set_title: Some("Portapapeles Aspire"),
            set_default_size: (WIN_W, WIN_H),
            set_decorated: false,
        }
    }

    fn init(init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let items = init;
        let shown = items.clone();
        let mut model = App {
            clip: Clip { all: items, shown, current: 0, cache: HashMap::new() },
            query: String::new(),
            window: root.clone(),
            entry: gtk::Entry::new(),
            list: gtk::ListBox::new(),
            list_scroll: ScrolledWindow::new(),
            stack: gtk::Stack::new(),
            text: gtk::TextView::new(),
            text_scroll: gtk::ScrolledWindow::new(),
            picture: gtk::Picture::new(),
            image_scroll: gtk::ScrolledWindow::new(),
            css_provider: gtk::CssProvider::new(),
        };

        // build UI imperatively
        let card = GtkBox::new(Orientation::Horizontal, 0);
        card.add_css_class("clip-card");
        let left = GtkBox::new(Orientation::Vertical, 0);
        left.set_size_request(LEFT_W, -1);
        let search = GtkBox::new(Orientation::Horizontal, 0);
        search.add_css_class("clip-search");
        search.append(&model.entry);
        model.entry.set_placeholder_text(Some("Search..."));
        model.entry.add_css_class("clip-entry");
        model.entry.set_hexpand(true);
        model.entry.set_valign(gtk::Align::Center);
        let list_scroll = ScrolledWindow::new();
        list_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        list_scroll.set_vexpand(true);
        list_scroll.set_hexpand(true);
        list_scroll.set_max_content_width(LEFT_W - 48);
        list_scroll.set_child(Some(&model.list));
        model.list.add_css_class("clip-list");
        model.list.set_selection_mode(gtk::SelectionMode::Single);
        left.append(&search);
        left.append(&list_scroll);
        let sep = Separator::new(Orientation::Vertical);
        sep.add_css_class("clip-sep");
        let right = GtkBox::new(Orientation::Vertical, 0);
        right.set_hexpand(true);
        right.set_size_request(440, -1);
        let stack = model.stack.clone();
        stack.set_vexpand(true);
        stack.set_hexpand(true);
        let text_scroll = model.text_scroll.clone();
        text_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
        text_scroll.set_hexpand(true);
        text_scroll.set_vexpand(true);
        model.text.set_editable(false);
        model.text.set_cursor_visible(false);
        model.text.set_wrap_mode(WrapMode::Word);
        model.text.set_monospace(true);
        model.text.set_top_margin(16);
        model.text.set_left_margin(20);
        model.text.set_right_margin(20);
        model.text.add_css_class("clip-preview");
        text_scroll.set_child(Some(&model.text));
        let image_scroll = model.image_scroll.clone();
        image_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
        image_scroll.set_hexpand(true);
        image_scroll.set_vexpand(true);
        image_scroll.set_min_content_width(420);
        image_scroll.set_min_content_height(380);
        model.picture.set_content_fit(ContentFit::ScaleDown);
        model.picture.set_hexpand(true);
        model.picture.set_vexpand(true);
        image_scroll.set_child(Some(&model.picture));
        stack.add_named(&text_scroll, Some("text"));
        stack.add_named(&image_scroll, Some("image"));
        right.append(&stack);
        card.append(&left);
        card.append(&sep);
        card.append(&right);
        model.window.set_child(Some(&card));
        model.list_scroll = list_scroll;

        // layer shell
        model.window.init_layer_shell();
        model.window.set_layer(Layer::Overlay);
        model.window.set_namespace(Some("aspire-clip"));
        model.window.set_keyboard_mode(KeyboardMode::Exclusive);
        model.window.set_exclusive_zone(0);

        // css
        model.css_provider.load_from_data(&build_css());
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, &model.css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        // watch theme-engine/current for live color updates
        let theme_dir = gtk::gio::File::for_path(&format!("{}/.config/theme-engine/current", std::env::var("HOME").unwrap_or_default()));
        if let Ok(monitor) = theme_dir.monitor_directory(gtk::gio::FileMonitorFlags::NONE, gtk::gio::Cancellable::NONE) {
            let provider = model.css_provider.clone();
            monitor.connect_changed(move |_, _, _, _| {
                provider.load_from_data(&build_css());
            });
            // keep monitor alive by leaking (simple)
            std::mem::forget(monitor);
        }

        // wire entry
        let sender_clone = sender.clone();
        model.entry.connect_changed(move |e| {
            sender_clone.input(AppMsg::SearchChanged(e.text().to_string()));
        });

        // list selection
        let sender_sel = sender.clone();
        model.list.connect_selected_rows_changed(move |list| {
            let idx = (0..list.observe_children().n_items() as i32).find_map(|i| {
                list.row_at_index(i).filter(|r| r.is_selected()).map(|_| i as usize)
            });
            if let Some(i) = idx {
                sender_sel.input(AppMsg::RowSelected(i));
            }
        });

        // key controller
        let key = gtk::EventControllerKey::new();
        key.set_propagation_phase(gtk::PropagationPhase::Capture);
        let s = sender.clone();
        let w = model.window.clone();
        key.connect_key_pressed(move |_, keyval, _, state| {
            use gtk::gdk::Key;
            let ctrl_alt = state.contains(gtk::gdk::ModifierType::CONTROL_MASK) || state.contains(gtk::gdk::ModifierType::ALT_MASK);
            if ctrl_alt {
                match keyval {
                    Key::h => {}
                    Key::j => { s.input(AppMsg::MoveDown); }
                    Key::k => { s.input(AppMsg::MoveUp); }
                    Key::l => { s.input(AppMsg::CopyCurrent); }
                    _ => {}
                }
                return gtk::glib::Propagation::Stop;
            }
            match keyval {
                Key::Escape => { s.input(AppMsg::Close); gtk::glib::Propagation::Stop }
                Key::Return | Key::KP_Enter => { s.input(AppMsg::CopyCurrent); gtk::glib::Propagation::Stop }
                Key::Up => { s.input(AppMsg::MoveUp); gtk::glib::Propagation::Stop }
                Key::Down => { s.input(AppMsg::MoveDown); gtk::glib::Propagation::Stop }
                _ => gtk::glib::Propagation::Proceed,
            }
        });
        model.window.add_controller(key);

        // populate list
        // (refresh_list uses model.list which is now the view's list)
        // We'll populate after view_output assigns widgets. For now, store model.

        model.window.set_default_size(WIN_W, WIN_H);
        model.window.present();
        model.entry.grab_focus();

        // populate list
        refresh_list(&mut model);
        update_preview(&mut model);

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::SearchChanged(q) => {
                self.query = q.to_lowercase();
                self.clip.shown = if self.query.is_empty() {
                    self.clip.all.clone()
                } else {
                    self.clip.all.iter().filter(|it| it.summary.to_lowercase().contains(&self.query)).cloned().collect()
                };
                self.clip.current = 0;
                refresh_list(self);
                update_preview(self);
            }
            AppMsg::MoveUp => {
                let n = self.clip.shown.len() as isize;
                if n == 0 { return; }
                self.clip.current = (self.clip.current as isize - 1).rem_euclid(n) as usize;
                if let Some(row) = self.list.row_at_index(self.clip.current as i32) {
                    self.list.select_row(Some(&row));
                    scroll_to_row(&self.list, &self.list_scroll, &row);
                }
                update_preview(self);
            }
            AppMsg::MoveDown => {
                let n = self.clip.shown.len() as isize;
                if n == 0 { return; }
                self.clip.current = (self.clip.current as isize + 1).rem_euclid(n) as usize;
                if let Some(row) = self.list.row_at_index(self.clip.current as i32) {
                    self.list.select_row(Some(&row));
                    scroll_to_row(&self.list, &self.list_scroll, &row);
                }
                update_preview(self);
            }
            AppMsg::CopyCurrent => {
                copy_current(self);
            }
            AppMsg::Close => {
                self.window.close();
            }
            AppMsg::RowSelected(i) => {
                self.clip.current = i;
                update_preview(self);
            }
        }
    }
}

fn main() {
    unsafe { std::env::set_var("GSK_RENDERER", "cairo"); }
    let items = load_history();
    let app = RelmApp::new("com.aspire.clip");
    app.run::<App>(items);
}
