//! Diálogos de aspire-files: entrada de texto, confirmación, propiedades y
//! preferencias. GTK4 puro (sin libadwaita).

use std::path::PathBuf;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gio, glib};

use aspire_config::{Config, Engine};
use aspire_fs::Entry;

fn dialog_shell(
    parent: &gtk::Window,
    title: &str,
    accept_label: &str,
) -> (gtk::Window, gtk::HeaderBar, gtk::Button, gtk::Button) {
    let window = gtk::Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .resizable(false)
        .build();
    let header = gtk::HeaderBar::new();
    let cancel = gtk::Button::with_label("Cancel");
    let accept = gtk::Button::with_label(accept_label);
    accept.add_css_class("suggested-action");
    header.pack_start(&cancel);
    header.pack_end(&accept);
    window.set_titlebar(Some(&header));
    (window, header, cancel, accept)
}

/// Pide un texto y lo entrega al aceptar (Enter o botón).
pub fn text_prompt<F>(
    parent: &gtk::Window,
    title: &str,
    label: &str,
    initial: &str,
    accept_label: &str,
    on_accept: F,
) where
    F: Fn(String) + 'static,
{
    let (window, _header, cancel, accept) = dialog_shell(parent, title, accept_label);
    window.set_default_width(420);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    let caption = gtk::Label::new(Some(label));
    caption.set_xalign(0.0);
    let entry = gtk::Entry::new();
    entry.set_text(initial);
    entry.select_region(0, -1);
    entry.set_activates_default(true);
    content.append(&caption);
    content.append(&entry);
    window.set_child(Some(&content));

    let on_accept = Rc::new(on_accept);
    let submit = {
        let window = window.clone();
        let entry = entry.clone();
        let on_accept = on_accept.clone();
        move || {
            let text = entry.text().to_string();
            on_accept(text);
            window.close();
        }
    };

    {
        let submit = submit.clone();
        accept.connect_clicked(move |_| submit());
    }
    {
        let submit = submit.clone();
        entry.connect_activate(move |_| submit());
    }
    cancel.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });

    window.present();
    entry.grab_focus();
}

/// Confirmación destructiva con `GtkAlertDialog`.
pub fn confirm<F>(
    parent: &gtk::Window,
    heading: &str,
    detail: &str,
    accept_label: &str,
    on_accept: F,
) where
    F: FnOnce() + 'static,
{
    let dialog = gtk::AlertDialog::builder()
        .message(heading)
        .detail(detail)
        .modal(true)
        .buttons(["Cancel", accept_label])
        .cancel_button(0)
        .default_button(0)
        .build();
    let on_accept = Rc::new(std::cell::RefCell::new(Some(on_accept)));
    dialog.choose(Some(parent), None::<&gio::Cancellable>, move |result| {
        if let Ok(index) = result {
            if index == 1 {
                if let Some(callback) = on_accept.borrow_mut().take() {
                    callback();
                }
            }
        }
    });
}

pub fn properties(parent: &gtk::Window, entry: &Entry) {
    let (window, _header, cancel, accept) = dialog_shell(parent, "Properties", "Close");
    cancel.set_visible(false);
    accept.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });
    window.set_default_width(520);

    let grid = gtk::Grid::builder()
        .row_spacing(6)
        .column_spacing(16)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();

    let modified = entry
        .modified
        .and_then(|time| {
            let duration = time.duration_since(std::time::UNIX_EPOCH).ok()?;
            glib::DateTime::from_unix_local(duration.as_secs() as i64).ok()
        })
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").map(|s| s.to_string()).unwrap_or_default())
        .unwrap_or_else(|| "—".into());

    let rows = [
        ("Name", entry.name.clone()),
        ("Path", entry.path.display().to_string()),
        (
            "Type",
            if entry.is_dir {
                "Folder".into()
            } else {
                format!("{} file", entry.extension().to_uppercase())
            },
        ),
        (
            "Size",
            if entry.is_dir {
                "—".into()
            } else {
                aspire_fs::format_size(entry.size)
            },
        ),
        ("Modified", modified),
        (
            "Symlink",
            if entry.is_symlink { "Yes" } else { "No" }.into(),
        ),
    ];

    for (row, (name, value)) in rows.iter().enumerate() {
        let label = gtk::Label::new(Some(name));
        label.set_xalign(0.0);
        label.add_css_class("dim-label");
        let value = gtk::Label::new(Some(value));
        value.set_xalign(0.0);
        value.set_selectable(true);
        value.set_wrap(true);
        value.set_hexpand(true);
        grid.attach(&label, 0, row as i32, 1, 1);
        grid.attach(&value, 1, row as i32, 1, 1);
    }

    window.set_child(Some(&grid));
    window.present();
}

