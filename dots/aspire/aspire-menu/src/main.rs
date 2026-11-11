use aspire_menu::menu_logic::{self, RgbColor};
use aspire_menu::model::{ActionType, AppState, MenuItem, MenuTree};
use gtk::gdk::{Key, ModifierType};
use gtk::glib::{self, Propagation};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, CssProvider, Entry, Label, ListBox, ListBoxRow,
    Orientation, PolicyType, ScrolledWindow,
};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use relm4::{ComponentParts, ComponentSender, RelmApp, SimpleComponent};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Ancho fijo de la ventana y duración de la animación de altura.
const WIN_W: i32 = 320;
const ANIM_SECS: f64 = 0.160;

struct Menu {
    state: AppState,
    query: String,
    engine: bool,
    current: usize,
    items: Vec<MenuItem>,
    height: f64,
    target_height: i32,
    anim_from: f64,
    list_height: f64,
    list_target: i32,
    list_from: f64,
    restart: bool,
    animating: bool,
}

#[derive(Clone)]
struct Ui {
    window: ApplicationWindow,
    card: GtkBox,
    scroll: ScrolledWindow,
    entry: Entry,
    list: ListBox,
    menu: Rc<RefCell<Menu>>,
}

#[derive(Debug)]
enum AppMsg {
    QueryChanged(String),
    MoveSelection(isize),
    GoBack,
    Activate,
    ActivateAt(usize),
    Close,
}

struct MenuApp {
    ui: Ui,
}

impl SimpleComponent for MenuApp {
    type Init = MenuTree;
    type Input = AppMsg;
    type Output = ();
    type Widgets = ();
    type Root = ApplicationWindow;

    fn init_root() -> Self::Root {
        ApplicationWindow::builder().title("Aspire Menu").build()
    }

    fn init(
        tree: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let ui = build_ui(root, tree, sender);
        ComponentParts {
            model: Self { ui },
            widgets: (),
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::QueryChanged(query) => {
                self.ui.menu.borrow_mut().query = query;
                refresh(&self.ui);
            }
            AppMsg::MoveSelection(delta) => move_selection(&self.ui, delta),
            AppMsg::GoBack => go_back(&self.ui),
            AppMsg::Activate => activate_current(&self.ui),
            AppMsg::ActivateAt(index) => {
                self.ui.menu.borrow_mut().current = index;
                activate_current(&self.ui);
            }
            AppMsg::Close => self.ui.window.close(),
        }
    }
}

fn hex(c: Option<RgbColor>, fallback: &str) -> String {
    c.map(|v| format!("#{:02x}{:02x}{:02x}", v.r, v.g, v.b))
        .unwrap_or_else(|| fallback.to_string())
}

fn build_css() -> String {
    let colors = menu_logic::load_theme_colors();
    let bg = hex(colors.background, "#1e1e2e");
    let alt = hex(colors.background_alt, "#313244");
    let fg = hex(colors.foreground, "#cdd6f4");
    let sel = hex(colors.selected, "#cba6f7");
    let seltext = hex(colors.selectedtext, "#000000");
    format!(
        r#"
.aspire-window {{ background-color: transparent; border: none; font-family: "JetBrainsMono Nerd Font"; }}
.aspire-card {{ background-color: {bg}; border: 1px solid {sel}; border-radius: 0; }}
.aspire-header {{ min-height: 50px; border-bottom: 1px solid {alt}; padding-left: 20px; padding-right: 20px; background: transparent; }}
.aspire-entry {{ background: transparent; border: none; box-shadow: none; outline: none; color: {fg}; font-size: 17px; padding: 0px; }}
.aspire-entry placeholder {{ color: {fg}; opacity: 0.9; }}
.aspire-entry.no-caret {{ caret-color: transparent; }}
.aspire-list {{ background: transparent; padding: 10px 14px 16px 14px; }}
.aspire-scroll {{ background: transparent; border: none; }}
.aspire-scroll viewport {{ background: transparent; border: none; }}
.aspire-scroll scrollbar {{ opacity: 0; }}
.aspire-scroll scrollbar slider {{ min-width: 0; min-height: 0; }}
.aspire-list row {{ min-height: 42px; background: transparent; border-radius: 0px; outline: none; }}
.aspire-list row:selected {{ background-color: {sel}; }}
.aspire-list row:selected label {{ color: {seltext}; }}
.aspire-icon, .aspire-label, .aspire-indicator {{ color: {fg}; font-size: 16px; }}
.aspire-path {{ color: {fg}; font-size: 11px; opacity: 0.6; }}
"#
    )
}

