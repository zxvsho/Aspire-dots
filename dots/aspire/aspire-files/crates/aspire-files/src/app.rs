//! Ventana principal y estado de aspire-files.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gdk, gio, glib, pango};

use aspire_config::{Config, Engine};
use aspire_fs::{Bookmark, Entry, ListOptions, SortBy, TrashEntry};

use crate::dialogs;
use crate::theme;
use crate::views;

/// Resultado de una búsqueda: generación y entradas (o error).
type SearchResult = (u64, Result<Vec<Entry>, String>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Files,
    Trash,
}

#[derive(Clone, Debug)]
pub struct Tab {
    pub path: PathBuf,
    pub history: Vec<PathBuf>,
    pub index: usize,
    pub title: String,
    pub mode: Mode,
}

impl Tab {
    fn new(path: PathBuf) -> Self {
        Self {
            title: title_for(&path),
            history: vec![path.clone()],
            index: 0,
            mode: mode_for_path(&path),
            path,
        }
    }
}

#[derive(Clone, Debug)]
enum SidebarTarget {
    Header,
    Place(PathBuf),
    Trash,
    Bookmark(usize),
}

#[derive(Clone, Debug)]
struct Clipboard {
    paths: Vec<PathBuf>,
    cut: bool,
}

pub struct App {
    pub window: gtk::ApplicationWindow,
    config: RefCell<Config>,
    tabs: RefCell<Vec<Tab>>,
    active: Cell<usize>,
    tab_strip: gtk::Box,
    sidebar: gtk::ListBox,
    sidebar_revealer: gtk::Revealer,
    sidebar_targets: RefCell<Vec<SidebarTarget>>,
    selected_bookmark: Cell<Option<usize>>,
    stack: gtk::Stack,
    list_store: gio::ListStore,
    grid_store: gio::ListStore,
    list_selection: gtk::MultiSelection,
    grid_selection: gtk::MultiSelection,
    entries: RefCell<Vec<Entry>>,
    trash: RefCell<Vec<TrashEntry>>,
    mode: Cell<Mode>,
    current: RefCell<PathBuf>,
    path_bar: gtk::Box,
    center_stack: gtk::Stack,
    location: gtk::Entry,
    search_entry: gtk::SearchEntry,
    search_revealer: gtk::Revealer,
    search_global: gtk::CheckButton,
    status: gtk::Label,
    back_button: gtk::Button,
    forward_button: gtk::Button,
    up_button: gtk::Button,
    view_button: gtk::Button,
    clipboard: RefCell<Option<Clipboard>>,
    bookmarks: RefCell<Vec<Bookmark>>,
    theme_provider: RefCell<Option<gtk::CssProvider>>,
    theme_monitors: RefCell<Vec<gio::FileMonitor>>,
    thumb_memory: RefCell<HashMap<PathBuf, Option<PathBuf>>>,
    search_generation: Cell<u64>,
    search_rx: RefCell<Option<Receiver<SearchResult>>>,
    suppress_search: Cell<bool>,
    icon_size: Cell<i32>,
    date_detailed: Rc<Cell<bool>>,
    list_view: gtk::ColumnView,
    grid_view: gtk::GridView,
}