/// Preferencias persistentes en `config.toml`.
pub fn preferences<F>(parent: &gtk::Window, config: &Config, on_save: F)
where
    F: Fn(Config) + 'static,
{
    let (window, _header, cancel, accept) = dialog_shell(parent, "Preferences", "Save");
    window.set_default_width(520);

    let grid = gtk::Grid::builder()
        .row_spacing(8)
        .column_spacing(16)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();

    let view_dropdown = gtk::DropDown::from_strings(&["Grid", "List"]);
    view_dropdown.set_selected(if config.view.view_mode == "list" { 1 } else { 0 });

    let icon_size = gtk::SpinButton::with_range(48.0, 160.0, 8.0);
    icon_size.set_value(config.view.icon_size as f64);

    let show_hidden = gtk::Switch::new();
    show_hidden.set_active(config.view.show_hidden);

    let sidebar = gtk::Switch::new();
    sidebar.set_active(config.window.sidebar_visible);

    let sort_dropdown = gtk::DropDown::from_strings(&["Name", "Size", "Type", "Modified"]);
    sort_dropdown.set_selected(match config.view.sort_by.as_str() {
        "name" => 0,
        "size" => 1,
        "type" => 2,
        _ => 3,
    });

    let descending = gtk::Switch::new();
    descending.set_active(config.view.sort_descending);

    let folders_first = gtk::Switch::new();
    folders_first.set_active(config.general.folders_first);

    let thumbnails = gtk::Switch::new();
    thumbnails.set_active(config.view.show_thumbnails);

    let engine = gtk::DropDown::from_strings(&["Auto", "Matugen", "Tinty", "None"]);
    engine.set_selected(match config.theme.engine {
        Engine::Auto => 0,
        Engine::Matugen => 1,
        Engine::Tinty => 2,
        Engine::None => 3,
    });

    let colors_entry = gtk::Entry::new();
    colors_entry.set_text(&config.theme.colors_file);
    colors_entry.set_hexpand(true);

    let mut row = 0;
    {
        let mut add = |name: &str, widget: &gtk::Widget| {
            let label = gtk::Label::new(Some(name));
            label.set_xalign(0.0);
            grid.attach(&label, 0, row, 1, 1);
            grid.attach(widget, 1, row, 1, 1);
            row += 1;
        };
        add("Default view", view_dropdown.upcast_ref());
        add("Icon size", icon_size.upcast_ref());
        add("Show hidden files", show_hidden.upcast_ref());
        add("Show sidebar", sidebar.upcast_ref());
        add("Sort by", sort_dropdown.upcast_ref());
        add("Descending", descending.upcast_ref());
        add("Folders first", folders_first.upcast_ref());
        add("Thumbnails", thumbnails.upcast_ref());
        add("Theme engine", engine.upcast_ref());
        add("Colors file", colors_entry.upcast_ref());
    }

    let note = gtk::Label::new(Some(
        "Some theme changes are applied the next time the application starts.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    grid.attach(&note, 0, row, 2, 1);

    window.set_child(Some(&grid));

    let base = config.clone();
    accept.connect_clicked({
        let window = window.clone();
        let on_save = Rc::new(on_save);
        move |_| {
            let mut updated = base.clone();
            updated.view.view_mode = if view_dropdown.selected() == 1 {
                "list".into()
            } else {
                "grid".into()
            };
            updated.view.icon_size = icon_size.value() as i32;
            updated.view.show_hidden = show_hidden.is_active();
            updated.window.sidebar_visible = sidebar.is_active();
            updated.view.sort_by = match sort_dropdown.selected() {
                0 => "name",
                1 => "size",
                2 => "type",
                _ => "modified",
            }
            .into();
            updated.view.sort_descending = descending.is_active();
            updated.general.folders_first = folders_first.is_active();
            updated.view.show_thumbnails = thumbnails.is_active();
            updated.theme.engine = match engine.selected() {
                1 => Engine::Matugen,
                2 => Engine::Tinty,
                3 => Engine::None,
                _ => Engine::Auto,
            };
            updated.theme.colors_file = colors_entry.text().to_string();
            on_save(updated);
            window.close();
        }
    });
    cancel.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });

    window.present();
}

/// Diálogo "Abrir con…". Permite elegir una aplicación para uno o varios
/// archivos y, opcionalmente, guardarla como predeterminada para su tipo MIME.
#[allow(deprecated)]
pub fn open_with<F>(parent: &gtk::Window, paths: &[PathBuf], on_launch: F)
where
    F: Fn(&gio::AppInfo, bool, &str) + 'static,
{
    let Some(first) = paths.first() else {
        return;
    };
    let (content_type, _) = gio::content_type_guess(Some(first), None::<&[u8]>);
    let content_type = content_type.to_string();

    let (window, _header, cancel, accept) = dialog_shell(parent, "Open With", "Open");
    window.set_default_width(560);
    window.set_default_height(560);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let heading = if paths.len() == 1 {
        let name = first
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| first.display().to_string());
        format!("Choose an application to open “{name}”.")
    } else {
        format!("Choose an application to open {} files.", paths.len())
    };
    let label = gtk::Label::new(Some(&heading));
    label.set_xalign(0.0);
    label.set_wrap(true);
    content.append(&label);

    let chooser = gtk::AppChooserWidget::new(&content_type);
    chooser.set_show_all(true);
    chooser.set_show_recommended(true);
    chooser.set_show_fallback(true);
    chooser.set_show_other(false);
    chooser.set_default_text("No applications available for this file type");
    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .min_content_height(340)
        .build();
    scroller.set_child(Some(&chooser));
    content.append(&scroller);

    let set_default = gtk::CheckButton::with_label("Set as default for this file type");
    content.append(&set_default);
    window.set_child(Some(&content));

    accept.connect_clicked({
        let window = window.clone();
        let chooser = chooser.clone();
        let set_default = set_default.clone();
        let content_type = content_type.clone();
        let on_launch = Rc::new(on_launch);
        move |_| {
            if let Some(info) = chooser.app_info() {
                on_launch(&info, set_default.is_active(), &content_type);
                window.close();
            }
        }
    });
    cancel.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });

    window.present();
}