fn make_row(item: &MenuItem, path: &str, submenu_mark: bool) -> ListBoxRow {
    let hbox = GtkBox::new(Orientation::Horizontal, 10);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);

    let icon = Label::new(Some(item.icon.as_str()));
    icon.set_size_request(24, -1);
    icon.add_css_class("aspire-icon");

    let label = Label::new(Some(item.label.as_str()));
    label.set_xalign(0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.add_css_class("aspire-label");

    let path_label = Label::new(Some(path));
    path_label.set_xalign(1.0);
    path_label.set_hexpand(true);
    path_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    path_label.add_css_class("aspire-path");

    let mark = if item.is_active {
        "✓"
    } else if submenu_mark {
        "›"
    } else {
        ""
    };
    let indicator = Label::new(Some(mark));
    indicator.add_css_class("aspire-indicator");

    hbox.append(&icon);
    hbox.append(&label);
    hbox.append(&path_label);
    hbox.append(&indicator);

    let row = ListBoxRow::new();
    row.set_child(Some(&hbox));
    row
}

fn refresh(ui: &Ui) {
    let built: Vec<(MenuItem, String, bool)>;
    let prompt: String;
    let query: String;
    {
        let mut m = ui.menu.borrow_mut();
        let query_empty = m.query.is_empty();
        let at_root = !m.engine && query_empty && m.state.level_stack.is_empty();
        let q = m.query.clone();
        let eng = m.engine;
        let items = menu_logic::visible(&mut m.state, &q, eng);
        built = items
            .into_iter()
            .map(|item| {
                let path = if query_empty {
                    String::new()
                } else {
                    menu_logic::parent_path(&m.state, &item)
                };
                let mark = item.is_submenu || at_root;
                (item, path, mark)
            })
            .collect();
        m.items = built.iter().map(|(it, _, _)| it.clone()).collect();
        m.current = 0;
        prompt = if m.engine {
            "Engine".to_string()
        } else {
            m.state.current_prompt()
        };
        query = m.query.clone();
    }
    // Fijar el alto visible actual ANTES de tocar la lista: GTK agranda
    // sola la ventana si el contenido crece, pero no la encoge si se
    // reduce. Sin este tope, al borrar (crecer) saltaba de golpe mientras
    // que al escribir (encoger) sí se veía la animación.
    {
        let m = ui.menu.borrow();
        ui.scroll
            .set_max_content_height(m.list_height.round() as i32);
    }
    ui.list.remove_all();
    for (item, path, mark) in &built {
        ui.list.append(&make_row(item, path, *mark));
    }
    if let Some(row) = ui.list.row_at_index(0) {
        ui.list.select_row(Some(&row));
    }
    let (natural_h, list_nat) = measure_targets(ui);
    {
        let mut m = ui.menu.borrow_mut();
        m.target_height = natural_h;
        m.list_target = list_nat;
        if m.height == 0.0 {
            m.height = natural_h as f64;
            m.list_height = list_nat as f64;
            ui.window.set_default_size(WIN_W, natural_h);
            ui.scroll.set_max_content_height(-1);
        }
    }
    ensure_anim(ui);
    ui.entry.set_placeholder_text(Some(prompt.as_str()));
    if query.is_empty() {
        ui.entry.add_css_class("no-caret");
    } else {
        ui.entry.remove_css_class("no-caret");
    }
    if ui.entry.text().as_str() != query {
        ui.entry.set_text(&query);
    }
}