fn title_for(path: &Path) -> String {
    if path == aspire_fs::home_dir() {
        return "Home".into();
    }
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn mode_for_path(path: &Path) -> Mode {
    if path == aspire_fs::trash_root() {
        Mode::Trash
    } else {
        Mode::Files
    }
}

pub fn build(application: &gtk::Application, initial: Option<PathBuf>) -> Rc<App> {
    let config = Config::load().unwrap_or_default();
    let bookmarks = aspire_fs::read_bookmarks();
    let start = initial
        .filter(|path| path.is_dir())
        .unwrap_or_else(aspire_fs::home_dir);
    let start_mode = mode_for_path(&start);

    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("Files")
        .default_width(1100)
        .default_height(720)
        .build();

    // Cabecera ------------------------------------------------------------
    let header = gtk::HeaderBar::new();
    let back_button = gtk::Button::from_icon_name("go-previous-symbolic");
    back_button.set_tooltip_text(Some("Back"));
    let forward_button = gtk::Button::from_icon_name("go-next-symbolic");
    forward_button.set_tooltip_text(Some("Forward"));
    let up_button = gtk::Button::from_icon_name("go-up-symbolic");
    up_button.set_tooltip_text(Some("Up"));
    header.pack_start(&back_button);
    header.pack_start(&forward_button);
    header.pack_start(&up_button);

    let path_bar = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    path_bar.set_hexpand(true);
    let location = gtk::Entry::new();
    location.set_hexpand(true);
    location.set_placeholder_text(Some("Type a path"));
    let location_revealer = gtk::Revealer::new();
    location_revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
    location_revealer.set_child(Some(&location));
    location_revealer.set_reveal_child(true);
    let center_stack = gtk::Stack::new();
    center_stack.set_hexpand(true);
    center_stack.add_named(&path_bar, Some("path"));
    center_stack.add_named(&location_revealer, Some("location"));
    center_stack.set_visible_child_name("path");
    header.set_title_widget(Some(&center_stack));

    let search_button = gtk::Button::from_icon_name("system-search-symbolic");
    search_button.set_tooltip_text(Some("Search (Ctrl+F)"));
    let view_button = gtk::Button::from_icon_name("view-grid-symbolic");
    view_button.set_tooltip_text(Some("Toggle grid/list (Ctrl+1 / Ctrl+2)"));
    let menu_button = gtk::MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_tooltip_text(Some("Menu"));
    header.pack_end(&menu_button);
    header.pack_end(&view_button);
    header.pack_end(&search_button);
    window.set_titlebar(Some(&header));

    // Barra de pestañas ---------------------------------------------------
    let tab_strip = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    tab_strip.add_css_class("tab-strip");

    // Buscador ------------------------------------------------------------
    let search_entry = gtk::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search files…"));
    search_entry.set_hexpand(true);
    let search_global = gtk::CheckButton::with_label("Global");
    search_global.set_tooltip_text(Some("Search subfolders (Ctrl+Shift+F)"));
    search_global.set_active(config.view.search_subfolders);
    let search_close = gtk::Button::from_icon_name("window-close-symbolic");
    search_close.set_tooltip_text(Some("Close search"));
    let search_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    search_box.set_margin_top(6);
    search_box.set_margin_bottom(6);
    search_box.set_margin_start(10);
    search_box.set_margin_end(10);
    search_box.append(&search_entry);
    search_box.append(&search_global);
    search_box.append(&search_close);
    let search_revealer = gtk::Revealer::new();
    search_revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
    search_revealer.set_child(Some(&search_box));

    // Vistas --------------------------------------------------------------
    let list_store = gio::ListStore::new::<glib::BoxedAnyObject>();
    let grid_store = gio::ListStore::new::<glib::BoxedAnyObject>();
    let list_selection = gtk::MultiSelection::new(Some(list_store.clone()));
    let grid_selection = gtk::MultiSelection::new(Some(grid_store.clone()));

    let icon_size = config.view.icon_size.clamp(48, 160);
    let date_detailed = Rc::new(Cell::new(config.view.date_format == "detailed"));
    let single_click = config.general.open_action == "single";

    let list_view = gtk::ColumnView::new(Some(list_selection.clone()));
    list_view.set_show_row_separators(false);
    list_view.set_show_column_separators(false);
    list_view.set_single_click_activate(single_click);
    let name_column =
        gtk::ColumnViewColumn::new(Some("Name"), Some(views::name_factory()));
    name_column.set_expand(true);
    name_column.set_resizable(true);
    let size_column = gtk::ColumnViewColumn::new(Some("Size"), Some(views::size_factory()));
    size_column.set_fixed_width(110);
    let modified_column = gtk::ColumnViewColumn::new(
        Some("Modified"),
        Some(views::modified_factory(date_detailed.clone())),
    );
    modified_column.set_fixed_width(if config.view.date_format == "detailed" {
        240
    } else {
        170
    });
    modified_column.set_resizable(true);
    list_view.append_column(&name_column);
    list_view.append_column(&size_column);
    list_view.append_column(&modified_column);

    let grid_view = gtk::GridView::new(
        None::<gtk::MultiSelection>,
        None::<gtk::SignalListItemFactory>,
    );
    grid_view.set_single_click_activate(single_click);
    let list_scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .build();
    list_scroller.set_child(Some(&list_view));
    let grid_scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .hexpand(true)
        .build();
    grid_scroller.set_child(Some(&grid_view));

    let stack = gtk::Stack::new();
    stack.set_vexpand(true);
    stack.set_hexpand(true);
    stack.add_named(&list_scroller, Some("list"));
    stack.add_named(&grid_scroller, Some("grid"));

    // Barra lateral -------------------------------------------------------
    let sidebar = gtk::ListBox::new();
    sidebar.set_selection_mode(gtk::SelectionMode::Single);
    sidebar.add_css_class("sidebar");
    let sidebar_scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();
    sidebar_scroller.set_child(Some(&sidebar));
    sidebar_scroller.set_width_request(210);

    let sidebar_actions = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    sidebar_actions.set_margin_top(4);
    sidebar_actions.set_margin_bottom(4);
    sidebar_actions.set_margin_start(8);
    sidebar_actions.set_margin_end(8);
    let add_bookmark = gtk::Button::from_icon_name("bookmark-new-symbolic");
    add_bookmark.set_tooltip_text(Some("Add current folder to bookmarks"));
    let remove_bookmark = gtk::Button::from_icon_name("user-trash-symbolic");
    remove_bookmark.set_tooltip_text(Some("Remove selected bookmark"));
    let bookmark_up = gtk::Button::from_icon_name("go-up-symbolic");
    bookmark_up.set_tooltip_text(Some("Move bookmark up"));
    let bookmark_down = gtk::Button::from_icon_name("go-down-symbolic");
    bookmark_down.set_tooltip_text(Some("Move bookmark down"));
    sidebar_actions.append(&add_bookmark);
    sidebar_actions.append(&remove_bookmark);
    sidebar_actions.append(&bookmark_up);
    sidebar_actions.append(&bookmark_down);

    let sidebar_column = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_column.append(&sidebar_scroller);
    sidebar_column.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    sidebar_column.append(&sidebar_actions);
    let sidebar_revealer = gtk::Revealer::new();
    sidebar_revealer.set_transition_type(gtk::RevealerTransitionType::SlideRight);
    sidebar_revealer.set_child(Some(&sidebar_column));
    sidebar_revealer.set_reveal_child(config.window.sidebar_visible);

    // Estado --------------------------------------------------------------
    let status = gtk::Label::new(Some(""));
    status.set_xalign(0.0);
    status.add_css_class("statusbar");

    // Ensamblado ----------------------------------------------------------
    let main_column = gtk::Box::new(gtk::Orientation::Vertical, 0);
    main_column.set_hexpand(true);
    main_column.set_vexpand(true);
    main_column.append(&tab_strip);
    main_column.append(&search_revealer);
    main_column.append(&stack);
    main_column.append(&status);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    content.set_vexpand(true);
    content.append(&sidebar_revealer);
    content.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    content.append(&main_column);
    window.set_child(Some(&content));

    // Menú ----------------------------------------------------------------
    let menu = gio::Menu::new();

    let file_menu = gio::Menu::new();
    file_menu.append(Some("New Window"), Some("win.new-window"));
    file_menu.append(Some("New Tab"), Some("win.new-tab"));
    file_menu.append(Some("New Folder…"), Some("win.new-folder"));
    file_menu.append(Some("Rename…"), Some("win.rename"));
    file_menu.append(Some("Close Tab"), Some("win.close-tab"));
    file_menu.append(Some("Preferences"), Some("win.preferences"));
    menu.append_submenu(Some("File"), &file_menu);

    let edit_menu = gio::Menu::new();
    edit_menu.append(Some("Copy"), Some("win.copy"));
    edit_menu.append(Some("Cut"), Some("win.cut"));
    edit_menu.append(Some("Paste"), Some("win.paste"));
    edit_menu.append(Some("Select All"), Some("win.select-all"));
    menu.append_submenu(Some("Edit"), &edit_menu);

    let view_menu = gio::Menu::new();
    view_menu.append(Some("Grid"), Some("win.view-grid"));
    view_menu.append(Some("List"), Some("win.view-list"));
    view_menu.append(Some("Show Hidden Files"), Some("win.toggle-hidden"));
    view_menu.append(Some("Sidebar"), Some("win.toggle-sidebar"));
    view_menu.append(Some("Refresh"), Some("win.refresh"));
    menu.append_submenu(Some("View"), &view_menu);

    let go_menu = gio::Menu::new();
    go_menu.append(Some("Back"), Some("win.back"));
    go_menu.append(Some("Forward"), Some("win.forward"));
    go_menu.append(Some("Up"), Some("win.up"));
    go_menu.append(Some("Home"), Some("win.home"));
    go_menu.append(Some("Location…"), Some("win.location"));
    menu.append_submenu(Some("Go"), &go_menu);

    let trash_menu = gio::Menu::new();
    trash_menu.append(Some("Move to Trash"), Some("win.trash"));
    trash_menu.append(Some("Delete Permanently…"), Some("win.delete"));
    trash_menu.append(Some("Restore"), Some("win.restore"));
    trash_menu.append(Some("Empty Trash…"), Some("win.empty-trash"));
    menu.append_submenu(Some("Trash"), &trash_menu);

    let tools_menu = gio::Menu::new();
    tools_menu.append(Some("Open With…"), Some("win.open-with"));
    tools_menu.append(Some("Properties"), Some("win.properties"));
    tools_menu.append(Some("Make Executable"), Some("win.make-executable"));
    menu.append_submenu(Some("Tools"), &tools_menu);
    menu_button.set_menu_model(Some(&menu));

    let app = Rc::new(App {
        window: window.clone(),
        config: RefCell::new(config),
        tabs: RefCell::new(vec![Tab::new(start.clone())]),
        active: Cell::new(0),
        tab_strip,
        sidebar,
        sidebar_revealer,
        sidebar_targets: RefCell::new(Vec::new()),
        selected_bookmark: Cell::new(None),
        stack,
        list_store,
        grid_store,
        list_selection,
        grid_selection,
        entries: RefCell::new(Vec::new()),
        trash: RefCell::new(Vec::new()),
        mode: Cell::new(start_mode),
        current: RefCell::new(start),
        path_bar,
        center_stack: center_stack.clone(),
        location,
        search_entry,
        search_revealer,
        search_global,
        status,
        back_button,
        forward_button,
        up_button,
        view_button,
        clipboard: RefCell::new(None),
        bookmarks: RefCell::new(bookmarks),
        theme_provider: RefCell::new(None),
        theme_monitors: RefCell::new(Vec::new()),
        thumb_memory: RefCell::new(HashMap::new()),
        search_generation: Cell::new(0),
        search_rx: RefCell::new(None),
        suppress_search: Cell::new(false),
        icon_size: Cell::new(icon_size),
        date_detailed,
        list_view: list_view.clone(),
        grid_view: grid_view.clone(),
    });

    grid_view.set_model(Some(&app.grid_selection));
    grid_view.set_factory(Some(&views::grid_factory(&app, icon_size)));

    app.connect_signals(application, &list_view, &grid_view);
    app.install_actions(application);

    // Botones de la barra lateral.
    add_bookmark.connect_clicked({
        let app = app.clone();
        move |_| app.add_bookmark()
    });
    remove_bookmark.connect_clicked({
        let app = app.clone();
        move |_| app.remove_bookmark()
    });
    bookmark_up.connect_clicked({
        let app = app.clone();
        move |_| app.move_bookmark(-1)
    });
    bookmark_down.connect_clicked({
        let app = app.clone();
        move |_| app.move_bookmark(1)
    });
    search_button.connect_clicked({
        let app = app.clone();
        move |_| app.toggle_search(false)
    });
    search_close.connect_clicked({
        let app = app.clone();
        move |_| app.close_search()
    });

    app.rebuild_sidebar();
    app.rebuild_tabs();
    app.apply_view_mode();
    app.apply_sidebar();
    app.reload_theme();
    app.watch_theme();
    app.load();

    window.present();
    app
}

impl App {
    fn connect_signals(
        self: &Rc<Self>,
        application: &gtk::Application,
        list_view: &gtk::ColumnView,
        grid_view: &gtk::GridView,
    ) {
        let _ = application;

        self.back_button.connect_clicked({
            let app = self.clone();
            move |_| app.go_back()
        });
        self.forward_button.connect_clicked({
            let app = self.clone();
            move |_| app.go_forward()
        });
        self.up_button.connect_clicked({
            let app = self.clone();
            move |_| app.go_up()
        });
        self.view_button.connect_clicked({
            let app = self.clone();
            move |_| app.toggle_view()
        });

        self.search_entry.connect_search_changed({
            let app = self.clone();
            move |_| app.on_search_changed()
        });
        self.search_entry.connect_stop_search({
            let app = self.clone();
            move |_| app.close_search()
        });
        self.search_global.connect_toggled({
            let app = self.clone();
            move |_| {
                {
                    let mut config = app.config.borrow_mut();
                    config.view.search_subfolders = app.search_global.is_active();
                    let _ = config.save();
                }
                if !app.search_entry.text().is_empty() {
                    app.on_search_changed();
                }
            }
        });

        self.location.connect_activate({
            let app = self.clone();
            move |entry| app.navigate_from_entry(&entry.text())
        });
        let location_keys = gtk::EventControllerKey::new();
        location_keys.connect_key_pressed({
            let app = self.clone();
            move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    app.hide_location();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        });
        self.location.add_controller(location_keys);

        self.sidebar.connect_row_activated({
            let app = self.clone();
            move |_, row| app.activate_sidebar_row(row.index())
        });
        self.sidebar.connect_row_selected({
            let app = self.clone();
            move |_, row| {
                let bookmark = row.and_then(|row| {
                    app.sidebar_targets
                        .borrow()
                        .get(row.index() as usize)
                        .cloned()
                        .and_then(|target| match target {
                            SidebarTarget::Bookmark(index) => Some(index),
                            _ => None,
                        })
                });
                app.selected_bookmark.set(bookmark);
            }
        });

        list_view.connect_activate({
            let app = self.clone();
            move |_, _| app.open_selected()
        });
        grid_view.connect_activate({
            let app = self.clone();
            move |_, _| app.open_selected()
        });

        self.list_selection.connect_selection_changed({
            let app = self.clone();
            move |_, _, _| app.update_status(None)
        });
        self.grid_selection.connect_selection_changed({
            let app = self.clone();
            move |_, _, _| app.update_status(None)
        });

        self.window.connect_close_request({
            let app = self.clone();
            move |_| {
                app.search_generation.set(app.search_generation.get() + 1);
                glib::Propagation::Proceed
            }
        });
    }

    fn install_actions(self: &Rc<Self>, application: &gtk::Application) {
        let add = |name: &str, callback: Box<dyn Fn()>| {
            let action = gio::SimpleAction::new(name, None);
            action.connect_activate(move |_, _| callback());
            self.window.add_action(&action);
        };

        add(
            "new-window",
            {
                let app = self.clone();
                Box::new(move || app.new_window())
            },
        );
        add(
            "new-tab",
            {
                let app = self.clone();
                Box::new(move || app.new_tab())
            },
        );
        add(
            "close-tab",
            {
                let app = self.clone();
                Box::new(move || app.close_tab(app.active.get()))
            },
        );
        add(
            "new-folder",
            {
                let app = self.clone();
                Box::new(move || app.new_folder())
            },
        );
        add(
            "rename",
            {
                let app = self.clone();
                Box::new(move || app.rename_selected())
            },
        );
        add(
            "copy",
            {
                let app = self.clone();
                Box::new(move || app.copy_selected(false))
            },
        );
        add(
            "cut",
            {
                let app = self.clone();
                Box::new(move || app.copy_selected(true))
            },
        );
        add(
            "paste",
            {
                let app = self.clone();
                Box::new(move || app.paste())
            },
        );
        add(
            "select-all",
            {
                let app = self.clone();
                Box::new(move || app.select_all())
            },
        );
        add(
            "trash",
            {
                let app = self.clone();
                Box::new(move || app.trash_selected())
            },
        );
        add(
            "delete",
            {
                let app = self.clone();
                Box::new(move || app.delete_selected())
            },
        );
        add(
            "restore",
            {
                let app = self.clone();
                Box::new(move || app.restore_selected())
            },
        );
        add(
            "empty-trash",
            {
                let app = self.clone();
                Box::new(move || app.empty_trash())
            },
        );
        add(
            "properties",
            {
                let app = self.clone();
                Box::new(move || app.show_properties())
            },
        );
        add(
            "open-with",
            {
                let app = self.clone();
                Box::new(move || app.open_with())
            },
        );
        add(
            "make-executable",
            {
                let app = self.clone();
                Box::new(move || app.make_executable())
            },
        );
        add(
            "preferences",
            {
                let app = self.clone();
                Box::new(move || app.preferences())
            },
        );
        add(
            "refresh",
            {
                let app = self.clone();
                Box::new(move || app.refresh())
            },
        );
        add(
            "back",
            {
                let app = self.clone();
                Box::new(move || app.go_back())
            },
        );
        add(
            "forward",
            {
                let app = self.clone();
                Box::new(move || app.go_forward())
            },
        );
        add(
            "up",
            {
                let app = self.clone();
                Box::new(move || app.go_up())
            },
        );
        add(
            "home",
            {
                let app = self.clone();
                Box::new(move || app.navigate(aspire_fs::home_dir()))
            },
        );
        add(
            "location",
            {
                let app = self.clone();
                Box::new(move || app.show_location())
            },
        );
        add(
            "toggle-search",
            {
                let app = self.clone();
                Box::new(move || app.toggle_search(false))
            },
        );
        add(
            "search-global",
            {
                let app = self.clone();
                Box::new(move || app.toggle_search(true))
            },
        );
        add(
            "view-grid",
            {
                let app = self.clone();
                Box::new(move || app.set_view_mode("grid"))
            },
        );
        add(
            "view-list",
            {
                let app = self.clone();
                Box::new(move || app.set_view_mode("list"))
            },
        );
        add(
            "toggle-sidebar",
            {
                let app = self.clone();
                Box::new(move || app.toggle_sidebar())
            },
        );
        add(
            "toggle-hidden",
            {
                let app = self.clone();
                Box::new(move || app.toggle_hidden())
            },
        );

        application.set_accels_for_action("win.new-tab", &["<Control>t"]);
        application.set_accels_for_action("win.close-tab", &["<Control>w"]);
        application.set_accels_for_action("win.new-folder", &["<Control><Shift>n"]);
        application.set_accels_for_action("win.rename", &["F2"]);
        application.set_accels_for_action("win.copy", &["<Control>c"]);
        application.set_accels_for_action("win.cut", &["<Control>x"]);
        application.set_accels_for_action("win.paste", &["<Control>v"]);
        application.set_accels_for_action("win.select-all", &["<Control>a"]);
        application.set_accels_for_action("win.trash", &["Delete"]);
        application.set_accels_for_action("win.delete", &["<Shift>Delete"]);
        application.set_accels_for_action("win.refresh", &["F5", "<Control>r"]);
        application.set_accels_for_action("win.preferences", &["<Control>comma"]);
        application.set_accels_for_action("win.location", &["<Control>l"]);
        application.set_accels_for_action("win.toggle-search", &["<Control>f"]);
        application.set_accels_for_action("win.search-global", &["<Control><Shift>f"]);
        application.set_accels_for_action("win.view-grid", &["<Control>1"]);
        application.set_accels_for_action("win.view-list", &["<Control>2"]);
        application.set_accels_for_action("win.toggle-sidebar", &["F9"]);
        application.set_accels_for_action("win.home", &["<Alt>Home"]);
    }

    // ---------------------------------------------------------------------
    // Carga y navegación
    // ---------------------------------------------------------------------

    fn list_options(&self) -> ListOptions {
        let config = self.config.borrow();
        ListOptions {
            show_hidden: config.view.show_hidden,
            folders_first: config.general.folders_first,
            sort_by: SortBy::parse(&config.view.sort_by),
            sort_descending: config.view.sort_descending,
        }
    }

    fn load(self: &Rc<Self>) {
        if self.mode.get() == Mode::Trash {
            self.load_trash();
            return;
        }
        let path = self.current.borrow().clone();
        let options = self.list_options();
        match aspire_fs::list_dir(&path, options) {
            Ok(entries) => self.display(entries, None),
            Err(err) => {
                self.set_error(&format!("Cannot open {}: {err}", path.display()));
                self.display(Vec::new(), None);
            }
        }
    }

    fn load_trash(self: &Rc<Self>) {
        match aspire_fs::list_trash() {
            Ok(items) => {
                let entries = items
                    .iter()
                    .map(|item| {
                        let mut entry = item.entry.clone();
                        if let Some(name) = item.original.file_name() {
                            entry.name = name.to_string_lossy().to_string();
                        }
                        entry
                    })
                    .collect();
                *self.trash.borrow_mut() = items;
                self.display(entries, None);
            }
            Err(err) => {
                self.set_error(&format!("Cannot read Trash: {err}"));
                self.display(Vec::new(), None);
            }
        }
    }

    fn display(self: &Rc<Self>, entries: Vec<Entry>, note: Option<String>) {
        self.list_store.remove_all();
        self.grid_store.remove_all();
        for entry in &entries {
            let object = glib::BoxedAnyObject::new(entry.clone());
            self.list_store.append(&object);
            self.grid_store.append(&object);
        }
        *self.entries.borrow_mut() = entries;
        self.list_selection.unselect_all();
        self.grid_selection.unselect_all();
        self.update_breadcrumbs();
        self.update_nav_buttons();
        self.update_status(note);
        self.update_title();
    }

    fn refresh(self: &Rc<Self>) {
        if self.search_entry.text().trim().is_empty() {
            self.load();
        } else {
            self.on_search_changed();
        }
    }

    fn navigate(self: &Rc<Self>, path: PathBuf) {
        if !path.is_dir() {
            self.set_error(&format!("Not a folder: {}", path.display()));
            return;
        }
        let path = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf());
        if path == aspire_fs::trash_root() {
            self.open_trash();
            return;
        }
        self.mode.set(Mode::Files);
        {
            let mut tabs = self.tabs.borrow_mut();
            let index = self.active.get();
            if let Some(tab) = tabs.get_mut(index) {
                if tab.path != path {
                    tab.history.truncate(tab.index + 1);
                    tab.history.push(path.clone());
                    tab.index = tab.history.len() - 1;
                }
                tab.path = path.clone();
                tab.mode = Mode::Files;
                tab.title = title_for(&path);
            }
        }
        *self.current.borrow_mut() = path;
        self.reset_search_widgets();
        self.rebuild_tabs();
        self.load();
    }

    fn open_trash(self: &Rc<Self>) {
        let trash = aspire_fs::trash_root();
        self.mode.set(Mode::Trash);
        {
            let mut tabs = self.tabs.borrow_mut();
            let index = self.active.get();
            if let Some(tab) = tabs.get_mut(index) {
                if tab.mode != Mode::Trash {
                    tab.history.truncate(tab.index + 1);
                    tab.history.push(trash.clone());
                    tab.index = tab.history.len() - 1;
                }
                tab.mode = Mode::Trash;
                tab.path = trash.clone();
                tab.title = "Trash".into();
            }
        }
        *self.current.borrow_mut() = trash;
        self.reset_search_widgets();
        self.rebuild_tabs();
        self.load();
    }

    fn navigate_from_entry(self: &Rc<Self>, text: &str) {
        let expanded = aspire_config::expand_tilde(text.trim());
        if expanded.is_dir() {
            self.hide_location();
            self.navigate(expanded);
        } else {
            self.set_error(&format!("Not a folder: {}", expanded.display()));
        }
    }

    fn go_back(self: &Rc<Self>) {
        let target = {
            let mut tabs = self.tabs.borrow_mut();
            let index = self.active.get();
            let Some(tab) = tabs.get_mut(index) else { return };
            if tab.index == 0 {
                return;
            }
            tab.index -= 1;
            let path = tab.history[tab.index].clone();
            let mode = mode_for_path(&path);
            tab.path = path.clone();
            tab.mode = mode;
            tab.title = if mode == Mode::Trash {
                "Trash".into()
            } else {
                title_for(&path)
            };
            Some((path, mode))
        };
        if let Some((path, mode)) = target {
            self.mode.set(mode);
            *self.current.borrow_mut() = path;
            self.rebuild_tabs();
            self.load();
        }
    }

    fn go_forward(self: &Rc<Self>) {
        let target = {
            let mut tabs = self.tabs.borrow_mut();
            let index = self.active.get();
            let Some(tab) = tabs.get_mut(index) else { return };
            if tab.index + 1 >= tab.history.len() {
                return;
            }
            tab.index += 1;
            let path = tab.history[tab.index].clone();
            let mode = mode_for_path(&path);
            tab.path = path.clone();
            tab.mode = mode;
            tab.title = if mode == Mode::Trash {
                "Trash".into()
            } else {
                title_for(&path)
            };
            Some((path, mode))
        };
        if let Some((path, mode)) = target {
            self.mode.set(mode);
            *self.current.borrow_mut() = path;
            self.rebuild_tabs();
            self.load();
        }
    }

    fn go_up(self: &Rc<Self>) {
        if self.mode.get() == Mode::Trash {
            self.navigate(aspire_fs::home_dir());
            return;
        }
        let path = self.current.borrow().clone();
        if let Some(parent) = path.parent() {
            self.navigate(parent.to_path_buf());
        }
    }

    fn update_nav_buttons(&self) {
        let (can_back, can_forward) = {
            let tabs = self.tabs.borrow();
            tabs.get(self.active.get())
                .map(|tab| (tab.index > 0, tab.index + 1 < tab.history.len()))
                .unwrap_or((false, false))
        };
        self.back_button.set_sensitive(can_back);
        self.forward_button.set_sensitive(can_forward);
        let can_up = self.mode.get() == Mode::Trash
            || self.current.borrow().parent().is_some();
        self.up_button.set_sensitive(can_up);
    }

    fn update_title(&self) {
        let title = if self.mode.get() == Mode::Trash {
            "Trash".to_string()
        } else {
            title_for(&self.current.borrow())
        };
        self.window.set_title(Some(&format!("{title} — Files")));
    }

    fn update_breadcrumbs(self: &Rc<Self>) {
        while let Some(child) = self.path_bar.first_child() {
            self.path_bar.remove(&child);
        }
        if self.mode.get() == Mode::Trash {
            let label = gtk::Label::new(Some("Trash"));
            label.add_css_class("path-button");
            self.path_bar.append(&label);
            return;
        }

        let path = self.current.borrow().clone();
        let home = aspire_fs::home_dir();
        let push_button = |label: &str, target: PathBuf| {
            let button = gtk::Button::with_label(label);
            button.add_css_class("path-button");
            button.set_has_frame(false);
            button.connect_clicked({
                let app = self.clone();
                move |_| app.navigate(target.clone())
            });
            self.path_bar.append(&button);
        };
        let push_separator = || {
            let label = gtk::Label::new(Some("›"));
            label.add_css_class("dim-label");
            self.path_bar.append(&label);
        };

        if path.starts_with(&home) {
            push_button("Home", home.clone());
            let mut accumulated = home.clone();
            if let Ok(relative) = path.strip_prefix(&home) {
                for component in relative.components() {
                    accumulated.push(component);
                    push_separator();
                    push_button(&component.as_os_str().to_string_lossy(), accumulated.clone());
                }
            }
        } else {
            push_button("/", PathBuf::from("/"));
            let mut accumulated = PathBuf::from("/");
            for component in path.components() {
                if component == std::path::Component::RootDir {
                    continue;
                }
                accumulated.push(component);
                push_separator();
                push_button(&component.as_os_str().to_string_lossy(), accumulated.clone());
            }
        }
    }

    // ---------------------------------------------------------------------
    // Pestañas
    // ---------------------------------------------------------------------

    fn rebuild_tabs(self: &Rc<Self>) {
        while let Some(child) = self.tab_strip.first_child() {
            self.tab_strip.remove(&child);
        }
        let active = self.active.get();
        let tabs = self.tabs.borrow().clone();
        for (index, tab) in tabs.iter().enumerate() {
            let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            container.add_css_class("tab");
            if index == active {
                container.add_css_class("tab-active");
            }
            let button = gtk::Button::with_label(&tab.title);
            button.set_has_frame(false);
            button.connect_clicked({
                let app = self.clone();
                move |_| app.switch_tab(index)
            });
            let close = gtk::Button::with_label("×");
            close.add_css_class("tab-close");
            close.set_has_frame(false);
            close.set_tooltip_text(Some("Close tab"));
            close.connect_clicked({
                let app = self.clone();
                move |_| app.close_tab(index)
            });
            container.append(&button);
            container.append(&close);
            self.tab_strip.append(&container);
        }
        let new_tab = gtk::Button::from_icon_name("list-add-symbolic");
        new_tab.set_has_frame(false);
        new_tab.set_tooltip_text(Some("New tab (Ctrl+T)"));
        new_tab.connect_clicked({
            let app = self.clone();
            move |_| app.new_tab()
        });
        self.tab_strip.append(&new_tab);
    }

    fn new_tab(self: &Rc<Self>) {
        let path = if self.mode.get() == Mode::Trash {
            aspire_fs::home_dir()
        } else {
            self.current.borrow().clone()
        };
        self.tabs.borrow_mut().push(Tab::new(path));
        let index = self.tabs.borrow().len() - 1;
        self.switch_tab(index);
    }

    fn switch_tab(self: &Rc<Self>, index: usize) {
        if index >= self.tabs.borrow().len() {
            return;
        }
        self.active.set(index);
        let (path, mode) = {
            let tabs = self.tabs.borrow();
            let tab = &tabs[index];
            (tab.path.clone(), tab.mode)
        };
        self.mode.set(mode);
        *self.current.borrow_mut() = path;
        self.reset_search_widgets();
        self.rebuild_tabs();
        self.load();
    }

    fn close_tab(self: &Rc<Self>, index: usize) {
        let current_active = self.active.get();
        {
            let mut tabs = self.tabs.borrow_mut();
            if index >= tabs.len() {
                return;
            }
            tabs.remove(index);
            if tabs.is_empty() {
                tabs.push(Tab::new(aspire_fs::home_dir()));
            }
        }
        let new_index = if index < current_active {
            current_active - 1
        } else {
            current_active
        }
        .min(self.tabs.borrow().len().saturating_sub(1));
        self.active.set(new_index);
        self.switch_tab(new_index);
    }

    // ---------------------------------------------------------------------
    // Búsqueda
    // ---------------------------------------------------------------------

    fn toggle_search(self: &Rc<Self>, global: bool) {
        let visible = self.search_revealer.reveals_child();
        if visible && !global {
            self.close_search();
            return;
        }
        self.search_revealer.set_reveal_child(true);
        if global {
            self.search_global.set_active(true);
        }
        self.search_entry.grab_focus();
    }

    fn close_search(self: &Rc<Self>) {
        self.search_generation.set(self.search_generation.get() + 1);
        self.search_rx.replace(None);
        self.suppress_search.set(true);
        self.search_entry.set_text("");
        self.suppress_search.set(false);
        self.search_revealer.set_reveal_child(false);
        self.load();
    }

    fn reset_search_widgets(&self) {
        self.search_generation.set(self.search_generation.get() + 1);
        self.search_rx.replace(None);
        self.suppress_search.set(true);
        self.search_entry.set_text("");
        self.suppress_search.set(false);
        self.search_revealer.set_reveal_child(false);
    }

    fn on_search_changed(self: &Rc<Self>) {
        if self.suppress_search.get() {
            return;
        }
        let query = self.search_entry.text().to_string();
        let generation = self.search_generation.get() + 1;
        self.search_generation.set(generation);
        if query.trim().is_empty() {
            self.load();
            return;
        }
        let app = self.clone();
        glib::timeout_add_local_once(Duration::from_millis(250), move || {
            if app.search_generation.get() == generation {
                app.start_search(query, generation);
            }
        });
    }

    fn start_search(self: &Rc<Self>, query: String, generation: u64) {
        let root = self.current.borrow().clone();
        let global = self.search_global.is_active();
        let show_hidden = self.config.borrow().view.show_hidden;
        let (sender, receiver) = channel();
        self.search_rx.replace(Some(receiver));
        let thread_query = query.clone();
        std::thread::spawn(move || {
            let result = aspire_fs::search(&root, &thread_query, global, show_hidden)
                .map_err(|err| err.to_string());
            let _ = sender.send((generation, result));
        });

        let app = self.clone();
        glib::timeout_add_local(Duration::from_millis(60), move || {
            if app.search_generation.get() != generation {
                return glib::ControlFlow::Break;
            }
            let received = {
                let receiver = app.search_rx.borrow();
                match receiver.as_ref() {
                    Some(receiver) => match receiver.try_recv() {
                        Ok(value) => Some(value),
                        Err(TryRecvError::Empty) => None,
                        Err(TryRecvError::Disconnected) => {
                            Some((generation, Err("search cancelled".into())))
                        }
                    },
                    None => return glib::ControlFlow::Break,
                }
            };
            let Some((result_generation, result)) = received else {
                return glib::ControlFlow::Continue;
            };
            app.search_rx.replace(None);
            if result_generation == app.search_generation.get() {
                match result {
                    Ok(entries) => {
                        let count = entries.len();
                        app.display(
                            entries,
                            Some(format!("{count} results for \"{query}\"")),
                        );
                    }
                    Err(err) => app.set_error(&err),
                }
            }
            glib::ControlFlow::Break
        });
    }

    // ---------------------------------------------------------------------
    // Selección y estado
    // ---------------------------------------------------------------------

    fn active_selection(&self) -> &gtk::MultiSelection {
        if self.stack.visible_child_name().as_deref() == Some("list") {
            &self.list_selection
        } else {
            &self.grid_selection
        }
    }

    fn selected_entries(&self) -> Vec<Entry> {
        let selection = self.active_selection();
        let bitset = selection.selection();
        let mut selected = Vec::new();
        for position in 0..selection.n_items() {
            if !bitset.contains(position) {
                continue;
            }
            let Some(object) = selection.item(position) else {
                continue;
            };
            let Ok(object) = object.downcast::<glib::BoxedAnyObject>() else {
                continue;
            };
            if let Ok(entry) = object.try_borrow::<Entry>() {
                selected.push(entry.clone());
            };
        }
        selected
    }

    fn update_status(&self, note: Option<String>) {
        let count = self.entries.borrow().len();
        let selected = self.selected_entries().len();
        let mut text = note.unwrap_or_else(|| {
            if count == 1 {
                "1 item".to_string()
            } else {
                format!("{count} items")
            }
        });
        if selected > 0 {
            text.push_str(&format!(" · {selected} selected"));
        }
        if self.mode.get() == Mode::Trash {
            text = format!("Trash · {text}");
        }
        self.status.set_text(&text);
    }

    fn set_error(&self, message: &str) {
        eprintln!("aspire-files: {message}");
        self.status.set_text(message);
    }

    fn select_path(&self, path: &Path) {
        let index = self
            .entries
            .borrow()
            .iter()
            .position(|entry| entry.path == path);
        let Some(index) = index else { return };
        self.list_selection.select_item(index as u32, true);
        self.grid_selection.select_item(index as u32, true);
    }

    fn select_all(&self) {
        self.active_selection().select_all();
    }

    // ---------------------------------------------------------------------
    // Operaciones
    // ---------------------------------------------------------------------

    fn open_selected(self: &Rc<Self>) {
        if self.mode.get() == Mode::Trash {
            self.set_error("Restore an item from Trash before opening it");
            return;
        }
        let Some(entry) = self.selected_entries().into_iter().next() else {
            return;
        };
        if entry.is_dir {
            self.navigate(entry.path);
        } else {
            self.launch(&entry.path);
        }
    }

    fn launch(self: &Rc<Self>, path: &Path) {
        if aspire_fs::is_appimage(path) {
            let _ = aspire_fs::make_executable(path);
            if let Err(err) = std::process::Command::new(path).spawn() {
                self.set_error(&format!("Cannot run {}: {err}", path.display()));
            }
            return;
        }
        let file = gio::File::for_path(path);
        if let Err(err) =
            gio::AppInfo::launch_default_for_uri(&file.uri(), None::<&gio::AppLaunchContext>)
        {
            self.set_error(&format!("No application for {}: {err}", path.display()));
        }
    }

    fn new_folder(self: &Rc<Self>) {
        if self.mode.get() == Mode::Trash {
            self.set_error("Cannot create folders in Trash");
            return;
        }
        let parent = self.current.borrow().clone();
        let app = self.clone();
        dialogs::text_prompt(
            self.window.upcast_ref(),
            "New Folder",
            "Folder name",
            "New Folder",
            "Create",
            move |name| match aspire_fs::create_dir(&parent, &name) {
                Ok(path) => {
                    app.load();
                    app.select_path(&path);
                }
                Err(err) => app.set_error(&format!("Cannot create folder: {err}")),
            },
        );
    }

    fn rename_selected(self: &Rc<Self>) {
        let selected = self.selected_entries();
        if selected.len() != 1 {
            self.set_error("Select one item to rename");
            return;
        }
        let entry = selected[0].clone();
        let app = self.clone();
        dialogs::text_prompt(
            self.window.upcast_ref(),
            "Rename",
            "New name",
            &entry.name,
            "Rename",
            move |name| match aspire_fs::rename_entry(&entry.path, &name) {
                Ok(path) => {
                    app.load();
                    app.select_path(&path);
                }
                Err(err) => app.set_error(&format!("Cannot rename: {err}")),
            },
        );
    }

    fn copy_selected(self: &Rc<Self>, cut: bool) {
        let paths: Vec<PathBuf> = self
            .selected_entries()
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        if paths.is_empty() {
            return;
        }
        *self.clipboard.borrow_mut() = Some(Clipboard { paths, cut });
        self.status.set_text(if cut {
            "Cut to clipboard"
        } else {
            "Copied to clipboard"
        });
    }

    fn paste(self: &Rc<Self>) {
        if self.mode.get() == Mode::Trash {
            self.set_error("Cannot paste in Trash");
            return;
        }
        let Some(clipboard) = self.clipboard.borrow().clone() else {
            return;
        };
        let target = self.current.borrow().clone();
        match aspire_fs::transfer(&clipboard.paths, &target, clipboard.cut) {
            Ok(_) => {
                if clipboard.cut {
                    self.clipboard.replace(None);
                }
                self.load();
            }
            Err(err) => self.set_error(&format!("Cannot paste: {err}")),
        }
    }

    fn trash_selected(self: &Rc<Self>) {
        if self.mode.get() == Mode::Trash {
            self.delete_selected();
            return;
        }
        let selected = self.selected_entries();
        if selected.is_empty() {
            return;
        }
        let mut errors = Vec::new();
        for entry in selected {
            if let Err(err) = aspire_fs::trash_path(&entry.path) {
                errors.push(format!("{}: {err}", entry.name));
            }
        }
        if !errors.is_empty() {
            self.set_error(&format!("Some items were not trashed: {}", errors.join("; ")));
        }
        self.load();
    }

    fn delete_selected(self: &Rc<Self>) {
        let selected = self.selected_entries();
        if selected.is_empty() {
            return;
        }
        let app = self.clone();
        let detail = if selected.len() == 1 {
            selected[0].name.clone()
        } else {
            format!("{} items", selected.len())
        };
        dialogs::confirm(
            self.window.upcast_ref(),
            "Delete permanently?",
            &format!("{detail} will be deleted permanently. This cannot be undone."),
            "Delete",
            move || {
                let mut errors = Vec::new();
                for entry in &selected {
                    if let Err(err) = aspire_fs::remove_path(&entry.path) {
                        errors.push(format!("{}: {err}", entry.name));
                    }
                }
                if !errors.is_empty() {
                    app.set_error(&format!("Some items were not deleted: {}", errors.join("; ")));
                }
                app.load();
            },
        );
    }

    fn restore_selected(self: &Rc<Self>) {
        if self.mode.get() != Mode::Trash {
            self.set_error("Restore is only available in Trash");
            return;
        }
        let selected = self.selected_entries();
        if selected.is_empty() {
            return;
        }
        let items: Vec<TrashEntry> = {
            let trash = self.trash.borrow();
            selected
                .iter()
                .filter_map(|entry| {
                    trash
                        .iter()
                        .find(|item| item.entry.path == entry.path)
                        .cloned()
                })
                .collect()
        };
        let mut errors = Vec::new();
        for item in &items {
            if let Err(err) = aspire_fs::restore_trash(item) {
                errors.push(format!("{}: {err}", item.entry.name));
            }
        }
        if !errors.is_empty() {
            self.set_error(&format!("Some items were not restored: {}", errors.join("; ")));
        }
        self.load();
    }

    fn empty_trash(self: &Rc<Self>) {
        if self.mode.get() != Mode::Trash {
            self.set_error("Empty Trash is only available in Trash");
            return;
        }
        let app = self.clone();
        dialogs::confirm(
            self.window.upcast_ref(),
            "Empty Trash?",
            "All items in Trash will be deleted permanently.",
            "Empty",
            move || match aspire_fs::empty_trash() {
                Ok(()) => app.load(),
                Err(err) => app.set_error(&format!("Cannot empty Trash: {err}")),
            },
        );
    }

    fn show_properties(self: &Rc<Self>) {
        let selected = self.selected_entries();
        if selected.len() != 1 {
            self.set_error("Select one item to see its properties");
            return;
        }
        dialogs::properties(self.window.upcast_ref(), &selected[0]);
    }

    fn open_with(self: &Rc<Self>) {
        let paths: Vec<PathBuf> = self
            .selected_entries()
            .into_iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.path)
            .collect();
        if paths.is_empty() {
            self.set_error("Select at least one file to open with another application");
            return;
        }
        let app = self.clone();
        let launch_paths = paths.clone();
        dialogs::open_with(self.window.upcast_ref(), &paths, move |info, set_default, content_type| {
            if set_default {
                if let Err(err) = info.set_as_default_for_type(content_type) {
                    app.set_error(&format!("Cannot set default application: {err}"));
                }
            }
            let _ = info.set_as_last_used_for_type(content_type);
            let files: Vec<gio::File> = launch_paths
                .iter()
                .map(gio::File::for_path)
                .collect();
            if let Err(err) = info.launch(&files, None::<&gio::AppLaunchContext>) {
                app.set_error(&format!("Cannot open with application: {err}"));
            }
        });
    }

    fn make_executable(self: &Rc<Self>) {
        let selected = self.selected_entries();
        let mut errors = Vec::new();
        for entry in selected {
            if entry.is_dir {
                continue;
            }
            if let Err(err) = aspire_fs::make_executable(&entry.path) {
                errors.push(format!("{}: {err}", entry.name));
            }
        }
        if !errors.is_empty() {
            self.set_error(&format!("Cannot make executable: {}", errors.join("; ")));
        }
        self.load();
    }

    // ---------------------------------------------------------------------
    // Marcadores
    // ---------------------------------------------------------------------

    fn add_bookmark(self: &Rc<Self>) {
        if self.mode.get() == Mode::Trash {
            self.set_error("Cannot bookmark Trash");
            return;
        }
        let mut targets: Vec<PathBuf> = self
            .selected_entries()
            .into_iter()
            .filter(|entry| entry.is_dir)
            .map(|entry| entry.path)
            .collect();
        if targets.is_empty() {
            targets.push(self.current.borrow().clone());
        }

        let mut added = 0;
        {
            let mut bookmarks = self.bookmarks.borrow_mut();
            for path in targets {
                if bookmarks.iter().any(|bookmark| bookmark.path == path) {
                    continue;
                }
                bookmarks.push(Bookmark {
                    label: title_for(&path),
                    path,
                });
                added += 1;
            }
            if added == 0 {
                self.status.set_text("Folder is already bookmarked");
                return;
            }
            if let Err(err) = aspire_fs::write_bookmarks(&bookmarks) {
                self.set_error(&format!("Cannot save bookmarks: {err}"));
            }
        }
        self.status
            .set_text(if added == 1 { "Bookmark added" } else { "Bookmarks added" });
        self.rebuild_sidebar();
    }

    fn remove_bookmark(self: &Rc<Self>) {
        let Some(index) = self.selected_bookmark.get() else {
            self.set_error("Select a bookmark in the sidebar first");
            return;
        };
        let Some(bookmark) = self.bookmarks.borrow().get(index).cloned() else {
            return;
        };
        let app = self.clone();
        dialogs::confirm(
            self.window.upcast_ref(),
            "Remove bookmark?",
            &format!("“{}” will be removed from the sidebar.", bookmark.label),
            "Remove",
            move || {
                {
                    let mut bookmarks = app.bookmarks.borrow_mut();
                    let Some(position) = bookmarks
                        .iter()
                        .position(|candidate| candidate.path == bookmark.path)
                    else {
                        return;
                    };
                    bookmarks.remove(position);
                    if let Err(err) = aspire_fs::write_bookmarks(&bookmarks) {
                        app.set_error(&format!("Cannot save bookmarks: {err}"));
                    }
                }
                app.selected_bookmark.set(None);
                app.rebuild_sidebar();
            },
        );
    }

    fn move_bookmark(self: &Rc<Self>, delta: i32) {
        let Some(index) = self.selected_bookmark.get() else {
            self.set_error("Select a bookmark in the sidebar first");
            return;
        };
        let target = index as i32 + delta;
        {
            let mut bookmarks = self.bookmarks.borrow_mut();
            if target < 0 || target as usize >= bookmarks.len() {
                return;
            }
            bookmarks.swap(index, target as usize);
            if let Err(err) = aspire_fs::write_bookmarks(&bookmarks) {
                self.set_error(&format!("Cannot save bookmarks: {err}"));
            }
        }
        self.selected_bookmark.set(Some(target as usize));
        self.rebuild_sidebar();
    }

    fn rebuild_sidebar(self: &Rc<Self>) {
        while let Some(child) = self.sidebar.first_child() {
            self.sidebar.remove(&child);
        }
        let mut targets = Vec::new();

        let section = gtk::ListBoxRow::new();
        section.set_selectable(false);
        section.set_activatable(false);
        section.set_focusable(false);
        let label = gtk::Label::new(Some("Places"));
        label.set_xalign(0.0);
        label.add_css_class("sidebar-section");
        section.set_child(Some(&label));
        self.sidebar.append(&section);
        targets.push(SidebarTarget::Header);

        for place in aspire_fs::places() {
            let row = gtk::ListBoxRow::new();
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            content.set_margin_top(2);
            content.set_margin_bottom(2);
            let icon = gtk::Image::from_icon_name(place.icon);
            icon.set_pixel_size(16);
            let label = gtk::Label::new(Some(&place.name));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            content.append(&icon);
            content.append(&label);
            row.set_child(Some(&content));
            self.sidebar.append(&row);
            targets.push(if place.is_trash {
                SidebarTarget::Trash
            } else {
                SidebarTarget::Place(place.path)
            });
        }

        let section = gtk::ListBoxRow::new();
        section.set_selectable(false);
        section.set_activatable(false);
        section.set_focusable(false);
        let label = gtk::Label::new(Some("Bookmarks"));
        label.set_xalign(0.0);
        label.add_css_class("sidebar-section");
        section.set_child(Some(&label));
        self.sidebar.append(&section);
        targets.push(SidebarTarget::Header);

        let bookmarks = self.bookmarks.borrow().clone();
        for (index, bookmark) in bookmarks.iter().enumerate() {
            let row = gtk::ListBoxRow::new();
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            content.set_margin_top(2);
            content.set_margin_bottom(2);
            let icon = gtk::Image::from_icon_name("bookmark-new-symbolic");
            icon.set_pixel_size(16);
            let label = gtk::Label::new(Some(&bookmark.label));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            label.set_ellipsize(pango::EllipsizeMode::Middle);
            content.append(&icon);
            content.append(&label);
            row.set_child(Some(&content));
            row.set_tooltip_text(Some(&bookmark.path.display().to_string()));
            self.sidebar.append(&row);
            targets.push(SidebarTarget::Bookmark(index));
        }

        *self.sidebar_targets.borrow_mut() = targets;
    }

    fn activate_sidebar_row(self: &Rc<Self>, index: i32) {
        let target = self
            .sidebar_targets
            .borrow()
            .get(index as usize)
            .cloned();
        match target {
            Some(SidebarTarget::Place(path)) => self.navigate(path),
            Some(SidebarTarget::Trash) => self.open_trash(),
            Some(SidebarTarget::Bookmark(index)) => {
                if let Some(bookmark) = self.bookmarks.borrow().get(index).cloned() {
                    self.navigate(bookmark.path);
                }
            }
            _ => {}
        }
    }

    // ---------------------------------------------------------------------
    // Vista, tema y preferencias
    // ---------------------------------------------------------------------

    fn apply_view_mode(&self) {
        let mode = self.config.borrow().view.view_mode.clone();
        if mode == "list" {
            self.stack.set_visible_child_name("list");
            self.view_button.set_icon_name("view-list-symbolic");
        } else {
            self.stack.set_visible_child_name("grid");
            self.view_button.set_icon_name("view-grid-symbolic");
        }
    }

    fn set_view_mode(self: &Rc<Self>, mode: &str) {
        {
            let mut config = self.config.borrow_mut();
            config.view.view_mode = mode.to_string();
            let _ = config.save();
        }
        self.apply_view_mode();
        self.update_status(None);
    }

    fn toggle_view(self: &Rc<Self>) {
        let next = if self.config.borrow().view.view_mode == "list" {
            "grid"
        } else {
            "list"
        };
        self.set_view_mode(next);
    }

    fn toggle_hidden(self: &Rc<Self>) {
        {
            let mut config = self.config.borrow_mut();
            config.view.show_hidden = !config.view.show_hidden;
            let _ = config.save();
        }
        self.load();
    }

    fn apply_sidebar(&self) {
        self.sidebar_revealer
            .set_reveal_child(self.config.borrow().window.sidebar_visible);
    }

    fn toggle_sidebar(self: &Rc<Self>) {
        {
            let mut config = self.config.borrow_mut();
            config.window.sidebar_visible = !config.window.sidebar_visible;
            let _ = config.save();
        }
        self.apply_sidebar();
    }

    fn preferences(self: &Rc<Self>) {
        let config = self.config.borrow().clone();
        let app = self.clone();
        dialogs::preferences(self.window.upcast_ref(), &config, move |updated| {
            *app.config.borrow_mut() = updated.clone();
            if let Err(err) = updated.save() {
                app.set_error(&format!("Cannot save preferences: {err}"));
            }
            app.apply_view_mode();
            app.apply_sidebar();
            app.date_detailed
                .set(updated.view.date_format == "detailed");
            let single_click = updated.general.open_action == "single";
            app.list_view.set_single_click_activate(single_click);
            app.grid_view.set_single_click_activate(single_click);
            app.search_global
                .set_active(updated.view.search_subfolders);
            app.reload_theme();
            app.watch_theme();
            app.load();
        });
    }

    fn show_location(self: &Rc<Self>) {
        let path = if self.mode.get() == Mode::Trash {
            aspire_fs::home_dir()
        } else {
            self.current.borrow().clone()
        };
        self.location.set_text(&path.display().to_string());
        self.center_stack.set_visible_child_name("location");
        self.location.grab_focus();
        self.location.select_region(0, -1);
    }

    fn hide_location(&self) {
        self.center_stack.set_visible_child_name("path");
    }

    fn reload_theme(self: &Rc<Self>) {
        let engine = self.config.borrow().resolved_engine();
        let path = self.config.borrow().colors_path();
        if let Some(display) = gdk::Display::default() {
            theme::apply(&display, &self.theme_provider, &path, engine);
        }
    }

    fn watch_theme(self: &Rc<Self>) {
        let path = self.config.borrow().colors_path();
        let mut monitors = Vec::new();
        if let Some(parent) = path.parent() {
            if let Ok(monitor) = gio::File::for_path(parent)
                .monitor_directory(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>)
            {
                let app = self.clone();
                monitor.connect_changed(move |_, _, _, _| {
                    let app = app.clone();
                    glib::timeout_add_local_once(Duration::from_millis(200), move || {
                        app.reload_theme()
                    });
                });
                monitors.push(monitor);
            }
        }
        if path.exists() {
            if let Ok(monitor) = gio::File::for_path(&path)
                .monitor_file(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>)
            {
                let app = self.clone();
                monitor.connect_changed(move |_, _, _, _| {
                    let app = app.clone();
                    glib::timeout_add_local_once(Duration::from_millis(200), move || {
                        app.reload_theme()
                    });
                });
                monitors.push(monitor);
            }
        }
        *self.theme_monitors.borrow_mut() = monitors;
    }

    // ---------------------------------------------------------------------
    // Utilidades expuestas a las vistas
    // ---------------------------------------------------------------------

    pub fn show_thumbnails(&self) -> bool {
        self.config.borrow().view.show_thumbnails
    }

    pub fn thumbnail_path(&self, entry: &Entry) -> Option<PathBuf> {
        if entry.is_dir {
            return None;
        }
        let extension = entry.extension();
        let supported = matches!(
            extension.as_str(),
            "png"
                | "jpg"
                | "jpeg"
                | "jpe"
                | "webp"
                | "bmp"
                | "gif"
                | "tif"
                | "tiff"
                | "svg"
                | "ico"
                | "avif"
                | "pdf"
                | "mp4"
                | "m4v"
                | "webm"
                | "mkv"
                | "mov"
                | "avi"
                | "ogv"
                | "mpg"
                | "mpeg"
                | "flv"
                | "wmv"
        );
        if !supported {
            return None;
        }
        if let Some(cached) = self.thumb_memory.borrow().get(&entry.path) {
            return cached.clone();
        }
        let size = (self.icon_size.get().max(64) * 2).min(512);
        let generated = aspire_thumb::thumbnail(&entry.path, size);
        self.thumb_memory
            .borrow_mut()
            .insert(entry.path.clone(), generated.clone());
        generated
    }

    fn new_window(self: &Rc<Self>) {
        let path = if self.mode.get() == Mode::Trash {
            aspire_fs::home_dir()
        } else {
            self.current.borrow().clone()
        };
        if let Some(application) = self.window.application() {
            let _window = crate::app::build(&application, Some(path));
        }
    }

    fn _unused_engine(&self) -> Engine {
        self.config.borrow().resolved_engine()
    }
}