/// Mide la altura real del contenido (ventana y lista) sin que el tope
/// la recorte. Todo ocurre dentro del mismo turno del bucle principal,
/// sin fotograma entre medias: quitar y reponer el tope no se ve ni mueve
/// la ventana, solo sirve para que `measure` devuelva el tamaño verdadero.
fn measure_targets(ui: &Ui) -> (i32, i32) {
    ui.scroll.set_max_content_height(100_000);
    let (_, win_h, _, _) = ui.card.measure(Orientation::Vertical, WIN_W);
    let (_, list_h, _, _) = ui.list.measure(Orientation::Vertical, WIN_W);
    ui.scroll
        .set_max_content_height(ui.menu.borrow().list_height.round() as i32);
    (win_h, list_h)
}

/// Anima la superficie (ease-out cúbico) hasta la altura objetivo.
/// La superficie es opaca e igual que la tarjeta, así que no queda zona
/// transparente que el blur convierta en franja. Si el objetivo cambia a
/// mitad del movimiento (seguir escribiendo, entrar/salir de nivel),
/// retoma desde la posición actual sin saltos.
fn ensure_anim(ui: &Ui) {
    {
        let mut m = ui.menu.borrow_mut();
        m.anim_from = m.height;
        m.list_from = m.list_height;
        m.restart = true;
        if m.animating || (m.target_height as f64 - m.height).abs() < 0.5 {
            return;
        }
        m.animating = true;
    }
    let last_win = Cell::new(ui.menu.borrow().height.round() as i32);
    let last_list = Cell::new(ui.menu.borrow().list_height.round() as i32);
    let ui_t = ui.clone();
    let start = Rc::new(RefCell::new(Instant::now()));
    glib::timeout_add_local(Duration::from_millis(8), move || {
        let (from, target, lfrom, ltarget) = {
            let mut m = ui_t.menu.borrow_mut();
            if m.restart {
                *start.borrow_mut() = Instant::now();
                m.restart = false;
            }
            (
                m.anim_from,
                m.target_height as f64,
                m.list_from,
                m.list_target as f64,
            )
        };
        let t = (start.borrow().elapsed().as_secs_f64() / ANIM_SECS).min(1.0);
        let e = 1.0 - (1.0 - t).powi(3);
        let h = (from + (target - from) * e).round() as i32;
        let lh = (lfrom + (ltarget - lfrom) * e).round() as i32;
        {
            let mut m = ui_t.menu.borrow_mut();
            m.height = h as f64;
            m.list_height = lh as f64;
        }
        if h != last_win.get() {
            ui_t.window.set_default_size(WIN_W, h);
            last_win.set(h);
        }
        if lh != last_list.get() {
            ui_t.scroll.set_max_content_height(lh);
            last_list.set(lh);
        }
        if t >= 1.0 {
            ui_t.menu.borrow_mut().animating = false;
            ui_t.scroll.set_max_content_height(-1);
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

fn fade_in(window: &ApplicationWindow) {
    let w = window.clone();
    let start = Instant::now();
    let dur = Duration::from_millis(40);
    glib::timeout_add_local(Duration::from_millis(16), move || {
        let t = (start.elapsed().as_secs_f32() / dur.as_secs_f32()).min(1.0);
        w.set_opacity(t as f64);
        if t >= 1.0 {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

fn move_selection(ui: &Ui, delta: isize) {
    let next = {
        let mut m = ui.menu.borrow_mut();
        let n = m.items.len() as isize;
        if n == 0 {
            return;
        }
        m.current = (m.current as isize).wrapping_add(delta).rem_euclid(n) as usize;
        m.current as i32
    };
    if let Some(row) = ui.list.row_at_index(next) {
        ui.list.select_row(Some(&row));
    }
}

fn go_back(ui: &Ui) {
    {
        let mut m = ui.menu.borrow_mut();
        if !m.query.is_empty() {
            m.query.clear();
        } else if m.engine {
            m.engine = false;
        } else if !m.state.go_back() {
            return;
        }
    }
    refresh(ui);
}

fn activate_current(ui: &Ui) {
    let item = {
        ui.menu
            .borrow()
            .items
            .get(ui.menu.borrow().current)
            .cloned()
    };
    let Some(item) = item else { return };

    if ui.menu.borrow().engine || item.id == "matugen" || item.id == "tinty" {
        // El motor de color ahora vive en aspire-bg (Rust), no en un script.
        let bg = dirs::home_dir()
            .map(|home| home.join(".local/bin/aspire-bg"))
            .filter(|path| path.is_file())
            .unwrap_or_else(|| std::path::PathBuf::from("aspire-bg"));
        let log = dirs::home_dir()
            .map(|home| home.join(".cache/aspire-bg"))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let _ = std::fs::create_dir_all(&log);
        let log_file = std::fs::File::create(log.join("engine.log")).ok();
        let mut command = std::process::Command::new("setsid");
        command
            .arg(bg)
            .args(["--set-engine", &item.id])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(match log_file {
                Some(file) => std::process::Stdio::from(file),
                None => std::process::Stdio::null(),
            });
        let _ = command.spawn();
        ui.window.close();
        return;
    }

    if item.label == "Engine" {
        let mut m = ui.menu.borrow_mut();
        m.engine = true;
        m.query.clear();
        drop(m);
        refresh(ui);
        return;
    }

    match item.action_type {
        ActionType::Submenu => {
            let target = item.label.to_lowercase().replace(' ', "_");
            let mut m = ui.menu.borrow_mut();
            if m.state.go_to(&target) {
                m.query.clear();
                drop(m);
                refresh(ui);
            }
        }
        ActionType::Action => {
            if let Some(script) = item.script_path.as_ref() {
                let expanded = shellexpand::tilde(script).to_string();
                let _ = std::process::Command::new("bash")
                    .arg("-c")
                    .arg(expanded)
                    .spawn();
            }
            ui.window.close();
        }
        ActionType::Toggle => {}
    }
}

fn connect_signals(ui: &Ui, sender: ComponentSender<MenuApp>) {
    {
        let sender = sender.clone();
        ui.entry.connect_changed(move |e| {
            sender.input(AppMsg::QueryChanged(e.text().to_string()));
        });
    }
    {
        let sender = sender.clone();
        ui.list.connect_row_activated(move |_, row| {
            sender.input(AppMsg::ActivateAt(row.index() as usize));
        });
    }
    let key = gtk::EventControllerKey::new();
    key.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let sender = sender.clone();
        key.connect_key_pressed(move |_, keyval, _, state| {
            let ctrl_alt = state.contains(ModifierType::CONTROL_MASK)
                || state.contains(ModifierType::ALT_MASK);
            if ctrl_alt {
                if keyval == Key::h {
                    sender.input(AppMsg::GoBack);
                    return Propagation::Stop;
                }
                if keyval == Key::j {
                    sender.input(AppMsg::MoveSelection(1));
                    return Propagation::Stop;
                }
                if keyval == Key::k {
                    sender.input(AppMsg::MoveSelection(-1));
                    return Propagation::Stop;
                }
                if keyval == Key::l {
                    sender.input(AppMsg::Activate);
                    return Propagation::Stop;
                }
                return Propagation::Proceed;
            }
            if keyval == Key::Escape {
                sender.input(AppMsg::Close);
                Propagation::Stop
            } else if keyval == Key::Return || keyval == Key::KP_Enter {
                sender.input(AppMsg::Activate);
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
    ui.window.add_controller(key);
}

fn build_ui(window: ApplicationWindow, tree: MenuTree, sender: ComponentSender<MenuApp>) -> Ui {
    window.add_css_class("aspire-window");
    window.set_size_request(320, -1);
    window.set_decorated(false);

    // Capa centrada que sigue al contenido: sin anclas, el compositor
    // centra la superficie y la vuelve a centrar cuando cambia su tamaño.
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("aspire-menu"));
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_exclusive_zone(0);

    let vbox = GtkBox::new(Orientation::Vertical, 0);
    vbox.add_css_class("aspire-card");
    vbox.set_overflow(gtk::Overflow::Hidden);

    let header = GtkBox::new(Orientation::Horizontal, 0);
    header.add_css_class("aspire-header");
    let entry = Entry::new();
    entry.add_css_class("aspire-entry");
    entry.set_hexpand(true);
    entry.set_valign(gtk::Align::Center);
    header.append(&entry);

    let list = ListBox::new();
    list.add_css_class("aspire-list");
    list.set_selection_mode(gtk::SelectionMode::Single);

    // Contenedor con tope de alto: evita que GTK agrande sola la ventana
    // al crecer la lista (al borrar), para que el crecimiento también lo
    // lleve la animación fotograma a fotograma. Con vscroll Automatic el
    // mínimo no propaga el contenido (puede desplazar) y no hay salto;
    // la barra se oculta por CSS.
    let scroll = ScrolledWindow::new();
    scroll.add_css_class("aspire-scroll");
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_propagate_natural_height(true);
    scroll.set_overlay_scrolling(true);
    scroll.set_has_frame(false);
    scroll.set_overflow(gtk::Overflow::Hidden);
    scroll.set_child(Some(&list));

    vbox.append(&header);
    vbox.append(&scroll);
    window.set_child(Some(&vbox));

    let provider = CssProvider::new();
    provider.load_from_string(&build_css());
    let display = gtk::gdk::Display::default().expect("aspire-menu: sin display");
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let theme_dir = gtk::gio::File::for_path(&format!(
        "{}/.config/theme-engine/current",
        std::env::var("HOME").unwrap_or_default()
    ));
    if let Ok(monitor) = theme_dir.monitor_directory(gtk::gio::FileMonitorFlags::NONE, gtk::gio::Cancellable::NONE) {
        let p = provider.clone();
        monitor.connect_changed(move |_, _, _, _| {
            p.load_from_string(&build_css());
        });
        std::mem::forget(monitor);
    }

    let ui = Ui {
        window: window.clone(),
        card: vbox.clone(),
        scroll: scroll.clone(),
        entry: entry.clone(),
        list: list.clone(),
        menu: Rc::new(RefCell::new(Menu {
            state: AppState::new(tree),
            query: String::new(),
            engine: false,
            current: 0,
            items: Vec::new(),
            height: 0.0,
            target_height: 0,
            anim_from: 0.0,
            list_height: 0.0,
            list_target: 0,
            list_from: 0.0,
            restart: false,
            animating: false,
        })),
    };

    connect_signals(&ui, sender);
    refresh(&ui);
    window.set_opacity(0.0);
    fade_in(&window);
    entry.grab_focus();
    ui
}

fn main() -> glib::ExitCode {
    let Some(home) = dirs::home_dir() else {
        eprintln!("aspire-menu: sin directorio home");
        return glib::ExitCode::FAILURE;
    };
    let path = std::env::var("ASPIRE_MENU_CONFIG")
        .unwrap_or_else(|_| format!("{}/.config/rofi-menu/menu.toml", home.display()));
    let tree = match MenuTree::load(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("aspire-menu: no pude cargar {path}: {e:#}");
            return glib::ExitCode::FAILURE;
        }
    };

    RelmApp::from_app(Application::builder().build())
        .with_args(Vec::new())
        .run::<MenuApp>(tree);
    glib::ExitCode::SUCCESS
}
