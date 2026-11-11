use crate::apply;
use crate::catalog::{self, FieldKind, FieldSpec, ModuleSpec};
use crate::document::BarDocument;
use crate::model::BarConfig;
use crate::palette::{self, Palette};
use crate::preview::Preview;
use crate::scripts;
use crate::style::{self, ModuleOverrides, StateOverride, StyleSettings, COLOR_TOKENS};
use crate::templates;
use crate::themes::{self, ThemeManifest};
use anyhow::{bail, Context, Result};
use gtk::gdk::{ContentProvider, DragAction};
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, DropDown, Entry,
    FileDialog, Label, ListBox, ListBoxRow, MenuButton, Notebook, Orientation, PolicyType,
    ScrolledWindow, SpinButton, TextBuffer, TextView,
};
use serde_json::{json, Map, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
struct Selection {
    zone: usize,
    module: String,
}

struct EditorState {
    document: BarDocument,
    bar_index: usize,
    selection: Option<Selection>,
}

struct EditorUi {
    window: ApplicationWindow,
    state: RefCell<EditorState>,
    saved_document: RefCell<BarDocument>,
    bar_dropdown: DropDown,
    module_dropdown: DropDown,
    custom_name: Entry,
    search: Entry,
    catalog: gtk::FlowBox,
    active_modules: ListBox,
    zone_summary: Label,
    summary: Label,
    form: GtkBox,
    status: Label,
    error_button: MenuButton,
    error_label: Label,
    save_button: Button,
    style: StyleWidgets,
    themes: ThemeWidgets,
    templates: TemplateWidgets,
    scripts: ScriptWidgets,
    files: FilesWidgets,
    about: AboutWidgets,
    preview: Rc<Preview>,
    quick_theme_label: Label,
    quick_theme_menu: GtkBox,
    previous_theme_button: Button,
    previous_theme: RefCell<Option<String>>,
    selection_sync: Cell<bool>,
    pending_test: RefCell<Option<apply::PendingApply>>,
    pending_document: RefCell<Option<BarDocument>>,
    test_seconds: Cell<u8>,
    test_generation: Cell<u64>,
    monitors: RefCell<Vec<gtk::gio::FileMonitor>>,
}

struct ThemeWidgets {
    page: GtkBox,
    list: ListBox,
    active_label: Label,
    selected: Rc<RefCell<Option<String>>>,
    apply: Button,
    restore: Button,
    create: Button,
    duplicate: Button,
    rename: Button,
    delete: Button,
    export: Button,
    export_archive: Button,
    import_folder: Button,
    import_archive: Button,
    import_files: Button,
    save_current: Button,
    open_folder: Button,
    restore_defaults: Button,
    palette_grid: GtkBox,
}

struct TemplateWidgets {
    page: GtkBox,
    list: ListBox,
    selected: Rc<RefCell<Option<String>>>,
}

struct ScriptWidgets {
    page: GtkBox,
    name: Entry,
    source: TextView,
    interval: SpinButton,
    return_type: DropDown,
    zone: DropDown,
    add: Button,
}

struct FilesWidgets {
    page: GtkBox,
    open_project_config: Button,
    open_project_style: Button,
    open_live_config: Button,
    open_live_style: Button,
    open_folder: Button,
}

struct AboutWidgets {
    page: GtkBox,
    diagnostic: TextView,
    copy_diagnostic: Button,
    open_config: Button,
}

struct StyleWidgets {
    page: GtkBox,
    palette: DropDown,
    palette_name: Entry,
    palette_colors: TextView,
    save_palette: Button,
    palette_sync: Cell<bool>,
    font_preset: DropDown,
    font_family: Entry,
    font_size: SpinButton,
    foreground: DropDown,
    background: DropDown,
    opacity: SpinButton,
    border_radius: SpinButton,
    border_width: SpinButton,
    border_color: DropDown,
    border_style: DropDown,
    shadow: CheckButton,
    module_padding_vertical: SpinButton,
    module_padding_horizontal: SpinButton,
    module_margin_horizontal: SpinButton,
    spacing: SpinButton,
    height: SpinButton,
    margin_top: SpinButton,
    margin_bottom: SpinButton,
    margin_left: SpinButton,
    margin_right: SpinButton,
    module: DropDown,
    state: DropDown,
    module_foreground: DropDown,
    module_background: DropDown,
    module_css: TextView,
    generated_css: TextView,
    advanced_css: TextView,
    save_override: Button,
    clear_override: Button,
    save_css: Button,
    retro: Button,
    apply_test: Button,
    confirm_test: Button,
    rollback_test: Button,
    countdown: Label,
}

pub fn run() -> Result<()> {
    let application = Application::builder()
        .application_id("com.aspire.bar")
        .build();
    let activation_error = Rc::new(RefCell::new(None));
    let error_slot = Rc::clone(&activation_error);

    application.connect_activate(move |application| {
        let result =
            apply::load_project_or_live().and_then(|document| build_editor(application, document));
        if let Err(error) = result {
            eprintln!("aspire-bar: no se pudo abrir la interfaz: {error:#}");
            *error_slot.borrow_mut() = Some(format!("{error:#}"));
            application.quit();
        }
    });

    let exit_code = application.run();
    if let Some(error) = activation_error.borrow_mut().take() {
        bail!("{error}");
    }
    if exit_code != gtk::glib::ExitCode::SUCCESS {
        bail!("la interfaz terminó con código {exit_code:?}");
    }
    Ok(())
}

fn build_editor(application: &Application, document: BarDocument) -> Result<()> {
    let saved_document = document.clone();
    let initial_selection = first_module_selection(&document, 0);
    let window = ApplicationWindow::builder()
        .application(application)
        .title("aspire-bar — Editor de Waybar")
        .default_width(1560)
        .default_height(900)
        .build();

    let header = GtkBox::new(Orientation::Horizontal, 8);
    header.set_margin_top(6);
    header.set_margin_bottom(6);
    header.set_margin_start(12);
    header.set_margin_end(12);
    header.set_widget_name("aspire-header");

    let title_box = GtkBox::new(Orientation::Vertical, 0);
    let title = Label::new(Some("aspire-bar"));
    title.add_css_class("title-1");
    title.set_halign(gtk::Align::Start);
    title_box.append(&title);
    header.append(&title_box);

    let bar_dropdown = DropDown::from_strings(&[]);
    bar_dropdown.set_hexpand(false);
    header.append(&bar_dropdown);
    let add_bar = Button::with_label("＋ Barra");
    add_bar.set_tooltip_text(Some("Añadir otra barra al documento"));
    let remove_bar = Button::with_label("− Barra");
    remove_bar.set_tooltip_text(Some("Eliminar la barra seleccionada"));
    let collapse_catalog = Button::with_label("‹");
    collapse_catalog.set_tooltip_text(Some("Contraer el panel de secciones/catálogo"));
    collapse_catalog.update_property(&[gtk::accessible::Property::Label(
        "Contraer o expandir el panel de secciones y catálogo",
    )]);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    let quick_theme_button = MenuButton::new();
    quick_theme_button.add_css_class("suggested-action");
    quick_theme_button.set_tooltip_text(Some("Cambiar tema de Waybar"));
    let quick_theme_label = Label::new(Some("Tema: cargando…  ▾"));
    quick_theme_button.set_child(Some(&quick_theme_label));
    let quick_theme_popover = gtk::Popover::new();
    let quick_theme_menu = GtkBox::new(Orientation::Vertical, 4);
    quick_theme_menu.set_margin_top(8);
    quick_theme_menu.set_margin_bottom(8);
    quick_theme_menu.set_margin_start(8);
    quick_theme_menu.set_margin_end(8);
    let quick_theme_scroll = ScrolledWindow::new();
    quick_theme_scroll.set_min_content_height(320);
    quick_theme_scroll.set_max_content_height(520);
    quick_theme_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    quick_theme_scroll.set_child(Some(&quick_theme_menu));
    quick_theme_popover.set_child(Some(&quick_theme_scroll));
    quick_theme_button.set_popover(Some(&quick_theme_popover));
    header.append(&quick_theme_button);
    let previous_theme_button = Button::with_label("↶ Anterior");
    previous_theme_button.set_tooltip_text(Some("Volver al tema usado anteriormente"));
    previous_theme_button.set_sensitive(false);
    header.append(&spacer);

    let status = Label::new(Some("Listo"));
    status.set_ellipsize(gtk::pango::EllipsizeMode::End);
    status.set_max_width_chars(80);
    status.set_halign(gtk::Align::Start);
    status.set_hexpand(true);

    let error_button = MenuButton::new();
    error_button.set_icon_name("dialog-warning-symbolic");
    error_button.add_css_class("flat");
    error_button.set_tooltip_text(Some("Ver detalle de la validación"));
    error_button.set_visible(false);
    let error_label = Label::new(None);
    error_label.set_wrap(true);
    error_label.set_max_width_chars(64);
    error_label.set_margin_top(10);
    error_label.set_margin_bottom(10);
    error_label.set_margin_start(12);
    error_label.set_margin_end(12);
    let error_popover = gtk::Popover::new();
    error_popover.set_child(Some(&error_label));
    error_button.set_popover(Some(&error_popover));

    let validate_button = Button::with_label("Validar");
    header.append(&validate_button);
    header.append(&error_button);
    let save_button = Button::with_label("Guardar");
    save_button.add_css_class("suggested-action");
    save_button.set_tooltip_text(Some("Guardar en el tema activo y aplicar a Waybar (Ctrl+S)"));
    header.append(&save_button);

    let footer = GtkBox::new(Orientation::Horizontal, 8);
    footer.set_margin_top(2);
    footer.set_margin_bottom(2);
    footer.set_margin_start(12);
    footer.set_margin_end(12);
    footer.set_widget_name("aspire-footer");
    footer.append(&add_bar);
    footer.append(&remove_bar);
    footer.append(&collapse_catalog);
    footer.append(&previous_theme_button);
    footer.append(&status);

    let modules = catalog::modules();
    let module_ids: Vec<&str> = modules.iter().map(|module| module.id.as_str()).collect();
    let module_dropdown = DropDown::from_strings(&module_ids);
    if let Some(index) = module_ids.iter().position(|id| *id == "custom/new-module") {
        module_dropdown.set_selected(index as u32);
    }
    module_dropdown.set_visible(false);
    let search = Entry::new();
    search.set_placeholder_text(Some("Buscar módulo…"));
    let custom_name = Entry::new();
    custom_name.set_placeholder_text(Some("ID personalizado: custom/cava"));
    custom_name.set_hexpand(true);
    let add_module = Button::with_label("＋ Añadir");
    add_module.set_tooltip_text(Some(
        "Añadir el módulo personalizado a la zona seleccionada",
    ));
    let search_row = GtkBox::new(Orientation::Horizontal, 8);
    search_row.append(&search);

    let catalog = gtk::FlowBox::new();
    catalog.set_row_spacing(4);
    catalog.set_column_spacing(4);
    catalog.set_hexpand(true);
    let catalog_scroll = ScrolledWindow::new();
    catalog_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    catalog_scroll.set_vexpand(false);
    catalog_scroll.set_min_content_height(80);
    catalog_scroll.set_max_content_height(260);
    catalog_scroll.set_child(Some(&catalog));
    let custom_row = GtkBox::new(Orientation::Horizontal, 6);
    custom_row.append(&custom_name);
    custom_row.append(&add_module);

    let module_page = GtkBox::new(Orientation::Vertical, 10);
    module_page.set_margin_top(12);
    module_page.set_margin_bottom(12);
    module_page.set_margin_start(12);
    module_page.set_margin_end(12);
    let catalog_heading = Label::new(Some("Catálogo de módulos"));
    catalog_heading.add_css_class("title-3");
    catalog_heading.set_halign(gtk::Align::Start);
    module_page.append(&catalog_heading);
    module_page.append(&search_row);
    module_page.append(&catalog_scroll);
    let custom_heading = Label::new(Some("Añadir módulo personalizado"));
    custom_heading.add_css_class("heading");
    custom_heading.set_halign(gtk::Align::Start);
    module_page.append(&custom_heading);
    module_page.append(&custom_row);

    let form = GtkBox::new(Orientation::Vertical, 10);
    form.set_margin_top(4);
    form.set_margin_bottom(8);
    form.set_margin_start(8);
    form.set_margin_end(8);
    let form_scroll = ScrolledWindow::new();
    form_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    form_scroll.set_vexpand(true);
    form_scroll.set_child(Some(&form));

    let inspector = GtkBox::new(Orientation::Vertical, 6);
    inspector.set_size_request(260, -1);
    inspector.set_margin_top(8);
    inspector.set_margin_bottom(8);
    inspector.set_margin_start(8);
    inspector.set_margin_end(8);
    let summary = Label::new(Some("Barra superior"));
    summary.set_halign(gtk::Align::Start);
    summary.set_ellipsize(gtk::pango::EllipsizeMode::End);
    summary.set_max_width_chars(40);
    footer.append(&summary);
    let zone_heading = Label::new(Some("Zonas"));
    zone_heading.add_css_class("heading");
    zone_heading.set_halign(gtk::Align::Start);
    let zone_summary = Label::new(Some("L 0  ·  C 0  ·  R 0"));
    zone_summary.set_halign(gtk::Align::Start);
    let active_heading = Label::new(Some("Activos"));
    active_heading.add_css_class("heading");
    active_heading.set_halign(gtk::Align::Start);
    let active_modules = ListBox::new();
    active_modules.set_selection_mode(gtk::SelectionMode::Single);
    active_modules.add_css_class("boxed-list");
    let active_scroll = ScrolledWindow::new();
    active_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    active_scroll.set_min_content_height(60);
    active_scroll.set_vexpand(true);
    active_scroll.set_child(Some(&active_modules));
    let properties_heading = Label::new(Some("Propiedades del módulo"));
    properties_heading.add_css_class("heading");
    properties_heading.set_halign(gtk::Align::Start);
    let actions_heading = Label::new(Some("Acciones"));
    actions_heading.add_css_class("heading");
    actions_heading.set_halign(gtk::Align::Start);
    let actions = GtkBox::new(Orientation::Horizontal, 4);
    let move_up = Button::with_label("↑");
    move_up.set_tooltip_text(Some("Subir módulo"));
    move_up.set_hexpand(true);
    let move_down = Button::with_label("↓");
    move_down.set_tooltip_text(Some("Bajar módulo"));
    move_down.set_hexpand(true);
    let duplicate = Button::with_label("⧉");
    duplicate.set_tooltip_text(Some("Duplicar módulo"));
    duplicate.set_hexpand(true);
    let remove = Button::with_label("−");
    remove.add_css_class("destructive-action");
    remove.set_tooltip_text(Some("Quitar módulo de la barra"));
    remove.set_hexpand(true);
    actions.append(&move_up);
    actions.append(&move_down);
    actions.append(&duplicate);
    actions.append(&remove);
    let generate = Button::with_label("Generar config");
    generate.set_hexpand(true);
    let open_files = Button::with_label("Editar archivos");
    open_files.set_hexpand(true);
    let open_scripts = Button::with_label("Scripts custom");
    open_scripts.set_hexpand(true);
    let open_themes = Button::with_label("Temas");
    open_themes.set_hexpand(true);
    inspector.append(&zone_heading);
    inspector.append(&zone_summary);
    inspector.append(&active_heading);
    inspector.append(&active_scroll);
    inspector.append(&properties_heading);
    inspector.append(&form_scroll);
    inspector.append(&actions_heading);
    inspector.append(&actions);
    inspector.append(&generate);
    inspector.append(&open_files);
    inspector.append(&open_scripts);
    inspector.append(&open_themes);

    let preview = Rc::new(Preview::new());

    let style = build_style_editor()?;
    let style_scroll = ScrolledWindow::new();
    style_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    style_scroll.set_child(Some(&style.page));
    let notebook = Notebook::new();
    notebook.set_scrollable(true);
    notebook.set_tab_pos(gtk::PositionType::Left);
    notebook.append_page(
        &module_page,
        Some(&tab_label("applications-utilities-symbolic", "Módulos")),
    );
    notebook.append_page(
        &style_scroll,
        Some(&tab_label("preferences-desktop-theme-symbolic", "Ajustes")),
    );
    let themes = build_theme_gallery();
    let themes_scroll = ScrolledWindow::new();
    themes_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    themes_scroll.set_child(Some(&themes.page));
    notebook.append_page(
        &themes_scroll,
        Some(&tab_label("preferences-color-symbolic", "Temas")),
    );
    let templates = build_template_gallery()?;
    let templates_scroll = ScrolledWindow::new();
    templates_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    templates_scroll.set_child(Some(&templates.page));
    notebook.append_page(
        &templates_scroll,
        Some(&tab_label("view-grid-symbolic", "Plantillas")),
    );
    let scripts = build_scripts_editor();
    let scripts_scroll = ScrolledWindow::new();
    scripts_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    scripts_scroll.set_child(Some(&scripts.page));
    notebook.append_page(
        &scripts_scroll,
        Some(&tab_label("text-x-script-symbolic", "Scripts")),
    );
    let files = build_edit_files_page();
    notebook.append_page(
        &files.page,
        Some(&tab_label("text-editor-symbolic", "Editar archivos")),
    );
    let about = build_about_page();
    let about_scroll = ScrolledWindow::new();
    about_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    about_scroll.set_child(Some(&about.page));
    notebook.append_page(
        &about_scroll,
        Some(&tab_label("help-about-symbolic", "Acerca de")),
    );
    inspector.set_size_request(330, -1);
    inspector.set_hexpand(false);
    inspector.set_vexpand(true);

    let is_catalog_collapsed = Rc::new(Cell::new(false));
    let collapsed_state = Rc::clone(&is_catalog_collapsed);
    let notebook_for_collapse = notebook.clone();
    collapse_catalog.connect_clicked(move |button| {
        let collapsed = !collapsed_state.get();
        collapsed_state.set(collapsed);
        notebook_for_collapse.set_size_request(if collapsed { 150 } else { 0 }, -1);
        if collapsed {
            button.set_label("›");
            button.set_tooltip_text(Some("Expandir el panel de secciones/catálogo"));
        } else {
            button.set_label("‹");
            button.set_tooltip_text(Some("Contraer el panel de secciones/catálogo"));
        }
    });

    let preview_container = GtkBox::new(Orientation::Vertical, 0);
    preview_container.set_hexpand(true);
    preview_container.set_vexpand(false);
    preview_container.set_size_request(-1, 96);
    preview_container.append(&preview.page);
    preview_container.set_widget_name("aspire-preview-top");

    let content_area = GtkBox::new(Orientation::Horizontal, 0);
    content_area.set_vexpand(true);
    content_area.set_hexpand(true);
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);
    content_area.append(&notebook);
    content_area.append(&gtk::Separator::new(Orientation::Vertical));
    content_area.append(&inspector);

    let root = GtkBox::new(Orientation::Vertical, 0);
    root.append(&header);
    root.append(&gtk::Separator::new(Orientation::Horizontal));
    root.append(&preview_container);
    root.append(&gtk::Separator::new(Orientation::Horizontal));
    root.append(&content_area);
    root.append(&gtk::Separator::new(Orientation::Horizontal));
    root.append(&footer);
    root.set_vexpand(true);

    window.set_child(Some(&root));

    let ui = Rc::new(EditorUi {
        window: window.clone(),
        state: RefCell::new(EditorState {
            document,
            bar_index: 0,
            selection: initial_selection,
        }),
        saved_document: RefCell::new(saved_document),
        bar_dropdown,
        module_dropdown,
        custom_name,
        search,
        catalog,
        active_modules,
        zone_summary,
        summary,
        form,
        status,
        error_button,
        error_label,
        save_button: save_button.clone(),
        style,
        themes,
        templates,
        scripts,
        files,
        about,
        preview,
        quick_theme_label,
        quick_theme_menu,
        previous_theme_button,
        previous_theme: RefCell::new(None),
        selection_sync: Cell::new(false),
        pending_test: RefCell::new(None),
        pending_document: RefCell::new(None),
        test_seconds: Cell::new(0),
        test_generation: Cell::new(0),
        monitors: RefCell::new(Vec::new()),
    });

    refresh_style_controls(&ui, true);
    refresh_style_module_choices(&ui);
    connect_bar_selection(&ui);
    connect_zone_lists(&ui);
    connect_drop_targets(&ui);
    connect_toolbar(&ui, &window, &add_module, &add_bar, &remove_bar, &validate_button, &save_button);
    connect_module_actions(&ui, &move_up, &move_down, &duplicate, &remove);
    connect_style_controls(&ui);
    connect_theme_controls(&ui, &window, &notebook);
    connect_template_controls(&ui, &window);
    connect_script_controls(&ui);
    connect_quick_theme_controls(&ui);
    connect_inspector_actions(
        &ui,
        &notebook,
        &generate,
        &open_files,
        &open_scripts,
        &open_themes,
    );
    connect_about_controls(&ui, &notebook);
    connect_file_page_controls(&ui);
    let ui_for_search = Rc::clone(&ui);
    ui.search
        .connect_changed(move |_| refresh_catalog(&ui_for_search));
    refresh_theme_gallery(&ui);
    refresh_template_gallery(&ui);
    refresh_bar_dropdown(&ui);
    refresh_zones(&ui);
    let ui_for_preview_selection = Rc::clone(&ui);
    ui.preview.set_on_module_selected(move |zone, module| {
        set_module_selection(&ui_for_preview_selection, zone, &module);
    });
    refresh_preview(&ui);
    refresh_form(&ui);
    refresh_quick_theme_menu(&ui);
    set_status(&ui, "Listo");
    install_editor_css()?;
    install_monitors(&ui);
    let ui_for_close = Rc::clone(&ui);
    let allow_close = Rc::new(Cell::new(false));
    let allow_close_for_request = Rc::clone(&allow_close);
    window.connect_close_request(move |window| {
        let mut pending = ui_for_close.pending_test.borrow_mut();
        if let Some(active_test) = pending.as_mut() {
            if let Err(error) = active_test.rollback() {
                set_status(
                    &ui_for_close,
                    &format!("No se puede cerrar: falló la reversión de prueba: {error:#}"),
                );
                return gtk::glib::Propagation::Stop;
            }
            pending.take();
            ui_for_close.pending_document.borrow_mut().take();
            set_status(&ui_for_close, "Prueba revertida al cerrar aspire-bar.");
        }
        drop(pending);
        if !allow_close_for_request.get() && has_unsaved_changes(&ui_for_close) {
            let dialog = gtk::AlertDialog::builder()
                .message("Cambios sin guardar")
                .detail("Guarda en el tema activo, descarta los cambios o cancela el cierre.")
                .buttons(["Cancelar", "Descartar", "Guardar"])
                .cancel_button(0)
                .default_button(2)
                .build();
            let ui_for_choice = Rc::clone(&ui_for_close);
            let window_for_choice = window.clone();
            let allow_close_for_choice = Rc::clone(&allow_close_for_request);
            dialog.choose(
                Some(window),
                None::<&gtk::gio::Cancellable>,
                move |result| match result {
                    Ok(1) => {
                        allow_close_for_choice.set(true);
                        window_for_choice.close();
                    }
                    Ok(2) => {
                        save_and_apply(&ui_for_choice, &window_for_choice);
                    }
                    _ => {}
                },
            );
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    window.present();
    Ok(())
}
fn build_theme_gallery() -> ThemeWidgets {
    let page = GtkBox::new(Orientation::Vertical, 10);
    page.set_margin_top(14);
    page.set_margin_bottom(14);
    page.set_margin_start(16);
    page.set_margin_end(16);

    let heading_row = GtkBox::new(Orientation::Horizontal, 10);
    let heading = Label::new(Some("Temas"));
    heading.add_css_class("title-2");
    heading.set_halign(gtk::Align::Start);
    heading_row.append(&heading);
    let heading_spacer = GtkBox::new(Orientation::Horizontal, 0);
    heading_spacer.set_hexpand(true);
    heading_row.append(&heading_spacer);
    let active_label = Label::new(Some("Activo: colores dinámicos del wallpaper"));
    active_label.set_halign(gtk::Align::End);
    active_label.add_css_class("dim-label");
    heading_row.append(&active_label);
    page.append(&heading_row);

    let actions_row = GtkBox::new(Orientation::Horizontal, 6);
    actions_row.set_halign(gtk::Align::Start);
    let create = Button::with_label("＋ Crear");
    create.add_css_class("suggested-action");
    create.set_tooltip_text(Some("Crear un tema nuevo con la configuración actual"));
    let import_theme = MenuButton::new();
    import_theme.set_label("Importar…");
    import_theme.set_tooltip_text(Some("Importar una carpeta, .tar.zst, .zip o dos archivos"));
    let import_popover = gtk::Popover::new();
    let import_box = GtkBox::new(Orientation::Vertical, 4);
    import_box.set_margin_top(8);
    import_box.set_margin_bottom(8);
    import_box.set_margin_start(8);
    import_box.set_margin_end(8);
    let import_folder = Button::with_label("Carpeta de tema…");
    let import_archive = Button::with_label("Paquete .tar.zst / .zip…");
    let import_files = Button::with_label("Dos archivos (config + style.css)…");
    for button in [&import_folder, &import_archive, &import_files] {
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");
        import_box.append(button);
    }
    import_popover.set_child(Some(&import_box));
    import_theme.set_popover(Some(&import_popover));
    let save_current = Button::with_label("Guardar mi Waybar actual");
    save_current.set_tooltip_text(Some("Copiar la Waybar en uso a un tema nuevo"));
    let open_folder = Button::with_label("Abrir carpeta");
    open_folder.set_tooltip_text(Some("Abrir ~/.config/aspire-bar/themes"));
    let restore_defaults = Button::with_label("Restaurar predeterminados");
    restore_defaults.set_tooltip_text(Some("Mostrar otra vez los temas predeterminados ocultos"));
    actions_row.append(&create);
    actions_row.append(&import_theme);
    actions_row.append(&save_current);
    actions_row.append(&open_folder);
    actions_row.append(&restore_defaults);
    page.append(&actions_row);

    let content = GtkBox::new(Orientation::Horizontal, 14);
    content.set_vexpand(true);
    let list = ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_vexpand(true);
    list.add_css_class("boxed-list");
    let list_scroll = ScrolledWindow::new();
    list_scroll.set_size_request(430, -1);
    list_scroll.set_hexpand(false);
    list_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    list_scroll.set_child(Some(&list));
    content.append(&list_scroll);

    let details = GtkBox::new(Orientation::Vertical, 10);
    details.set_hexpand(true);
    details.set_vexpand(true);
    let palette_heading = Label::new(Some("Paleta del tema"));
    palette_heading.add_css_class("heading");
    palette_heading.set_halign(gtk::Align::Start);
    details.append(&palette_heading);
    let palette_note = Label::new(Some(
        "La paleta dinámica sigue el wallpaper; las paletas fijas conservan sus propios colores.",
    ));
    palette_note.set_wrap(true);
    palette_note.set_halign(gtk::Align::Start);
    palette_note.add_css_class("dim-label");
    details.append(&palette_note);
    let palette_grid = GtkBox::new(Orientation::Vertical, 6);
    palette_grid.set_vexpand(true);
    let palette_scroll = ScrolledWindow::new();
    palette_scroll.set_min_content_height(200);
    palette_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    palette_scroll.set_child(Some(&palette_grid));
    details.append(&palette_scroll);

    let theme_actions = GtkBox::new(Orientation::Horizontal, 8);
    let apply = Button::with_label("Usar tema");
    apply.add_css_class("suggested-action");
    let restore = Button::with_label("Restaurar último backup");
    theme_actions.append(&apply);
    theme_actions.append(&restore);
    details.append(&theme_actions);

    let package_actions = GtkBox::new(Orientation::Horizontal, 8);
    let duplicate = Button::with_label("Duplicar");
    let rename = Button::with_label("Renombrar");
    let delete = Button::with_label("Eliminar");
    delete.add_css_class("destructive-action");
    duplicate.set_sensitive(false);
    rename.set_sensitive(false);
    delete.set_sensitive(false);
    package_actions.append(&duplicate);
    package_actions.append(&rename);
    package_actions.append(&delete);
    details.append(&package_actions);

    let file_actions = GtkBox::new(Orientation::Horizontal, 8);
    let export = Button::with_label("Exportar carpeta");
    let export_archive = Button::with_label("Exportar .tar.zst");
    file_actions.append(&export);
    file_actions.append(&export_archive);
    details.append(&file_actions);
    content.append(&details);
    page.append(&content);

    ThemeWidgets {
        page,
        list,
        active_label,
        selected: Rc::new(RefCell::new(None)),
        apply,
        restore,
        create,
        duplicate,
        rename,
        delete,
        export,
        export_archive,
        import_folder,
        import_archive,
        import_files,
        save_current,
        open_folder,
        restore_defaults,
        palette_grid,
    }
}

fn build_template_gallery() -> Result<TemplateWidgets> {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_top(16);
    page.set_margin_bottom(16);
    page.set_margin_start(18);
    page.set_margin_end(18);

    let heading = Label::new(Some("Plantillas de estructura"));
    heading.add_css_class("title-2");
    heading.set_halign(gtk::Align::Start);
    page.append(&heading);
    let note = Label::new(Some(
        "Las plantillas cambian únicamente módulos, posición y altura. La paleta y el estilo de la configuración se conservan.",
    ));
    note.set_wrap(true);
    note.set_halign(gtk::Align::Start);
    note.add_css_class("dim-label");
    page.append(&note);

    let list = ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_vexpand(true);
    list.add_css_class("boxed-list");
    let scroll = ScrolledWindow::new();
    scroll.set_min_content_height(480);
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_child(Some(&list));
    page.append(&scroll);

    Ok(TemplateWidgets {
        page,
        list,
        selected: Rc::new(RefCell::new(None)),
    })
}

fn build_edit_files_page() -> FilesWidgets {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_top(18);
    page.set_margin_bottom(18);
    page.set_margin_start(18);
    page.set_margin_end(18);
    let heading = Label::new(Some("Editar archivos"));
    heading.add_css_class("title-2");
    heading.set_halign(gtk::Align::Start);
    page.append(&heading);
    let note = Label::new(Some(
        "Abre la configuración de aspire-bar o la configuración viva de Waybar en la aplicación asociada.",
    ));
    note.set_wrap(true);
    note.set_halign(gtk::Align::Start);
    note.add_css_class("dim-label");
    page.append(&note);
    let open_project_config = Button::with_label("Editar config.jsonc");
    let open_project_style = Button::with_label("Editar style.css");
    let open_live_config = Button::with_label("Editar config.jsonc de Waybar");
    let open_live_style = Button::with_label("Editar style.css de Waybar");
    let open_folder = Button::with_label("Abrir carpeta de configuración");
    for button in [
        &open_project_config,
        &open_project_style,
        &open_live_config,
        &open_live_style,
        &open_folder,
    ] {
        button.set_halign(gtk::Align::Start);
        page.append(button);
    }
    FilesWidgets {
        page,
        open_project_config,
        open_project_style,
        open_live_config,
        open_live_style,
        open_folder,
    }
}

fn build_about_page() -> AboutWidgets {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_top(18);
    page.set_margin_bottom(18);
    page.set_margin_start(18);
    page.set_margin_end(18);
    let heading = Label::new(Some("aspire-bar"));
    heading.add_css_class("title-1");
    heading.set_halign(gtk::Align::Start);
    let description = Label::new(Some(
        "Editor visual de Waybar en Rust + GTK4, integrado con MangoWM y aspire-core.",
    ));
    description.set_wrap(true);
    description.set_halign(gtk::Align::Start);
    let version = Label::new(Some(&format!(
        "Versión {} · commit {}",
        env!("CARGO_PKG_VERSION"),
        build_commit()
    )));
    version.set_halign(gtk::Align::Start);
    page.append(&heading);
    page.append(&description);
    page.append(&version);

    let credits = Label::new(Some(
        "Inspirado en vsWaybar-Studio (victorsosaMx, MIT). Gracias a Waybar, MangoWM y a las comunidades de Catppuccin, Dracula, Nord, Gruvbox, Tokyo Night, Rosé Pine y Everforest. El manual de Aspire declara GPL-3.0-or-later para la suite.",
    ));
    credits.set_wrap(true);
    credits.set_halign(gtk::Align::Start);
    page.append(&credits);

    let sources = GtkBox::new(Orientation::Vertical, 3);
    for (name, url) in [
        (
            "vsWaybar-Studio · MIT",
            "https://github.com/victorsosaMx/vsWaybar-Studio",
        ),
        ("Waybar", "https://github.com/Alexays/Waybar"),
        ("MangoWM", "https://github.com/DreamMaoMao/mangowc"),
        ("Catppuccin", "https://github.com/catppuccin/catppuccin"),
        ("Dracula", "https://draculatheme.com/"),
        ("Nord", "https://www.nordtheme.com/"),
        ("Gruvbox", "https://github.com/morhetz/gruvbox"),
        ("Tokyo Night", "https://github.com/tokyo-night"),
        ("Rosé Pine", "https://rosepinetheme.com/"),
        ("Everforest", "https://github.com/sainnhe/everforest"),
    ] {
        let link = gtk::LinkButton::with_label(url, name);
        link.set_halign(gtk::Align::Start);
        sources.append(&link);
    }
    page.append(&sources);

    let links = GtkBox::new(Orientation::Horizontal, 8);
    links.set_halign(gtk::Align::Start);
    if let Some(url) = std::env::var("ASPIRE_BAR_REPOSITORY_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
    {
        links.append(&gtk::LinkButton::with_label(&url, "Repositorio"));
    } else {
        links.append(&Label::new(Some("Repositorio: no configurado")));
    }
    if let Some(url) = std::env::var("ASPIRE_BAR_DOCUMENTATION_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
    {
        links.append(&gtk::LinkButton::with_label(&url, "Documentación"));
    } else {
        links.append(&Label::new(Some("Documentación: no configurada")));
    }
    page.append(&links);

    let diagnostic_heading = Label::new(Some("Diagnóstico"));
    diagnostic_heading.add_css_class("heading");
    diagnostic_heading.set_halign(gtk::Align::Start);
    page.append(&diagnostic_heading);
    let diagnostic = TextView::new();
    diagnostic.set_editable(false);
    diagnostic.set_cursor_visible(false);
    diagnostic.set_monospace(true);
    diagnostic.set_wrap_mode(gtk::WrapMode::WordChar);
    let diagnostic_scroll = ScrolledWindow::new();
    diagnostic_scroll.set_vexpand(true);
    diagnostic_scroll.set_min_content_height(180);
    diagnostic_scroll.set_child(Some(&diagnostic));
    page.append(&diagnostic_scroll);
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    let copy_diagnostic = Button::with_label("Copiar diagnóstico");
    let open_config = Button::with_label("Abrir carpeta de configuración");
    actions.append(&copy_diagnostic);
    actions.append(&open_config);
    page.append(&actions);
    AboutWidgets {
        page,
        diagnostic,
        copy_diagnostic,
        open_config,
    }
}

fn build_scripts_editor() -> ScriptWidgets {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_top(16);
    page.set_margin_bottom(16);
    page.set_margin_start(18);
    page.set_margin_end(18);
    let heading = Label::new(Some("Scripts para módulos personalizados"));
    heading.add_css_class("title-2");
    heading.set_halign(gtk::Align::Start);
    page.append(&heading);
    let note = Label::new(Some(
        "El script se guarda en ~/.config/aspire-bar/scripts/ con backup al reemplazarlo. No se ejecuta durante la edición; Waybar lo lanzará al aplicar el módulo.",
    ));
    note.set_wrap(true);
    note.set_halign(gtk::Align::Start);
    note.add_css_class("dim-label");
    page.append(&note);

    let form = GtkBox::new(Orientation::Vertical, 8);
    let name = Entry::new();
    name.set_placeholder_text(Some("Nombre descriptivo, por ejemplo batería"));
    form.append(&labelled("Nombre del script", &name));

    let source = TextView::new();
    source.set_monospace(true);
    source.set_wrap_mode(gtk::WrapMode::None);
    source.set_size_request(-1, 260);
    source.buffer().set_text("printf 'estado listo\\n'\n");
    let source_scroll = ScrolledWindow::new();
    source_scroll.set_min_content_height(260);
    source_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    source_scroll.set_child(Some(&source));
    form.append(&labelled("Contenido Bash", &source_scroll));

    let options = GtkBox::new(Orientation::Horizontal, 10);
    let interval = SpinButton::with_range(1.0, 86_400.0, 1.0);
    interval.set_value(30.0);
    options.append(&labelled("Intervalo (s)", &interval));
    let return_type = DropDown::from_strings(&["Texto plano", "JSON"]);
    options.append(&labelled("Salida", &return_type));
    let zone = DropDown::from_strings(&["Izquierda", "Centro", "Derecha"]);
    zone.set_selected(2);
    options.append(&labelled("Zona", &zone));
    form.append(&options);

    let add = Button::with_label("Guardar script y añadir módulo");
    add.add_css_class("suggested-action");
    form.append(&add);
    page.append(&form);

    ScriptWidgets {
        page,
        name,
        source,
        interval,
        return_type,
        zone,
        add,
    }
}

fn labelled<W: IsA<gtk::Widget>>(title: &str, widget: &W) -> GtkBox {
    let column = GtkBox::new(Orientation::Vertical, 4);
    let label = Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    column.append(&label);
    column.append(widget);
    column
}

fn tab_label(icon_name: &str, _title: &str) -> GtkBox {
    let label = GtkBox::new(Orientation::Horizontal, 4);
    label.set_margin_start(2);
    label.set_margin_end(2);
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);
    label.append(&icon);
    label
}

fn connect_theme_controls(ui: &Rc<EditorUi>, window: &ApplicationWindow, notebook: &Notebook) {
    let ui_for_selection = Rc::clone(ui);
    ui.themes.list.connect_row_selected(move |_, row| {
        let name = row.and_then(|row| row.tooltip_text().map(|name| name.to_string()));
        *ui_for_selection.themes.selected.borrow_mut() = name.clone();
        if let Some(name) = name {
            match themes::find_package(&name) {
                Ok(package) => {
                    let active = match themes::active_theme() {
                        Ok(active) => active.as_deref() == Some(name.as_str()),
                        Err(error) => {
                            set_status(
                                &ui_for_selection,
                                &format!("No se pudo leer el tema activo: {error:#}"),
                            );
                            false
                        }
                    };
                    ui_for_selection.themes.duplicate.set_sensitive(true);
                    ui_for_selection
                        .themes
                        .rename
                        .set_sensitive(!package.is_default);
                    ui_for_selection
                        .themes
                        .delete
                        .set_sensitive(!package.is_default && !active);
                    ui_for_selection.preview.render(&package.document);
                    refresh_theme_palette_grid(&ui_for_selection, &name, package.is_default);
                }
                Err(error) => set_status(
                    &ui_for_selection,
                    &format!("No se pudo cargar «{name}»: {error:#}"),
                ),
            }
        }
    });

    let ui_for_apply = Rc::clone(ui);
    ui.themes.apply.connect_clicked(move |_| {
        let Some(name) = ui_for_apply.themes.selected.borrow().clone() else {
            set_status(&ui_for_apply, "Selecciona un tema de la galería.");
            return;
        };
        match apply_gallery_theme(&ui_for_apply, &name) {
            Ok(()) => {
                notify_app("Tema aplicado", &name);
                set_status(&ui_for_apply, &format!("Tema «{name}» aplicado."))
            },
            Err(error) => set_status(
                &ui_for_apply,
                &format!("No se pudo aplicar «{name}»: {error:#}"),
            ),
        }
    });

    let ui_for_restore = Rc::clone(ui);
    ui.themes
        .restore
        .connect_clicked(move |_| match apply::restore() {
            Ok(()) => set_status(&ui_for_restore, "Último backup restaurado."),
            Err(error) => set_status(&ui_for_restore, &format!("No se pudo restaurar: {error:#}")),
        });

    let parent = window.clone();
    let ui_for_create = Rc::clone(ui);
    ui.themes.create.connect_clicked(move |_| {
        let ui_for_submit = Rc::clone(&ui_for_create);
        prompt_text(
            &parent,
            "Crear tema",
            "Nombre del tema",
            "",
            move |display_name| match themes::slugify(&display_name).and_then(|slug| {
                let manifest = gallery_manifest(&ui_for_submit, &slug, &display_name)?;
                let document = ui_for_submit.state.borrow().document.clone();
                themes::create_theme(&manifest, &document)
            }) {
                Ok(path) => {
                    refresh_theme_gallery(&ui_for_submit);
                    set_status(
                        &ui_for_submit,
                        &format!("Tema creado en {}.", path.display()),
                    );
                }
                Err(error) => set_status(
                    &ui_for_submit,
                    &format!("No se pudo crear el tema: {error:#}"),
                ),
            },
        );
    });

    let parent = window.clone();
    let ui_for_duplicate = Rc::clone(ui);
    ui.themes.duplicate.connect_clicked(move |_| {
        let Some(source) = ui_for_duplicate.themes.selected.borrow().clone() else {
            set_status(
                &ui_for_duplicate,
                "Selecciona el tema que quieres duplicar.",
            );
            return;
        };
        let ui_for_submit = Rc::clone(&ui_for_duplicate);
        prompt_text(
            &parent,
            "Duplicar tema",
            "Nombre de la copia",
            "",
            move |display_name| {
                let result = themes::slugify(&display_name).and_then(|slug| {
                    themes::duplicate_theme(&source, &slug)?;
                    themes::rename_theme(&slug, &slug, &display_name)
                });
                match result {
                    Ok(path) => {
                        refresh_theme_gallery(&ui_for_submit);
                        set_status(
                            &ui_for_submit,
                            &format!("Tema duplicado en {}.", path.display()),
                        );
                    }
                    Err(error) => set_status(
                        &ui_for_submit,
                        &format!("No se pudo duplicar el tema: {error:#}"),
                    ),
                }
            },
        );
    });

    let parent = window.clone();
    let ui_for_rename = Rc::clone(ui);
    ui.themes.rename.connect_clicked(move |_| {
        let Some(source) = ui_for_rename.themes.selected.borrow().clone() else {
            set_status(&ui_for_rename, "Selecciona el tema que quieres renombrar.");
            return;
        };
        let package = match themes::find_package(&source) {
            Ok(package) if package.is_default => {
                set_status(
                    &ui_for_rename,
                    "Los temas predeterminados son de solo lectura.",
                );
                return;
            }
            Ok(package) => package,
            Err(error) => {
                set_status(
                    &ui_for_rename,
                    &format!("No se pudo leer el tema: {error:#}"),
                );
                return;
            }
        };
        let initial = package.manifest.display_name;
        let ui_for_submit = Rc::clone(&ui_for_rename);
        prompt_text(
            &parent,
            "Renombrar tema",
            "Nombre visible",
            &initial,
            move |display_name| {
                let result = themes::slugify(&display_name)
                    .and_then(|slug| themes::rename_theme(&source, &slug, &display_name));
                match result {
                    Ok(path) => {
                        *ui_for_submit.themes.selected.borrow_mut() = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(str::to_owned);
                        refresh_theme_gallery(&ui_for_submit);
                        set_status(&ui_for_submit, "Tema renombrado.");
                    }
                    Err(error) => set_status(
                        &ui_for_submit,
                        &format!("No se pudo renombrar el tema: {error:#}"),
                    ),
                }
            },
        );
    });

    let ui_for_delete = Rc::clone(ui);
    ui.themes.delete.connect_clicked(move |_| {
        let Some(name) = ui_for_delete.themes.selected.borrow().clone() else {
            set_status(&ui_for_delete, "Selecciona el tema que quieres eliminar.");
            return;
        };
        match themes::delete_theme(&name) {
            Ok(()) => {
                *ui_for_delete.themes.selected.borrow_mut() = None;
                refresh_theme_gallery(&ui_for_delete);
                set_status(&ui_for_delete, "Tema eliminado.");
            }
            Err(error) => set_status(
                &ui_for_delete,
                &format!("No se pudo eliminar el tema: {error:#}"),
            ),
        }
    });

    let ui_for_export = Rc::clone(ui);
    let parent = window.clone();
    ui.themes.export.connect_clicked(move |_| {
        let Some(name) = ui_for_export.themes.selected.borrow().clone() else {
            set_status(&ui_for_export, "Selecciona el tema que quieres exportar.");
            return;
        };
        let chooser = FileDialog::builder()
            .title("Exportar tema a una carpeta")
            .accept_label("Exportar")
            .build();
        let ui_for_response = Rc::clone(&ui_for_export);
        chooser.select_folder(
            Some(&parent),
            None::<&gtk::gio::Cancellable>,
            move |result| match file_dialog_path(result) {
                Ok(path) => export_theme_result(&ui_for_response, &name, &path),
                Err(error) => set_status(
                    &ui_for_response,
                    &format!("No se pudo elegir la carpeta destino: {error}"),
                ),
            },
        );
    });

    let ui_for_export = Rc::clone(ui);
    let parent = window.clone();
    ui.themes.export_archive.connect_clicked(move |_| {
        let Some(name) = ui_for_export.themes.selected.borrow().clone() else {
            set_status(&ui_for_export, "Selecciona el tema que quieres exportar.");
            return;
        };
        let chooser = FileDialog::builder()
            .title("Exportar tema como archivo")
            .accept_label("Guardar")
            .initial_name(format!("{name}.tar.zst"))
            .build();
        let ui_for_response = Rc::clone(&ui_for_export);
        chooser.save(
            Some(&parent),
            None::<&gtk::gio::Cancellable>,
            move |result| match file_dialog_path(result) {
                Ok(path) => export_theme_result(&ui_for_response, &name, &path),
                Err(error) => set_status(
                    &ui_for_response,
                    &format!("No se pudo elegir el archivo destino: {error}"),
                ),
            },
        );
    });

    connect_theme_folder_import(ui, window, &ui.themes.import_folder);
    connect_theme_archive_import(ui, window, &ui.themes.import_archive);

    let ui_for_import_files = Rc::clone(ui);
    let parent = window.clone();
    ui.themes.import_files.connect_clicked(move |_| {
        let chooser = FileDialog::builder()
            .title("Elegir config.jsonc de Waybar")
            .accept_label("Siguiente")
            .build();
        let ui_for_style = Rc::clone(&ui_for_import_files);
        let parent_for_style = parent.clone();
        chooser.open(
            Some(&parent),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                let Ok(config) = file_dialog_path(result) else {
                    return;
                };
                let style_chooser = FileDialog::builder()
                    .title("Elegir style.css del tema")
                    .accept_label("Importar")
                    .build();
                let ui_for_submit = Rc::clone(&ui_for_style);
                style_chooser.open(
                    Some(&parent_for_style),
                    None::<&gtk::gio::Cancellable>,
                    move |result| {
                        let Ok(style) = file_dialog_path(result) else {
                            return;
                        };
                        let config = config.clone();
                        let ui_for_name = Rc::clone(&ui_for_submit);
                        prompt_text(
                            &ui_for_submit.window,
                            "Importar dos archivos",
                            "Nombre del tema",
                            "",
                            move |display_name| {
                                match themes::import_files(&config, &style, &display_name) {
                                    Ok(path) => {
                                        refresh_theme_gallery(&ui_for_name);
                                        refresh_quick_theme_menu(&ui_for_name);
                                        notify_app("Tema importado", &display_name);
                                        set_status(
                                            &ui_for_name,
                                            &format!("Tema importado en {}.", path.display()),
                                        );
                                    }
                                    Err(error) => set_status(
                                        &ui_for_name,
                                        &format!("No se pudo importar: {error:#}"),
                                    ),
                                }
                            },
                        );
                    },
                );
            },
        );
    });

    let ui_for_restore_defaults = Rc::clone(ui);
    ui.themes.restore_defaults.connect_clicked(move |_| {
        match themes::restore_all_defaults() {
            Ok(()) => {
                refresh_theme_gallery(&ui_for_restore_defaults);
                set_status(&ui_for_restore_defaults, "Temas predeterminados restaurados.");
            }
            Err(error) => set_status(
                &ui_for_restore_defaults,
                &format!("No se pudo restaurar: {error:#}"),
            ),
        }
    });

    let ui_for_save = Rc::clone(ui);
    let parent = window.clone();
    ui.themes.save_current.connect_clicked(move |_| {
        let (config, style) = match apply::live_paths() {
            Ok(paths) => paths,
            Err(error) => {
                set_status(&ui_for_save, &format!("No se pudo leer Waybar: {error:#}"));
                return;
            }
        };
        let document = match BarDocument::read(&config, &style) {
            Ok(document) => document,
            Err(error) => {
                set_status(&ui_for_save, &format!("Waybar no se pudo cargar: {error:#}"));
                return;
            }
        };
        let ui_for_submit = Rc::clone(&ui_for_save);
        prompt_text(
            &parent,
            "Guardar Waybar actual como tema",
            "Nombre del tema",
            "",
            move |display_name| match themes::slugify(&display_name).and_then(|slug| {
                let manifest = gallery_manifest(&ui_for_submit, &slug, &display_name)?;
                themes::create_theme(&manifest, &document)
            }) {
                Ok(path) => {
                    refresh_theme_gallery(&ui_for_submit);
                    notify_app("Tema creado", &display_name);
                    set_status(&ui_for_submit, &format!("Tema guardado en {}.", path.display()));
                }
                Err(error) => set_status(
                    &ui_for_submit,
                    &format!("No se pudo guardar: {error:#}"),
                ),
            },
        );
    });

    let ui_for_open_folder = Rc::clone(ui);
    ui.themes.open_folder.connect_clicked(move |_| {
        let result = themes::user_root().and_then(|root| open_path(&root));
        report_open_result(&ui_for_open_folder, result, "carpeta de temas");
    });

    let ui_for_page = Rc::clone(ui);
    notebook.connect_switch_page(move |_, _, page| {
        if page == 2 {
            refresh_theme_gallery(&ui_for_page);
        } else {
            refresh_preview(&ui_for_page);
        }
    });
}

fn connect_quick_theme_controls(ui: &Rc<EditorUi>) {
    let ui_for_previous = Rc::clone(ui);
    ui.previous_theme_button.connect_clicked(move |_| {
        let Some(previous) = ui_for_previous.previous_theme.borrow().clone() else {
            set_status(&ui_for_previous, "No hay un tema anterior para restaurar.");
            return;
        };
        match apply_gallery_theme(&ui_for_previous, &previous) {
            Ok(()) => set_status(
                &ui_for_previous,
                &format!("Tema anterior «{previous}» aplicado."),
            ),
            Err(error) => set_status(
                &ui_for_previous,
                &format!("No se pudo aplicar «{previous}»: {error:#}"),
            ),
        }
    });
}

fn refresh_quick_theme_menu(ui: &Rc<EditorUi>) {
    clear_box(&ui.quick_theme_menu);
    let packages = match themes::list_packages() {
        Ok(packages) => packages,
        Err(error) => {
            ui.quick_theme_label.set_text("Tema: error  ▾");
            set_status(
                ui,
                &format!("No se pudo cargar el selector de temas: {error:#}"),
            );
            return;
        }
    };
    let hidden = themes::hidden_themes();
    let packages = packages
        .into_iter()
        .filter(|package| !hidden.iter().any(|name| name == &package.manifest.name))
        .collect::<Vec<_>>();
    let active = match themes::active_theme() {
        Ok(active) => active,
        Err(error) => {
            ui.quick_theme_label.set_text("Tema: desconocido  ▾");
            set_status(ui, &format!("No se pudo leer el tema activo: {error:#}"));
            None
        }
    };
    let active_label = active
        .as_deref()
        .and_then(|name| {
            packages
                .iter()
                .find(|package| package.manifest.name == name)
        })
        .map(|package| package.manifest.display_name.clone())
        .unwrap_or_else(|| {
            if active.is_some() {
                active.clone().unwrap_or_default()
            } else {
                "Dinámico".to_owned()
            }
        });
    ui.quick_theme_label
        .set_text(&format!("Tema: {active_label}  ▾"));
    ui.previous_theme_button
        .set_sensitive(ui.previous_theme.borrow().is_some());

    if let Some(previous) = ui.previous_theme.borrow().clone() {
        let label = packages
            .iter()
            .find(|package| package.manifest.name == previous)
            .map(|package| package.manifest.display_name.clone())
            .unwrap_or_else(|| previous.clone());
        let button = Button::new();
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let icon = Label::new(Some("↶"));
        row.append(&icon);
        let name = Label::new(Some(&format!("Volver a «{label}»")));
        name.set_halign(gtk::Align::Start);
        name.set_hexpand(true);
        row.append(&name);
        button.set_child(Some(&row));
        let ui_for_previous = Rc::clone(ui);
        button.connect_clicked(move |_| {
            apply_theme_with_guard(&ui_for_previous, &previous);
        });
        ui.quick_theme_menu.append(&button);
        ui.quick_theme_menu.append(&gtk::Separator::new(Orientation::Horizontal));
    }

    for package in packages {
        let button = Button::new();
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");
        button.set_tooltip_text(Some(&package.manifest.description));
        let row = GtkBox::new(Orientation::Horizontal, 8);
        let dots = theme_palette_dots(&package.manifest.palette);
        row.append(&dots);
        let name = Label::new(Some(&package.manifest.display_name));
        name.set_halign(gtk::Align::Start);
        name.set_hexpand(true);
        row.append(&name);
        if active.as_deref() == Some(package.manifest.name.as_str()) {
            let active_label = Label::new(Some("Activo"));
            active_label.add_css_class("success");
            row.append(&active_label);
        }
        button.set_child(Some(&row));
        let ui_for_apply = Rc::clone(ui);
        let name = package.manifest.name.clone();
        button.connect_clicked(move |_| {
            apply_theme_with_guard(&ui_for_apply, &name);
        });
        ui.quick_theme_menu.append(&button);
    }
}

fn theme_palette_dots(palette_id: &str) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 2);
    let colors = if palette_id == palette::DYNAMIC_ID {
        let theme = aspire_core::colors::load_theme_colors();
        [
            css_color(theme.background, "#1e1e2e"),
            css_color(theme.selected, "#cba6f7"),
            css_color(theme.active, "#89b4fa"),
        ]
    } else if let Ok(palette) = palette::find_by_id(palette_id) {
        [
            palette.colors["background"].clone(),
            palette.colors["selected"].clone(),
            palette.colors["active"].clone(),
        ]
    } else {
        [
            "#666666".to_owned(),
            "#888888".to_owned(),
            "#aaaaaa".to_owned(),
        ]
    };
    for color in colors {
        let dot = Label::new(Some("●"));
        dot.set_markup(&format!("<span foreground=\"{color}\">●</span>"));
        row.append(&dot);
    }
    row
}

fn connect_inspector_actions(
    ui: &Rc<EditorUi>,
    notebook: &Notebook,
    generate: &Button,
    open_files: &Button,
    open_scripts: &Button,
    open_themes: &Button,
) {
    let ui_for_generate = Rc::clone(ui);
    generate.connect_clicked(move |_| {
        let document = ui_for_generate.state.borrow().document.clone();
        match apply::save_project(&document) {
            Ok(()) => {
                *ui_for_generate.saved_document.borrow_mut() = document;
                notify_app("Guardado", "Tema aplicado y guardado.");
                set_status(
                    &ui_for_generate,
                    "Config y estilo generados en la configuración.",
                );
            }
            Err(error) => set_status(
                &ui_for_generate,
                &format!("No se pudo generar la configuración: {error:#}"),
            ),
        }
    });
    let notebook_for_files = notebook.clone();
    open_files.connect_clicked(move |_| notebook_for_files.set_current_page(Some(5)));
    let notebook_for_scripts = notebook.clone();
    open_scripts.connect_clicked(move |_| notebook_for_scripts.set_current_page(Some(4)));
    let notebook_for_themes = notebook.clone();
    open_themes.connect_clicked(move |_| notebook_for_themes.set_current_page(Some(2)));
}

fn connect_file_page_controls(ui: &Rc<EditorUi>) {
    let ui_project_config = Rc::clone(ui);
    ui.files.open_project_config.connect_clicked(move |_| {
        let result = apply::project_paths().and_then(|(config, _)| open_path(&config));
        report_open_result(&ui_project_config, result, "config.jsonc de la configuración");
    });
    let ui_project_style = Rc::clone(ui);
    ui.files.open_project_style.connect_clicked(move |_| {
        let result = apply::project_paths().and_then(|(_, style)| open_path(&style));
        report_open_result(&ui_project_style, result, "style.css de la configuración");
    });
    let ui_live_config = Rc::clone(ui);
    ui.files.open_live_config.connect_clicked(move |_| {
        let result = apply::live_paths().and_then(|(config, _)| open_path(&config));
        report_open_result(&ui_live_config, result, "config.jsonc de Waybar");
    });
    let ui_live_style = Rc::clone(ui);
    ui.files.open_live_style.connect_clicked(move |_| {
        let result = apply::live_paths().and_then(|(_, style)| open_path(&style));
        report_open_result(&ui_live_style, result, "style.css de Waybar");
    });
    let ui_folder = Rc::clone(ui);
    ui.files.open_folder.connect_clicked(move |_| {
        let result = crate::aspire_bar_dir().and_then(|root| open_path(&root));
        report_open_result(&ui_folder, result, "carpeta de configuración");
    });
}

fn connect_about_controls(ui: &Rc<EditorUi>, notebook: &Notebook) {
    build_diagnostic_text(&ui.about.diagnostic);
    let ui_for_page = Rc::clone(ui);
    notebook.connect_switch_page(move |_, _, page| {
        if page == 6 {
            build_diagnostic_text(&ui_for_page.about.diagnostic);
        }
    });
    let ui_for_refresh = Rc::clone(ui);
    let diagnostic = ui.about.diagnostic.clone();
    ui.about.copy_diagnostic.connect_clicked(move |_| {
        let text = build_diagnostic();
        diagnostic.buffer().set_text(&text);
        if let Some(display) = gtk::gdk::Display::default() {
            display.clipboard().set_text(&text);
            set_status(&ui_for_refresh, "Diagnóstico copiado al portapapeles.");
        } else {
            set_status(&ui_for_refresh, "No hay portapapeles GTK disponible.");
        }
    });
    let ui_for_open = Rc::clone(ui);
    ui.about.open_config.connect_clicked(move |_| {
        let result = crate::aspire_bar_dir().and_then(|root| open_path(&root));
        report_open_result(&ui_for_open, result, "carpeta de configuración");
    });
}

fn notify_app(title: &str, body: &str) {
    let _ = std::process::Command::new("notify-send")
        .args(["-a", "aspire-bar", "-t", title, "-i", "applications-system-symbolic", body])
        .output();
}

fn build_diagnostic() -> String {
    let config_dir = dirs::config_dir();
    let aspire = crate::aspire_bar_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|error| format!("no disponible: {error:#}"));
    let waybar = crate::waybar_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|error| format!("no disponible: {error:#}"));
    let current = match config_dir
        .as_ref()
        .map(|path| path.join("theme-engine/current"))
    {
        None => "no se pudo determinar XDG_CONFIG_HOME".to_owned(),
        Some(theme_root) => match std::fs::symlink_metadata(&theme_root) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                match std::fs::read_link(&theme_root) {
                    Ok(target) => format!("symlink -> {}", target.display()),
                    Err(error) => format!("symlink no legible: {error}"),
                }
            }
            Ok(_) => "existe, pero no es un symlink".to_owned(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => "no existe".to_owned(),
            Err(error) => format!("error: {error}"),
        },
    };
    let active = match themes::active_theme() {
        Ok(Some(name)) => name,
        Ok(None) => "Dinámico (wallpaper)".to_owned(),
        Err(error) => format!("error al leer el tema activo: {error:#}"),
    };
    format!(
        "aspire-bar {}\nCommit: {}\nWaybar: {}\nMangoWM: {}\nGTK4: {}\n\
         Config aspire-bar: {}\nConfig Waybar: {}\nTema activo: {}\n\
         theme-engine/current: {}\n",
        env!("CARGO_PKG_VERSION"),
        build_commit(),
        command_version("waybar", &["--version"]),
        command_version("mango", &["-v"]),
        command_version("pkg-config", &["--modversion", "gtk4"]),
        aspire,
        waybar,
        active,
        current,
    )
}

fn build_commit() -> &'static str {
    option_env!("ASPIRE_BAR_GIT_COMMIT")
        .or(option_env!("GIT_COMMIT"))
        .unwrap_or("no disponible")
}

fn command_version(command: &str, args: &[&str]) -> String {
    match std::process::Command::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            format!("no disponible ({})", stderr.trim())
        }
        Err(error) => format!("no disponible ({error})"),
    }
}

fn open_path(path: &std::path::Path) -> Result<()> {
    std::process::Command::new("xdg-open")
        .arg(path)
        .spawn()
        .with_context(|| format!("no se pudo abrir {}", path.display()))?;
    Ok(())
}

fn report_open_result(ui: &Rc<EditorUi>, result: Result<()>, label: &str) {
    match result {
        Ok(()) => set_status(ui, &format!("Abriendo {label}.")),
        Err(error) => set_status(ui, &format!("No se pudo abrir {label}: {error:#}")),
    }
}

fn build_diagnostic_text(view: &TextView) {
    view.buffer().set_text(&build_diagnostic());
}

fn connect_template_controls(ui: &Rc<EditorUi>, window: &ApplicationWindow) {
    let ui_for_selection = Rc::clone(ui);
    let parent = window.clone();
    ui.templates.list.connect_row_selected(move |_, row| {
        let Some(id) = row.and_then(|row| row.tooltip_text().map(|id| id.to_string())) else {
            return;
        };
        *ui_for_selection.templates.selected.borrow_mut() = Some(id.clone());
        match templates::find(&id) {
            Ok(template) => apply_template_with_confirmation(&ui_for_selection, &parent, template),
            Err(error) => set_status(
                &ui_for_selection,
                &format!("No se pudo cargar la plantilla «{id}»: {error:#}"),
            ),
        }
    });
}

fn connect_script_controls(ui: &Rc<EditorUi>) {
    let ui_for_add = Rc::clone(ui);
    ui.scripts.add.connect_clicked(move |_| {
        let name = ui_for_add.scripts.name.text().trim().to_owned();
        let source = text_view_text(&ui_for_add.scripts.source);
        if name.is_empty() {
            set_status(&ui_for_add, "Escribe un nombre para el script.");
            return;
        }
        if source.trim().is_empty() {
            set_status(&ui_for_add, "El contenido del script no puede estar vacío.");
            return;
        }
        let path = match scripts::save(&name, &source) {
            Ok(path) => path,
            Err(error) => {
                set_status(&ui_for_add, &format!("No se pudo guardar el script: {error:#}"));
                return;
            }
        };
        let Some(module_name) = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| format!("custom/{stem}"))
        else {
            set_status(&ui_for_add, "No se pudo obtener el nombre del módulo guardado.");
            return;
        };
        let zone = ui_for_add.scripts.zone.selected() as usize;
        match place_module_in_zone(&ui_for_add, &module_name, zone) {
            Ok(()) => {
                update_module_field(
                    &ui_for_add,
                    &module_name,
                    "exec",
                    Value::String(format!("bash {}", scripts::shell_quote(&path))),
                );
                update_module_field(
                    &ui_for_add,
                    &module_name,
                    "interval",
                    json!(ui_for_add.scripts.interval.value_as_int()),
                );
                update_module_field(
                    &ui_for_add,
                    &module_name,
                    "format",
                    Value::String("{}".to_owned()),
                );
                if ui_for_add.scripts.return_type.selected() == 1 {
                    update_module_field(
                        &ui_for_add,
                        &module_name,
                        "return-type",
                        Value::String("json".to_owned()),
                    );
                } else {
                    remove_module_field(&ui_for_add, &module_name, "return-type");
                }
                refresh_zones(&ui_for_add);
                refresh_style_module_choices(&ui_for_add);
                refresh_form(&ui_for_add);
                set_status(
                    &ui_for_add,
                    &format!(
                        "Script guardado y módulo «{module_name}» añadido. Revisa antes de aplicar y guarda la configuración."
                    ),
                );
            }
            Err(error) => set_status(
                &ui_for_add,
                &format!(
                    "El script quedó guardado en {}, pero no se pudo añadir el módulo: {error:#}",
                    path.display()
                ),
            ),
        }
    });
}

fn connect_theme_folder_import(ui: &Rc<EditorUi>, window: &ApplicationWindow, button: &Button) {
    let ui_for_click = Rc::clone(ui);
    let parent = window.clone();
    button.connect_clicked(move |_| {
        let chooser = FileDialog::builder()
            .title("Importar tema desde carpeta")
            .accept_label("Importar")
            .build();
        let ui_for_response = Rc::clone(&ui_for_click);
        chooser.select_folder(
            Some(&parent),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                import_theme_result(&ui_for_response, result);
            },
        );
    });
}

fn connect_theme_archive_import(ui: &Rc<EditorUi>, window: &ApplicationWindow, button: &Button) {
    let ui_for_click = Rc::clone(ui);
    let parent = window.clone();
    button.connect_clicked(move |_| {
        let chooser = FileDialog::builder()
            .title("Importar paquete .tar.zst")
            .accept_label("Importar")
            .build();
        let ui_for_response = Rc::clone(&ui_for_click);
        chooser.open(
            Some(&parent),
            None::<&gtk::gio::Cancellable>,
            move |result| {
                import_theme_result(&ui_for_response, result);
            },
        );
    });
}

fn import_theme_result(ui: &Rc<EditorUi>, result: Result<gtk::gio::File, gtk::glib::Error>) {
    match file_dialog_path(result) {
        Ok(path) => match themes::import_theme(&path) {
            Ok(imported) => {
                refresh_theme_gallery(ui);
                set_status(ui, &format!("Tema importado en {}.", imported.display()));
            }
            Err(error) => set_status(ui, &format!("No se pudo importar: {error:#}")),
        },
        Err(error) => set_status(
            ui,
            &format!("No se pudo elegir el tema para importar: {error}"),
        ),
    }
}

fn file_dialog_path(
    result: Result<gtk::gio::File, gtk::glib::Error>,
) -> Result<std::path::PathBuf> {
    result?
        .path()
        .context("el selector no proporcionó una ruta local")
}

fn export_theme_result(ui: &Rc<EditorUi>, name: &str, destination: &std::path::Path) {
    match themes::export_theme(name, destination) {
        Ok(exported) => set_status(ui, &format!("Tema exportado a {}.", exported.display())),
        Err(error) => set_status(ui, &format!("No se pudo exportar: {error:#}")),
    }
}

fn prompt_text(
    parent: &ApplicationWindow,
    title: &str,
    placeholder: &str,
    initial: &str,
    on_submit: impl Fn(String) + 'static,
) {
    let dialog = gtk::Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(420)
        .build();
    let header = gtk::HeaderBar::new();
    header.add_css_class("flat");
    dialog.set_titlebar(Some(&header));
    let content = GtkBox::new(Orientation::Vertical, 10);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    let entry = Entry::new();
    entry.set_placeholder_text(Some(placeholder));
    entry.set_text(initial);
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let cancel = Button::with_label("Cancelar");
    let accept = Button::with_label("Aceptar");
    accept.add_css_class("suggested-action");
    let dialog_for_cancel = dialog.clone();
    cancel.connect_clicked(move |_| dialog_for_cancel.close());
    let dialog_for_accept = dialog.clone();
    let entry_for_accept = entry.clone();
    accept.connect_clicked(move |_| {
        on_submit(entry_for_accept.text().trim().to_owned());
        dialog_for_accept.close();
    });
    actions.append(&cancel);
    actions.append(&accept);
    content.append(&entry);
    content.append(&actions);
    dialog.set_child(Some(&content));
    dialog.present();
}

fn gallery_manifest(ui: &Rc<EditorUi>, slug: &str, display_name: &str) -> Result<ThemeManifest> {
    if display_name.trim().is_empty() {
        bail!("escribe un nombre para el tema");
    }
    let state = ui.state.borrow();
    let mut required_modules = state
        .document
        .config
        .bars()
        .iter()
        .flat_map(|bar| bar.zones())
        .flatten()
        .collect::<Vec<_>>();
    required_modules.sort();
    required_modules.dedup();
    let palette = state
        .document
        .style_settings
        .palette_id
        .clone()
        .filter(|id| {
            state.document.style_settings.use_aspire_palette == Some(false)
                || id == palette::DYNAMIC_ID
        })
        .unwrap_or_else(|| palette::DYNAMIC_ID.to_owned());
    drop(state);
    let allow_fixed_colors = palette != palette::DYNAMIC_ID;
    Ok(ThemeManifest {
        name: slug.to_owned(),
        display_name: display_name.trim().to_owned(),
        author: "Personal".to_owned(),
        description: format!("Tema «{}» creado con aspire-bar.", display_name.trim()),
        version: "1.0.0".to_owned(),
        palette,
        required_modules,
        optional_dependencies: Vec::new(),
        allow_fixed_colors,
    })
}

fn apply_gallery_theme(ui: &Rc<EditorUi>, name: &str) -> Result<()> {
    if ui.pending_test.borrow().is_some() {
        bail!("hay una prueba activa; confírmala o reviértela primero");
    }
    let previous = themes::active_theme()?;
    let package = themes::find_package(name)?;
    let mut document = package.document;
    document.use_dynamic_palette_when_unspecified();
    apply::apply_document(&document, name)?;
    themes::set_active_theme(name)?;
    apply::save_project(&document)?;
    *ui.previous_theme.borrow_mut() = previous.filter(|previous| previous != name);
    ui.previous_theme_button
        .set_sensitive(ui.previous_theme.borrow().is_some());
    *ui.state.borrow_mut() = EditorState {
        selection: first_module_selection(&document, 0),
        document: document.clone(),
        bar_index: 0,
    };
    *ui.saved_document.borrow_mut() = document;
    refresh_bar_dropdown(ui);
    refresh_zones(ui);
    refresh_style_module_choices(ui);
    refresh_style_controls(ui, true);
    show_empty_form(ui);
    refresh_preview(ui);
    refresh_theme_gallery(ui);
    refresh_quick_theme_menu(ui);
    Ok(())
}

fn refresh_theme_gallery(ui: &Rc<EditorUi>) {
    let _ = themes::ensure_readme();
    let selected = ui.themes.selected.borrow().clone();
    ui.themes.duplicate.set_sensitive(false);
    ui.themes.rename.set_sensitive(false);
    ui.themes.delete.set_sensitive(false);
    clear_list(&ui.themes.list);
    let (packages, invalid) = match themes::list_all() {
        Ok(result) => result,
        Err(error) => {
            set_status(ui, &format!("No se pudo cargar la galería: {error:#}"));
            return;
        }
    };
    let active = match themes::active_theme() {
        Ok(active) => active,
        Err(error) => {
            set_status(ui, &format!("No se pudo leer el tema activo: {error:#}"));
            None
        }
    };
    let active_display = active
        .as_deref()
        .and_then(|name| {
            packages
                .iter()
                .find(|package| package.manifest.name == name)
        })
        .map(|package| package.manifest.display_name.clone())
        .unwrap_or_else(|| "colores dinámicos del wallpaper".to_owned());
    ui.themes
        .active_label
        .set_text(&format!("Activo: {active_display}"));

    let hidden = themes::hidden_themes();
    let mine = packages
        .iter()
        .filter(|package| !package.is_default)
        .collect::<Vec<_>>();
    let defaults = packages
        .iter()
        .filter(|package| package.is_default && !hidden.iter().any(|h| h == &package.manifest.name))
        .collect::<Vec<_>>();
    let hidden_packages = packages
        .iter()
        .filter(|package| package.is_default && hidden.iter().any(|h| h == &package.manifest.name))
        .collect::<Vec<_>>();

    append_theme_section(&ui.themes.list, "Mis temas");
    if mine.is_empty() {
        append_theme_hint(
            &ui.themes.list,
            "Aún no tienes temas propios. Usa «＋ Crear», «Importar…» o «Guardar mi Waybar actual».",
        );
    }
    for package in mine {
        append_theme_card(ui, package, active.as_deref(), false);
    }

    append_theme_section(&ui.themes.list, "Predeterminados");
    for package in defaults {
        append_theme_card(ui, package, active.as_deref(), false);
    }

    if !hidden_packages.is_empty() {
        append_theme_section(&ui.themes.list, "Ocultos");
        for package in hidden_packages {
            append_theme_card(ui, package, active.as_deref(), true);
        }
    }

    if !invalid.is_empty() {
        append_theme_section(&ui.themes.list, "No válidos");
        for entry in &invalid {
            append_invalid_theme(&ui.themes.list, &entry.slug, &entry.reason);
        }
    }

    ui.themes
        .restore_defaults
        .set_sensitive(!hidden.is_empty());

    let selected_name = selected.or(active);
    if let Some(name) = selected_name {
        let mut child = ui.themes.list.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if let Ok(row) = widget.downcast::<ListBoxRow>() {
                if row.tooltip_text().as_deref() == Some(name.as_str()) {
                    ui.themes.list.select_row(Some(&row));
                    break;
                }
            }
        }
    }
    refresh_quick_theme_menu(ui);
}

fn append_theme_section(list: &ListBox, title: &str) {
    let row = ListBoxRow::new();
    row.set_selectable(false);
    row.set_activatable(false);
    let label = Label::new(Some(title));
    label.add_css_class("heading");
    label.set_halign(gtk::Align::Start);
    label.set_margin_top(10);
    label.set_margin_bottom(2);
    label.set_margin_start(4);
    row.set_child(Some(&label));
    list.append(&row);
}

fn append_theme_hint(list: &ListBox, text: &str) {
    let row = ListBoxRow::new();
    row.set_selectable(false);
    row.set_activatable(false);
    let label = Label::new(Some(text));
    label.set_wrap(true);
    label.set_halign(gtk::Align::Start);
    label.add_css_class("dim-label");
    label.set_margin_start(8);
    label.set_margin_end(8);
    label.set_margin_top(6);
    label.set_margin_bottom(6);
    row.set_child(Some(&label));
    list.append(&row);
}

fn append_invalid_theme(list: &ListBox, slug: &str, reason: &str) {
    let row = ListBoxRow::new();
    row.set_selectable(false);
    row.set_activatable(false);
    let card = GtkBox::new(Orientation::Vertical, 2);
    card.set_margin_top(6);
    card.set_margin_bottom(6);
    card.set_margin_start(10);
    card.set_margin_end(10);
    let title = Label::new(Some(slug));
    title.add_css_class("heading");
    title.set_halign(gtk::Align::Start);
    card.append(&title);
    let detail = Label::new(Some(reason));
    detail.set_wrap(true);
    detail.set_halign(gtk::Align::Start);
    detail.add_css_class("dim-label");
    card.append(&detail);
    row.set_child(Some(&card));
    list.append(&row);
}

fn append_theme_card(
    ui: &Rc<EditorUi>,
    package: &themes::ThemePackage,
    active: Option<&str>,
    hidden: bool,
) {
    let row = ListBoxRow::new();
    let card = GtkBox::new(Orientation::Vertical, 4);
    card.set_margin_top(6);
    card.set_margin_bottom(6);
    card.set_margin_start(8);
    card.set_margin_end(8);
    let title_row = GtkBox::new(Orientation::Horizontal, 8);
    let title = Label::new(Some(&package.manifest.display_name));
    title.add_css_class("heading");
    title.set_halign(gtk::Align::Start);
    title.set_hexpand(true);
    title_row.append(&title);
    if active == Some(package.manifest.name.as_str()) {
        let badge = Label::new(Some("ACTIVO"));
        badge.add_css_class("success");
        title_row.append(&badge);
    }
    card.append(&title_row);

    let palette_name = if package.manifest.palette == palette::DYNAMIC_ID {
        "Dinámica · wallpaper".to_owned()
    } else {
        palette::find_by_id(&package.manifest.palette)
            .map(|palette| palette.name)
            .unwrap_or_else(|_| package.manifest.palette.clone())
    };
    let meta_row = GtkBox::new(Orientation::Horizontal, 6);
    meta_row.append(&palette_swatch_row(&package.manifest.palette));
    let palette_label = Label::new(Some(&palette_name));
    palette_label.add_css_class("dim-label");
    palette_label.set_halign(gtk::Align::Start);
    meta_row.append(&palette_label);
    card.append(&meta_row);

    let colors = palette_color_values(&package.manifest.palette);
    let background = colors.first().map(String::as_str).unwrap_or("#1e1e2e");
    let surface = colors.get(1).map(String::as_str).unwrap_or("#313244");
    let foreground = colors.get(2).map(String::as_str).unwrap_or("#cdd6f4");
    let accent = colors.get(3).map(String::as_str).unwrap_or("#cba6f7");
    let urgent = colors.get(4).map(String::as_str).unwrap_or("#f38ba8");
    let mini_bar = Label::new(None);
    mini_bar.set_markup(&format!(
        "<span background=\"{background}\" foreground=\"{foreground}\">  ● módulos  </span> <span background=\"{surface}\" foreground=\"{foreground}\">  ● centro  </span> <span background=\"{accent}\" foreground=\"{foreground}\">  ● estado  </span> <span foreground=\"{urgent}\"> ● </span>"
    ));
    mini_bar.add_css_class("preview-bar-mini");
    mini_bar.set_halign(gtk::Align::Start);
    card.append(&mini_bar);

    let settings = &package.document.style_settings;
    let radius = settings.border_radius.unwrap_or(10);
    let gap = package
        .document
        .config
        .bars()
        .first()
        .and_then(|bar| bar.spacing.as_ref())
        .and_then(Value::as_u64)
        .unwrap_or(4);
    let padding = settings.module_padding_horizontal.unwrap_or(10);
    let metrics = Label::new(Some(&format!(
        "{radius}px radio · {gap}px gap · 0 {padding}px pad"
    )));
    metrics.add_css_class("dim-label");
    metrics.set_halign(gtk::Align::Start);
    card.append(&metrics);

    let actions = GtkBox::new(Orientation::Horizontal, 4);
    actions.set_margin_top(2);
    let use_button = Button::with_label(if hidden { "Mostrar y usar" } else { "Usar" });
    use_button.add_css_class("flat");
    let duplicate_button = Button::with_label("Duplicar");
    duplicate_button.add_css_class("flat");
    let rename_button = Button::with_label("Renombrar");
    rename_button.add_css_class("flat");
    let export_button = Button::with_label("Exportar");
    export_button.add_css_class("flat");
    actions.append(&use_button);
    actions.append(&duplicate_button);
    if !package.is_default {
        actions.append(&rename_button);
        actions.append(&export_button);
    }
    let danger = if package.is_default {
        Button::with_label(if hidden { "Restaurar" } else { "Ocultar" })
    } else {
        Button::with_label("Eliminar")
    };
    danger.add_css_class("flat");
    danger.add_css_class("destructive-action");
    actions.append(&danger);
    card.append(&actions);

    row.set_tooltip_text(Some(&package.manifest.name));
    row.set_child(Some(&card));
    let preview = Rc::clone(&ui.preview);
    let document = package.document.clone();
    let motion = gtk::EventControllerMotion::new();
    motion.connect_enter(move |_, _, _| preview.render(&document));
    let ui_for_leave = Rc::clone(ui);
    motion.connect_leave(move |_| refresh_preview(&ui_for_leave));
    row.add_controller(motion);
    ui.themes.list.append(&row);

    let name = package.manifest.name.clone();
    let is_default = package.is_default;
    let ui_for_use = Rc::clone(ui);
    let name_for_use = name.clone();
    use_button.connect_clicked(move |_| {
        if hidden {
            let _ = themes::set_hidden(&name_for_use, false);
        }
        apply_theme_with_guard(&ui_for_use, &name_for_use);
    });

    let ui_for_duplicate = Rc::clone(ui);
    let name_for_duplicate = name.clone();
    duplicate_button.connect_clicked(move |_| {
        let ui_for_submit = Rc::clone(&ui_for_duplicate);
        let source = name_for_duplicate.clone();
        prompt_text(
            &ui_for_duplicate.window,
            "Duplicar tema",
            "Nombre de la copia",
            "",
            move |display_name| {
                let result = themes::slugify(&display_name).and_then(|slug| {
                    themes::duplicate_theme(&source, &slug)?;
                    themes::rename_theme(&slug, &slug, &display_name)
                });
                match result {
                    Ok(path) => {
                        refresh_theme_gallery(&ui_for_submit);
                        notify_app("Tema duplicado", &display_name);
                        set_status(&ui_for_submit, &format!("Copia creada en {}.", path.display()));
                    }
                    Err(error) => set_status(
                        &ui_for_submit,
                        &format!("No se pudo duplicar el tema: {error:#}"),
                    ),
                }
            },
        );
    });

    if !is_default {
        let ui_for_rename = Rc::clone(ui);
        let name_for_rename = name.clone();
        let initial = package.manifest.display_name.clone();
        rename_button.connect_clicked(move |_| {
            let ui_for_submit = Rc::clone(&ui_for_rename);
            let source = name_for_rename.clone();
            prompt_text(
                &ui_for_rename.window,
                "Renombrar tema",
                "Nombre visible",
                &initial,
                move |display_name| {
                    let result = themes::slugify(&display_name)
                        .and_then(|slug| themes::rename_theme(&source, &slug, &display_name));
                    match result {
                        Ok(path) => {
                            *ui_for_submit.themes.selected.borrow_mut() = path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .map(str::to_owned);
                            refresh_theme_gallery(&ui_for_submit);
                            refresh_quick_theme_menu(&ui_for_submit);
                            set_status(&ui_for_submit, "Tema renombrado.");
                        }
                        Err(error) => set_status(
                            &ui_for_submit,
                            &format!("No se pudo renombrar el tema: {error:#}"),
                        ),
                    }
                },
            );
        });

        let ui_for_export = Rc::clone(ui);
        let name_for_export = name.clone();
        export_button.connect_clicked(move |_| {
            let chooser = FileDialog::builder()
                .title("Exportar tema a una carpeta")
                .accept_label("Exportar")
                .build();
            let ui_for_response = Rc::clone(&ui_for_export);
            let export_name = name_for_export.clone();
            chooser.select_folder(
                Some(&ui_for_export.window),
                None::<&gtk::gio::Cancellable>,
                move |result| match file_dialog_path(result) {
                    Ok(path) => export_theme_result(&ui_for_response, &export_name, &path),
                    Err(error) => set_status(
                        &ui_for_response,
                        &format!("No se pudo elegir la carpeta destino: {error}"),
                    ),
                },
            );
        });

        let ui_for_delete = Rc::clone(ui);
        let name_for_delete = name.clone();
        danger.connect_clicked(move |_| {
            delete_theme_with_confirm(&ui_for_delete, &name_for_delete);
        });
    } else {
        let ui_for_hide = Rc::clone(ui);
        let name_for_hide = name.clone();
        danger.connect_clicked(move |_| {
            match themes::set_hidden(&name_for_hide, !hidden) {
                Ok(()) => {
                    refresh_theme_gallery(&ui_for_hide);
                    set_status(
                        &ui_for_hide,
                        if hidden {
                            "Tema predeterminado restaurado."
                        } else {
                            "Tema predeterminado oculto."
                        },
                    );
                }
                Err(error) => set_status(
                    &ui_for_hide,
                    &format!("No se pudo cambiar la visibilidad: {error:#}"),
                ),
            }
        });
    }
}

fn apply_theme_with_guard(ui: &Rc<EditorUi>, name: &str) {
    if !has_unsaved_changes(ui) {
        report_theme_apply(ui, name);
        return;
    }
    let dialog = gtk::AlertDialog::builder()
        .message("Cambios sin guardar")
        .detail(format!(
            "El tema «{name}» se aplicará a Waybar y se descartarán los cambios actuales."
        ))
        .buttons(["Cancelar", "Descartar y cambiar", "Guardar"])
        .cancel_button(0)
        .default_button(1)
        .build();
    let ui_for_choice = Rc::clone(ui);
    let name = name.to_owned();
    dialog.choose(
        Some(&ui.window),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(1) => report_theme_apply(&ui_for_choice, &name),
            Ok(2) => {
                let window = ui_for_choice.window.clone();
                save_and_apply(&ui_for_choice, &window);
            }
            _ => {}
        },
    );
}

fn report_theme_apply(ui: &Rc<EditorUi>, name: &str) {
    match apply_gallery_theme(ui, name) {
        Ok(()) => {
            let label = themes::find_package(name)
                .map(|package| package.manifest.display_name)
                .unwrap_or_else(|_| name.to_owned());
            notify_app("Tema aplicado", &label);
            set_status(ui, &format!("Tema «{label}» aplicado."));
        }
        Err(error) => set_status(ui, &format!("No se pudo aplicar «{name}»: {error:#}")),
    }
}

fn delete_theme_with_confirm(ui: &Rc<EditorUi>, name: &str) {
    let package = match themes::find_package(name) {
        Ok(package) => package,
        Err(error) => {
            set_status(ui, &format!("No se pudo leer el tema: {error:#}"));
            return;
        }
    };
    if package.is_default {
        set_status(ui, "Los temas predeterminados solo se pueden ocultar.");
        return;
    }
    let active = themes::active_theme().ok().flatten().as_deref() == Some(name);
    let detail = if active {
        format!(
            "«{}» está activo. Elige un tema de reemplazo para poder eliminarlo.",
            package.manifest.display_name
        )
    } else {
        format!(
            "«{}» se moverá a la papelera del sistema.",
            package.manifest.display_name
        )
    };
    let (buttons, cancel, default) = if active {
        (vec!["Cancelar", "Cambiar y eliminar"], 0, 1)
    } else {
        (vec!["Cancelar", "Eliminar"], 0, 1)
    };
    let dialog = gtk::AlertDialog::builder()
        .message("Eliminar tema")
        .detail(detail)
        .buttons(buttons)
        .cancel_button(cancel)
        .default_button(default)
        .build();
    let ui_for_choice = Rc::clone(ui);
    let name = name.to_owned();
    dialog.choose(
        Some(&ui.window),
        None::<&gtk::gio::Cancellable>,
        move |result| {
            if !matches!(result, Ok(1)) {
                return;
            }
            if active {
                let replacement = themes::list_all()
                    .ok()
                    .map(|(packages, _)| {
                        packages
                            .into_iter()
                            .filter(|package| package.manifest.name != name)
                            .find(|package| !package.is_default)
                            .map(|package| package.manifest.name)
                    })
                    .flatten()
                    .or_else(|| {
                        themes::default_theme_names()
                            .ok()
                            .and_then(|names| names.into_iter().next())
                    });
                let Some(replacement) = replacement else {
                    set_status(&ui_for_choice, "No hay otro tema para cambiar antes de eliminar.");
                    return;
                };
                if let Err(error) = apply_gallery_theme(&ui_for_choice, &replacement) {
                    set_status(
                        &ui_for_choice,
                        &format!("No se pudo cambiar de tema: {error:#}"),
                    );
                    return;
                }
            }
            match themes::trash_theme(&name) {
                Ok(()) => {
                    *ui_for_choice.themes.selected.borrow_mut() = None;
                    refresh_theme_gallery(&ui_for_choice);
                    refresh_quick_theme_menu(&ui_for_choice);
                    notify_app("Tema eliminado", &name);
                    set_status(&ui_for_choice, "Tema movido a la papelera.");
                }
                Err(error) => set_status(
                    &ui_for_choice,
                    &format!("No se pudo eliminar el tema: {error:#}"),
                ),
            }
        },
    );
}

fn refresh_theme_palette_grid(ui: &Rc<EditorUi>, theme_name: &str, is_default: bool) {
    while let Some(child) = ui.themes.palette_grid.first_child() {
        ui.themes.palette_grid.remove(&child);
    }
    let package = match themes::find_package(theme_name) {
        Ok(package) => package,
        Err(error) => {
            set_status(
                ui,
                &format!("No se pudo cargar el selector de paleta: {error:#}"),
            );
            return;
        }
    };
    let mut options = vec![(
        palette::DYNAMIC_ID.to_owned(),
        "Dinámica · wallpaper".to_owned(),
    )];
    match palette::list() {
        Ok(palettes) => options.extend(
            palettes
                .into_iter()
                .map(|palette| (palette.id, palette.name)),
        ),
        Err(error) => {
            set_status(ui, &format!("No se pudieron leer las paletas: {error:#}"));
            return;
        }
    }
    let mut row: Option<GtkBox> = None;
    for (index, (id, name)) in options.into_iter().enumerate() {
        if index % 2 == 0 {
            let line = GtkBox::new(Orientation::Horizontal, 8);
            ui.themes.palette_grid.append(&line);
            row = Some(line);
        }
        let button = Button::new();
        button.set_hexpand(true);
        button.set_sensitive(!is_default);
        if package.manifest.palette == id {
            button.add_css_class("suggested-action");
        }
        let card = GtkBox::new(Orientation::Vertical, 4);
        card.set_margin_top(5);
        card.set_margin_bottom(5);
        card.set_margin_start(7);
        card.set_margin_end(7);
        let name_label = Label::new(Some(&name));
        name_label.set_halign(gtk::Align::Start);
        name_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        card.append(&name_label);
        card.append(&palette_swatch_row(&id));
        button.set_child(Some(&card));
        let ui_for_palette = Rc::clone(ui);
        let theme_name = theme_name.to_owned();
        let palette_id = id;
        button.connect_clicked(move |_| {
            match themes::set_theme_palette(&theme_name, &palette_id) {
                Ok(()) => {
                    match themes::find_package(&theme_name) {
                        Ok(updated) => {
                            ui_for_palette.preview.render(&updated.document);
                            let active = match themes::active_theme() {
                                Ok(active) => active.as_deref() == Some(theme_name.as_str()),
                                Err(error) => {
                                    set_status(
                                        &ui_for_palette,
                                        &format!("Paleta guardada, pero no se pudo leer el tema activo: {error:#}"),
                                    );
                                    return;
                                }
                            };
                            if active {
                                match apply::apply_document(&updated.document, &theme_name)
                            .and_then(|()| apply::save_project(&updated.document))
                        {
                            Ok(()) => {
                                *ui_for_palette.state.borrow_mut() = EditorState {
                                    document: updated.document.clone(),
                                    bar_index: 0,
                                    selection: first_module_selection(
                                        &updated.document,
                                        0,
                                    ),
                                };
                                *ui_for_palette.saved_document.borrow_mut() =
                                    updated.document.clone();
                                refresh_zones(&ui_for_palette);
                                refresh_style_controls(&ui_for_palette, true);
                                refresh_preview(&ui_for_palette);
                                set_status(
                                    &ui_for_palette,
                                    "Paleta activa actualizada y aplicada a Waybar.",
                                );
                            }
                            Err(error) => set_status(
                                &ui_for_palette,
                                &format!("La paleta se guardó, pero no se pudo aplicar: {error:#}"),
                            ),
                        }
                            } else {
                                set_status(
                                    &ui_for_palette,
                                    "Paleta del tema actualizada; usa el tema para aplicarla.",
                                );
                            }
                            refresh_theme_gallery(&ui_for_palette);
                            refresh_quick_theme_menu(&ui_for_palette);
                        }
                        Err(error) => set_status(
                            &ui_for_palette,
                            &format!("No se pudo recargar el tema: {error:#}"),
                        ),
                    }
                }
                Err(error) => set_status(
                    &ui_for_palette,
                    &format!("No se pudo cambiar la paleta: {error:#}"),
                ),
            }
        });
        if let Some(line) = &row {
            line.append(&button);
        }
    }
    if is_default {
        let note = Label::new(Some(
            "Predeterminado de solo lectura. Duplícalo para cambiar su paleta.",
        ));
        note.set_wrap(true);
        note.add_css_class("dim-label");
        ui.themes.palette_grid.append(&note);
    }
}

fn palette_swatch_row(id: &str) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 4);
    let colors = palette_color_values(id);
    for color in colors {
        let dot = Label::new(None);
        dot.set_markup(&format!("<span foreground=\"{color}\">●</span>"));
        row.append(&dot);
    }
    row
}

fn palette_color_values(id: &str) -> Vec<String> {
    if id == palette::DYNAMIC_ID {
        let colors = aspire_core::colors::load_theme_colors();
        return [
            colors.background,
            colors.background_alt,
            colors.foreground,
            colors.selected,
            colors.active,
        ]
        .into_iter()
        .flatten()
        .map(|color| format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b))
        .collect();
    }
    palette::find_by_id(id)
        .map(|palette| {
            [
                "background",
                "background-alt",
                "foreground",
                "selected",
                "urgent",
            ]
            .into_iter()
            .filter_map(|name| palette.colors.get(name).cloned())
            .collect()
        })
        .unwrap_or_default()
}

fn refresh_template_gallery(ui: &Rc<EditorUi>) {
    clear_list(&ui.templates.list);
    match templates::list() {
        Ok(templates) => {
            for template in templates {
                let row = ListBoxRow::new();
                let card = GtkBox::new(Orientation::Vertical, 6);
                card.set_margin_top(10);
                card.set_margin_bottom(10);
                card.set_margin_start(12);
                card.set_margin_end(12);
                let heading = GtkBox::new(Orientation::Horizontal, 8);
                let title = Label::new(Some(&template.name));
                title.add_css_class("heading");
                title.set_halign(gtk::Align::Start);
                title.set_hexpand(true);
                let category = Label::new(Some(&template.category.to_uppercase()));
                category.add_css_class("dim-label");
                heading.append(&title);
                heading.append(&category);
                card.append(&heading);
                let description = Label::new(Some(&template.description));
                description.set_wrap(true);
                description.set_halign(gtk::Align::Start);
                description.add_css_class("dim-label");
                card.append(&description);

                let bars = template.config.bars();
                let bar = bars.first();
                let modules = bar.map(|bar| bar.zones()).unwrap_or_default();
                let icons = GtkBox::new(Orientation::Horizontal, 10);
                for module in modules.iter().flatten() {
                    let icon = gtk::Image::from_icon_name(module_icon_name(module));
                    icon.set_pixel_size(16);
                    icon.set_tooltip_text(Some(module));
                    icons.append(&icon);
                }
                card.append(&icons);
                let facts = Label::new(Some(&format!(
                    "L {}  ·  C {}  ·  R {}  ·  {} · {} px",
                    modules[0].len(),
                    modules[1].len(),
                    modules[2].len(),
                    bar.and_then(|bar| bar.position.as_ref())
                        .and_then(Value::as_str)
                        .unwrap_or("top"),
                    bar.and_then(|bar| bar.height.as_ref())
                        .and_then(Value::as_u64)
                        .unwrap_or(36),
                )));
                facts.set_halign(gtk::Align::Start);
                facts.add_css_class("dim-label");
                card.append(&facts);
                row.set_tooltip_text(Some(&template.id));
                row.set_child(Some(&card));
                ui.templates.list.append(&row);
            }
        }
        Err(error) => set_status(
            ui,
            &format!("No se pudieron cargar las plantillas: {error:#}"),
        ),
    }
}

fn module_icon_name(module: &str) -> &'static str {
    let base = module.split('/').last().unwrap_or(module);
    match module {
        "ext/workspaces" | "hyprland/workspaces" | "sway/workspaces" | "niri/workspaces"
        | "river/tags" | "dwl/tags" => "view-grid-symbolic",
        "clock" => "alarm-symbolic",
        "battery" => "battery-good-symbolic",
        "wireplumber" | "pulseaudio" | "wireplumber/slider" | "pulseaudio/slider" => {
            "audio-volume-high-symbolic"
        }
        "network" => "network-wireless-symbolic",
        "cpu" | "memory" | "temperature" | "disk" | "load" => {
            "utilities-system-monitor-symbolic"
        }
        "custom/notification" => "preferences-system-notifications-symbolic",
        "backlight" => "display-brightness-symbolic",
        "bluetooth" => "bluetooth-active-symbolic",
        "capslock" | "numlock" | "scrolllock" | "keyboard-state" | "hyprland/keyboard-state"
        | "sway/keyboard-state" | "hyprland/language" | "niri/language" | "sway/language"
        | "river/layout" | "river/mode" | "sway/mode" | "hyprland/submap" | "sway/scratchpad" => {
            "input-keyboard-symbolic"
        }
        "tray" => "system-tray-symbolic",
        "idle_inhibitor" => "preferences-system-power-symbolic",
        "gamemode" => "applications-games-symbolic",
        "mpd" | "mpris" => "multimedia-player-symbolic",
        "image" => "image-x-generic-symbolic",
        "wlr/taskbar" => "view-app-grid-symbolic",
        "privacy" => "channel-insecure-symbolic",
        "power-profiles-daemon" => "power-profile-balanced-symbolic",
        "systemd-failed-units" => "dialog-error-symbolic",
        "hyprland/window" | "sway/window" | "river/window" | "niri/window" | "dwl/window"
        | "custom/window-title" => "window-symbolic",
        "custom/cava" => "audio-x-generic-symbolic",
        "custom/mango-submap" => "preferences-desktop-keyboard-shortcuts-symbolic",
        _ if base.starts_with("custom/") => "application-x-executable-symbolic",
        _ => "application-x-executable-symbolic",
    }
}

fn apply_template_with_confirmation(
    ui: &Rc<EditorUi>,
    _parent: &ApplicationWindow,
    template: templates::Template,
) {
    apply_template_to_editor(ui, template);
}

fn apply_template_to_editor(ui: &Rc<EditorUi>, template: templates::Template) {
    let mut state = ui.state.borrow_mut();
    let bar_index = state.bar_index;
    let result = templates::apply_layout(&mut state.document, &template, bar_index);
    if let Err(error) = result {
        drop(state);
        set_status(ui, &format!("No se pudo aplicar la plantilla: {error:#}"));
        return;
    }
    state.selection = first_module_selection(&state.document, bar_index);
    drop(state);
    refresh_zones(ui);
    refresh_style_module_choices(ui);
    refresh_preview(ui);
    refresh_form(ui);
    set_status(
        ui,
        &format!(
            "Plantilla «{}» aplicada a la configuración; el estilo se conservó.",
            template.name
        ),
    );
}

fn refresh_palette_choices(ui: &Rc<EditorUi>) {
    let palettes = match palette::list() {
        Ok(palettes) => palettes,
        Err(error) => {
            set_status(ui, &format!("No se pudieron cargar las paletas: {error:#}"));
            return;
        }
    };
    let mut labels = vec!["Dinámica · Aspire (matugen/tinty)".to_owned()];
    labels.extend(palettes.iter().map(|palette| palette.name.clone()));
    let refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
    ui.style.palette_sync.set(true);
    ui.style
        .palette
        .set_model(Some(&gtk::StringList::new(&refs)));
    ui.style.palette_sync.set(false);
}

fn set_fixed_palette(document: &mut BarDocument, palette: &Palette) {
    document.style_settings.use_aspire_palette = Some(false);
    document.style_settings.palette_id = Some(palette.id.clone());
    document.style_settings.palette_name = Some(palette.name.clone());
    document.style_settings.palette_colors = palette.colors.clone();
}

fn refresh_preview(ui: &Rc<EditorUi>) {
    let state = ui.state.borrow();
    let selected = state
        .selection
        .as_ref()
        .map(|selection| (selection.zone, selection.module.as_str()));
    ui.preview.render_selected(&state.document, selected);
    drop(state);
    update_dirty(ui);
}

fn build_style_editor() -> Result<StyleWidgets> {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_top(14);
    page.set_margin_bottom(14);
    page.set_margin_start(18);
    page.set_margin_end(18);

    let intro = GtkBox::new(Orientation::Horizontal, 12);
    let title = Label::new(Some("Estilo visual"));
    title.add_css_class("title-2");
    title.set_halign(gtk::Align::Start);
    intro.append(&title);
    let intro_spacer = GtkBox::new(Orientation::Horizontal, 0);
    intro_spacer.set_hexpand(true);
    intro.append(&intro_spacer);
    let retro = Button::with_label("Aplicar estilo Retro terminal");
    intro.append(&retro);
    page.append(&intro);
    let explanation = Label::new(Some(
        "Los campos numéricos muestran el valor sugerido si no hay override; el CSS avanzado conserva el estilo actual.",
    ));
    explanation.add_css_class("dim-label");
    explanation.set_halign(gtk::Align::Start);
    page.append(&explanation);

    let top = GtkBox::new(Orientation::Horizontal, 12);
    top.set_homogeneous(true);
    let typography_grid = style_grid();
    let palettes = palette::list()?;
    let mut palette_labels = vec!["Dinámica · Aspire (matugen/tinty)".to_owned()];
    palette_labels.extend(palettes.iter().map(|palette| palette.name.clone()));
    let palette_refs = palette_labels
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let palette = DropDown::from_strings(&palette_refs);
    style_row(&typography_grid, 0, "Paleta de colores", &palette);
    let palette_name = Entry::new();
    palette_name.set_placeholder_text(Some("Nombre de paleta personalizada"));
    style_row(&typography_grid, 1, "Guardar paleta como", &palette_name);
    let palette_colors = TextView::new();
    palette_colors.set_monospace(true);
    palette_colors.set_wrap_mode(gtk::WrapMode::WordChar);
    palette_colors.set_size_request(-1, 120);
    let palette_colors_scroll = ScrolledWindow::new();
    palette_colors_scroll.set_min_content_height(120);
    palette_colors_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    palette_colors_scroll.set_child(Some(&palette_colors));
    style_row(
        &typography_grid,
        2,
        "Colores canónicos (JSON)",
        &palette_colors_scroll,
    );
    let save_palette = Button::with_label("Guardar paleta personalizada");
    typography_grid.attach(&save_palette, 0, 3, 2, 1);
    let font_preset = DropDown::from_strings(&[
        "Elegir fuente",
        "JetBrainsMono Nerd Font",
        "JetBrainsMono Nerd Font Propo",
        "CaskaydiaCove Nerd Font",
        "FiraCode Nerd Font",
        "Iosevka Nerd Font",
        "monospace",
        "serif",
        "sans-serif",
    ]);
    style_row(&typography_grid, 4, "Fuentes sugeridas", &font_preset);
    let font_family = Entry::new();
    font_family.set_placeholder_text(Some("JetBrainsMono Nerd Font, monospace"));
    style_row(&typography_grid, 5, "Familia tipográfica", &font_family);
    let font_size = style_spin(6.0, 96.0, 1.0, 0);
    style_row(&typography_grid, 6, "Tamaño de fuente (px)", &font_size);
    let foreground = style_color_dropdown();
    style_row(&typography_grid, 7, "Texto global", &foreground);
    let background = style_color_dropdown();
    style_row(&typography_grid, 8, "Fondo de barra", &background);
    top.append(&style_frame("Tipografía y paleta", &typography_grid));

    let geometry_grid = style_grid();
    let opacity = style_spin(0.1, 1.0, 0.05, 2);
    style_row(&geometry_grid, 0, "Opacidad", &opacity);
    let border_radius = style_spin(0.0, 64.0, 1.0, 0);
    style_row(&geometry_grid, 1, "Radio de borde (px)", &border_radius);
    let border_width = style_spin(0.0, 16.0, 1.0, 0);
    style_row(&geometry_grid, 2, "Grosor de borde (px)", &border_width);
    let border_style =
        DropDown::from_strings(&["Sin borde", "Sólido", "Discontinuo", "Punteado", "Doble"]);
    style_row(&geometry_grid, 3, "Estilo de borde", &border_style);
    let border_color = style_color_dropdown();
    style_row(&geometry_grid, 4, "Color de borde", &border_color);
    let shadow = CheckButton::with_label("Activar sombra suave");
    style_row(&geometry_grid, 5, "Sombra", &shadow);
    top.append(&style_frame("Forma y profundidad", &geometry_grid));
    page.append(&top);

    let layout = GtkBox::new(Orientation::Horizontal, 12);
    layout.set_homogeneous(true);
    let spacing_grid = style_grid();
    let spacing = style_spin(0.0, 128.0, 1.0, 0);
    style_row(&spacing_grid, 0, "Separación entre módulos", &spacing);
    let height = style_spin(0.0, 512.0, 1.0, 0);
    style_row(&spacing_grid, 8, "Altura de la barra (px)", &height);
    let margin_top = style_spin(0.0, 512.0, 1.0, 0);
    style_row(&spacing_grid, 1, "Margen superior de barra", &margin_top);
    let margin_bottom = style_spin(0.0, 512.0, 1.0, 0);
    style_row(&spacing_grid, 2, "Margen inferior de barra", &margin_bottom);
    let margin_left = style_spin(0.0, 512.0, 1.0, 0);
    style_row(&spacing_grid, 3, "Margen izquierdo de barra", &margin_left);
    let margin_right = style_spin(0.0, 512.0, 1.0, 0);
    style_row(&spacing_grid, 4, "Margen derecho de barra", &margin_right);
    let module_padding_vertical = style_spin(0.0, 64.0, 1.0, 0);
    style_row(
        &spacing_grid,
        5,
        "Padding vertical de módulos",
        &module_padding_vertical,
    );
    let module_padding_horizontal = style_spin(0.0, 64.0, 1.0, 0);
    style_row(
        &spacing_grid,
        6,
        "Padding horizontal de módulos",
        &module_padding_horizontal,
    );
    let module_margin_horizontal = style_spin(0.0, 64.0, 1.0, 0);
    style_row(
        &spacing_grid,
        7,
        "Margen horizontal de módulos",
        &module_margin_horizontal,
    );
    layout.append(&style_frame("Geometría Waybar", &spacing_grid));

    let module_grid = style_grid();
    let module = DropDown::from_strings(&[]);
    style_row(&module_grid, 0, "Módulo", &module);
    let state = DropDown::from_strings(&[
        "normal", "hover", "warning", "critical", "urgent", "active", "disabled",
    ]);
    style_row(&module_grid, 1, "Estado", &state);
    let module_foreground = style_color_dropdown();
    style_row(&module_grid, 2, "Color del texto", &module_foreground);
    let module_background = style_color_dropdown();
    style_row(&module_grid, 3, "Color de fondo", &module_background);
    let module_actions = GtkBox::new(Orientation::Horizontal, 8);
    let save_override = Button::with_label("Guardar override");
    save_override.add_css_class("suggested-action");
    let clear_override = Button::with_label("Quitar override");
    module_actions.append(&save_override);
    module_actions.append(&clear_override);
    module_grid.attach(&module_actions, 0, 4, 2, 1);
    let module_css = TextView::new();
    module_css.set_monospace(true);
    module_css.set_editable(false);
    module_css.set_wrap_mode(gtk::WrapMode::WordChar);
    module_css.set_size_request(-1, 100);
    let module_css_scroll = ScrolledWindow::new();
    module_css_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    module_css_scroll.set_min_content_height(100);
    module_css_scroll.set_child(Some(&module_css));
    module_grid.attach(&module_css_scroll, 0, 5, 2, 1);
    layout.append(&style_frame("Overrides por módulo y estado", &module_grid));
    page.append(&layout);

    let css_row = GtkBox::new(Orientation::Horizontal, 12);
    css_row.set_homogeneous(true);
    let generated_css = TextView::new();
    generated_css.set_monospace(true);
    generated_css.set_editable(false);
    generated_css.set_wrap_mode(gtk::WrapMode::WordChar);
    generated_css.set_size_request(-1, 180);
    let generated_scroll = ScrolledWindow::new();
    generated_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    generated_scroll.set_min_content_height(180);
    generated_scroll.set_child(Some(&generated_css));
    let generated_box = GtkBox::new(Orientation::Vertical, 6);
    generated_box.append(&Label::new(Some("CSS que producen los controles")));
    generated_box.append(&generated_scroll);
    css_row.append(&style_frame("Vista previa CSS", &generated_box));

    let advanced_css = TextView::new();
    advanced_css.set_monospace(true);
    advanced_css.set_wrap_mode(gtk::WrapMode::None);
    advanced_css.set_size_request(-1, 180);
    let advanced_scroll = ScrolledWindow::new();
    advanced_scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    advanced_scroll.set_min_content_height(180);
    advanced_scroll.set_child(Some(&advanced_css));
    let advanced_box = GtkBox::new(Orientation::Vertical, 6);
    advanced_box.append(&Label::new(Some(
        "CSS avanzado adicional (para personalización completa)",
    )));
    advanced_box.append(&advanced_scroll);
    let save_css = Button::with_label("Validar y guardar CSS avanzado");
    advanced_box.append(&save_css);
    css_row.append(&style_frame("Editor CSS avanzado", &advanced_box));
    page.append(&css_row);

    let test_row = GtkBox::new(Orientation::Horizontal, 8);
    let apply_test = Button::with_label("Aplicar a prueba (10 s)");
    apply_test.add_css_class("suggested-action");
    let confirm_test = Button::with_label("Confirmar");
    confirm_test.set_sensitive(false);
    let rollback_test = Button::with_label("Revertir ahora");
    rollback_test.set_sensitive(false);
    let countdown = Label::new(Some("Sin prueba activa"));
    countdown.add_css_class("dim-label");
    test_row.append(&apply_test);
    test_row.append(&confirm_test);
    test_row.append(&rollback_test);
    test_row.append(&countdown);
    page.append(&style_frame("Prueba segura en la barra activa", &test_row));

    Ok(StyleWidgets {
        page,
        palette,
        palette_name,
        palette_colors,
        save_palette,
        palette_sync: Cell::new(false),
        font_preset,
        font_family,
        font_size,
        foreground,
        background,
        opacity,
        border_radius,
        border_width,
        border_color,
        border_style,
        shadow,
        module_padding_vertical,
        module_padding_horizontal,
        module_margin_horizontal,
        spacing,
        height,
        margin_top,
        margin_bottom,
        margin_left,
        margin_right,
        module,
        state,
        module_foreground,
        module_background,
        module_css,
        generated_css,
        advanced_css,
        save_override,
        clear_override,
        save_css,
        retro,
        apply_test,
        confirm_test,
        rollback_test,
        countdown,
    })
}

fn style_grid() -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_column_spacing(10);
    grid.set_row_spacing(8);
    grid.set_margin_top(8);
    grid.set_margin_bottom(8);
    grid.set_margin_start(10);
    grid.set_margin_end(10);
    grid
}

fn style_frame<W: IsA<gtk::Widget>>(title: &str, content: &W) -> gtk::Frame {
    let frame = gtk::Frame::new(Some(title));
    frame.set_child(Some(content));
    frame
}

fn style_row<W: IsA<gtk::Widget>>(grid: &gtk::Grid, row: i32, title: &str, widget: &W) {
    let label = Label::new(Some(title));
    label.set_halign(gtk::Align::Start);
    grid.attach(&label, 0, row, 1, 1);
    grid.attach(widget, 1, row, 1, 1);
}

fn style_spin(min: f64, max: f64, step: f64, digits: u32) -> SpinButton {
    let spin = SpinButton::with_range(min, max, step);
    spin.set_digits(digits);
    spin
}

fn style_color_dropdown() -> DropDown {
    let mut choices = vec!["Sin cambios".to_owned()];
    choices.extend(COLOR_TOKENS.iter().map(|token| format!("@{token}")));
    let choices = choices.iter().map(String::as_str).collect::<Vec<_>>();
    DropDown::from_strings(&choices)
}

fn connect_bar_selection(ui: &Rc<EditorUi>) {
    let ui_for_change = Rc::clone(ui);
    ui.bar_dropdown.connect_selected_notify(move |dropdown| {
        let index = dropdown.selected() as usize;
        let mut state = ui_for_change.state.borrow_mut();
        if index < state.document.config.bars().len() {
            state.bar_index = index;
            state.selection = first_module_selection(&state.document, index);
            drop(state);
            refresh_zones(&ui_for_change);
            refresh_style_module_choices(&ui_for_change);
            refresh_style_controls(&ui_for_change, false);
            refresh_form(&ui_for_change);
        }
    });
}

fn connect_style_controls(ui: &Rc<EditorUi>) {
    let ui_for_change = Rc::clone(ui);
    ui.style.palette.connect_selected_notify(move |dropdown| {
        if ui_for_change.style.palette_sync.get() {
            return;
        }
        let selected = dropdown.selected() as usize;
        if selected == 0 {
            update_style_settings(&ui_for_change, |settings| {
                settings.use_aspire_palette = Some(true);
                settings.palette_id = Some(palette::DYNAMIC_ID.to_owned());
                settings.palette_name = None;
                settings.palette_colors.clear();
            });
            return;
        }
        match palette::list().and_then(|palettes| {
            palettes
                .get(selected - 1)
                .cloned()
                .context("la paleta seleccionada ya no está disponible")
        }) {
            Ok(selected_palette) => {
                let mut state = ui_for_change.state.borrow_mut();
                set_fixed_palette(&mut state.document, &selected_palette);
                drop(state);
                ui_for_change
                    .style
                    .palette_name
                    .set_text(&selected_palette.name);
                ui_for_change.style.palette_colors.buffer().set_text(
                    &serde_json::to_string_pretty(&selected_palette.colors)
                        .unwrap_or_else(|_| "{}".to_owned()),
                );
                refresh_style_preview(&ui_for_change);
                set_status(
                    &ui_for_change,
                    "Paleta fija seleccionada; guarda la configuración.",
                );
            }
            Err(error) => set_status(
                &ui_for_change,
                &format!("No se pudo seleccionar la paleta: {error:#}"),
            ),
        }
    });
    let ui_for_save_palette = Rc::clone(ui);
    ui.style.save_palette.connect_clicked(move |_| {
        let name = ui_for_save_palette
            .style
            .palette_name
            .text()
            .trim()
            .to_owned();
        let colors = match serde_json::from_str::<std::collections::BTreeMap<String, String>>(
            &text_view_text(&ui_for_save_palette.style.palette_colors),
        ) {
            Ok(colors) => colors,
            Err(error) => {
                set_status(
                    &ui_for_save_palette,
                    &format!("JSON de colores inválido: {error}"),
                );
                return;
            }
        };
        let selected_palette = Palette {
            id: String::new(),
            name,
            author: "Personal".to_owned(),
            description: "Paleta creada con aspire-bar.".to_owned(),
            colors,
        };
        match palette::save_custom(&selected_palette) {
            Ok(path) => {
                refresh_palette_choices(&ui_for_save_palette);
                if let Ok(palettes) = palette::list() {
                    if let Some(index) = palettes
                        .iter()
                        .position(|item| item.name == selected_palette.name)
                    {
                        ui_for_save_palette
                            .style
                            .palette
                            .set_selected((index + 1) as u32);
                    }
                }
                set_status(
                    &ui_for_save_palette,
                    &format!("Paleta guardada en {}.", path.display()),
                );
            }
            Err(error) => set_status(
                &ui_for_save_palette,
                &format!("No se pudo guardar la paleta: {error:#}"),
            ),
        }
    });

    let ui_for_font = Rc::clone(ui);
    ui.style
        .font_preset
        .connect_selected_notify(move |dropdown| {
            if dropdown.selected() > 0 {
                if let Some(font) = dropdown_string(dropdown) {
                    ui_for_font.style.font_family.set_text(&font);
                }
            }
        });

    let ui_for_change = Rc::clone(ui);
    ui.style.font_family.connect_changed(move |entry| {
        let value = entry.text().trim().to_owned();
        update_style_settings(&ui_for_change, |settings| {
            settings.font_family = (!value.is_empty()).then_some(value);
        });
    });

    let ui_for_change = Rc::clone(ui);
    ui.style.font_size.connect_value_changed(move |spin| {
        update_style_settings(&ui_for_change, |settings| {
            settings.font_size = Some(spin.value_as_int() as u16);
        });
    });
    connect_color_change(ui, &ui.style.foreground, |settings, value| {
        settings.foreground = value;
    });
    connect_color_change(ui, &ui.style.background, |settings, value| {
        settings.background = value;
    });

    for (spin, setter) in [
        (&ui.style.opacity, 0_u8),
        (&ui.style.border_radius, 1),
        (&ui.style.border_width, 2),
        (&ui.style.module_padding_vertical, 3),
        (&ui.style.module_padding_horizontal, 4),
        (&ui.style.module_margin_horizontal, 5),
    ] {
        let ui_for_change = Rc::clone(ui);
        spin.connect_value_changed(move |spin| {
            let value = spin.value();
            update_style_settings(&ui_for_change, |settings| match setter {
                0 => settings.opacity = Some(value),
                1 => settings.border_radius = Some(value as u16),
                2 => settings.border_width = Some(value as u16),
                3 => settings.module_padding_vertical = Some(value as u16),
                4 => settings.module_padding_horizontal = Some(value as u16),
                _ => settings.module_margin_horizontal = Some(value as u16),
            });
        });
    }

    let ui_for_change = Rc::clone(ui);
    ui.style
        .border_style
        .connect_selected_notify(move |dropdown| {
            let style = match dropdown.selected() {
                1 => "solid",
                2 => "dashed",
                3 => "dotted",
                4 => "double",
                _ => "none",
            };
            update_style_settings(&ui_for_change, |settings| {
                settings.border_style = Some(style.to_owned());
            });
        });
    connect_color_change(ui, &ui.style.border_color, |settings, value| {
        settings.border_color = value;
    });
    let ui_for_change = Rc::clone(ui);
    ui.style.shadow.connect_toggled(move |check| {
        update_style_settings(&ui_for_change, |settings| {
            settings.shadow = Some(check.is_active());
        });
    });

    for (spin, key) in [
        (&ui.style.spacing, "spacing"),
        (&ui.style.height, "height"),
        (&ui.style.margin_top, "margin-top"),
        (&ui.style.margin_bottom, "margin-bottom"),
        (&ui.style.margin_left, "margin-left"),
        (&ui.style.margin_right, "margin-right"),
    ] {
        let ui_for_change = Rc::clone(ui);
        let key = key.to_owned();
        spin.connect_value_changed(move |spin| {
            update_bar_style_number(&ui_for_change, &key, spin.value_as_int());
        });
    }

    let ui_for_change = Rc::clone(ui);
    ui.style
        .module
        .connect_selected_notify(move |_| refresh_module_override_controls(&ui_for_change));
    let ui_for_change = Rc::clone(ui);
    ui.style
        .state
        .connect_selected_notify(move |_| refresh_module_override_controls(&ui_for_change));

    let ui_for_save = Rc::clone(ui);
    ui.style
        .save_override
        .connect_clicked(move |_| save_module_override(&ui_for_save));
    let ui_for_clear = Rc::clone(ui);
    ui.style
        .clear_override
        .connect_clicked(move |_| clear_module_override(&ui_for_clear));
    let ui_for_save = Rc::clone(ui);
    ui.style
        .save_css
        .connect_clicked(move |_| save_advanced_css(&ui_for_save));
    let ui_for_retro = Rc::clone(ui);
    ui.style
        .retro
        .connect_clicked(move |_| apply_retro_preset(&ui_for_retro));

    let ui_for_test = Rc::clone(ui);
    ui.style
        .apply_test
        .connect_clicked(move |_| start_test_apply(&ui_for_test));
    let ui_for_confirm = Rc::clone(ui);
    ui.style.confirm_test.connect_clicked(move |_| {
        let Some(document) = ui_for_confirm.pending_document.borrow().clone() else {
            set_status(
                &ui_for_confirm,
                "No hay una configuración de prueba para confirmar.",
            );
            return;
        };
        if let Err(error) = apply::save_project(&document) {
            set_status(
                &ui_for_confirm,
                &format!("No se pudo guardar la configuración; la prueba sigue activa: {error:#}"),
            );
            return;
        }
        let mut pending = ui_for_confirm.pending_test.borrow_mut();
        if let Some(mut pending_apply) = pending.take() {
            pending_apply.confirm();
            ui_for_confirm.pending_document.borrow_mut().take();
            ui_for_confirm
                .test_generation
                .set(ui_for_confirm.test_generation.get().wrapping_add(1));
            ui_for_confirm.style.confirm_test.set_sensitive(false);
            ui_for_confirm.style.rollback_test.set_sensitive(false);
            ui_for_confirm
                .style
                .countdown
                .set_text("Prueba confirmada.");
            ui_for_confirm.test_seconds.set(0);
            set_status(&ui_for_confirm, "Cambios confirmados en Waybar.");
        }
    });
    let ui_for_rollback = Rc::clone(ui);
    ui.style.rollback_test.connect_clicked(move |_| {
        let mut pending = ui_for_rollback.pending_test.borrow_mut();
        if let Some(pending_apply) = pending.as_mut() {
            match pending_apply.rollback() {
                Ok(()) => {
                    pending.take();
                    ui_for_rollback.pending_document.borrow_mut().take();
                    ui_for_rollback
                        .test_generation
                        .set(ui_for_rollback.test_generation.get().wrapping_add(1));
                    ui_for_rollback.style.confirm_test.set_sensitive(false);
                    ui_for_rollback.style.rollback_test.set_sensitive(false);
                    ui_for_rollback
                        .style
                        .countdown
                        .set_text("Prueba revertida.");
                    ui_for_rollback.test_seconds.set(0);
                    set_status(&ui_for_rollback, "Prueba revertida.");
                }
                Err(error) => set_status(
                    &ui_for_rollback,
                    &format!("No se pudo revertir; puedes volver a intentarlo: {error:#}"),
                ),
            }
        }
    });
}

fn connect_color_change(
    ui: &Rc<EditorUi>,
    dropdown: &DropDown,
    setter: impl Fn(&mut StyleSettings, Option<String>) + 'static,
) {
    let ui_for_change = Rc::clone(ui);
    dropdown.connect_selected_notify(move |dropdown| {
        let value = selected_color(dropdown);
        update_style_settings(&ui_for_change, |settings| setter(settings, value));
    });
}

fn update_style_settings(ui: &Rc<EditorUi>, update: impl FnOnce(&mut StyleSettings)) {
    let mut state = ui.state.borrow_mut();
    update(&mut state.document.style_settings);
    drop(state);
    refresh_style_preview(ui);
    refresh_preview(ui);
    set_status(ui, "Estilo actualizado; guarda la configuración.");
}

fn update_bar_style_number(ui: &Rc<EditorUi>, key: &str, value: i32) {
    let mut state = ui.state.borrow_mut();
    let bar_index = state.bar_index;
    let Some(bar) = state.document.config.bar_mut(bar_index) else {
        set_status(ui, "No existe la barra seleccionada.");
        return;
    };
    let number = json!(value);
    match key {
        "spacing" => bar.spacing = Some(number),
        "height" => bar.height = Some(number),
        "margin-top" => bar.margin_top = Some(number),
        "margin-bottom" => bar.margin_bottom = Some(number),
        "margin-left" => bar.margin_left = Some(number),
        "margin-right" => bar.margin_right = Some(number),
        _ => {
            set_status(ui, "Control geométrico desconocido.");
            return;
        }
    }
    drop(state);
    set_status(ui, "Geometría de Waybar actualizada; guarda la configuración.");
}

fn refresh_style_controls(ui: &Rc<EditorUi>, update_advanced: bool) {
    let state = ui.state.borrow();
    let settings = state.document.style_settings.clone();
    let style_source = state.document.style.clone();
    let bar = state.document.config.bars().get(state.bar_index);
    let geometry = bar.map(|bar| {
        (
            json_number(&bar.spacing, 0.0),
            json_number(&bar.margin_top, 0.0),
            json_number(&bar.margin_bottom, 0.0),
            json_number(&bar.margin_left, 0.0),
            json_number(&bar.margin_right, 0.0),
        )
    });
    let bar_height = bar.map(|bar| json_number(&bar.height, 36.0)).unwrap_or(36.0);
    drop(state);

    let palette_index = if settings.use_aspire_palette == Some(false) {
        palette::list()
            .ok()
            .and_then(|palettes| {
                settings.palette_name.as_ref().and_then(|name| {
                    palettes
                        .iter()
                        .position(|palette| &palette.name == name)
                        .map(|index| (index + 1) as u32)
                })
            })
            .unwrap_or(0)
    } else {
        0
    };
    ui.style.palette_sync.set(true);
    ui.style.palette.set_selected(palette_index);
    ui.style.palette_sync.set(false);
    ui.style
        .palette_name
        .set_text(settings.palette_name.as_deref().unwrap_or(""));
    ui.style.palette_colors.buffer().set_text(
        &serde_json::to_string_pretty(&settings.palette_colors).unwrap_or_else(|_| "{}".to_owned()),
    );
    ui.style
        .font_family
        .set_text(settings.font_family.as_deref().unwrap_or(""));
    ui.style.font_preset.set_selected(0);
    ui.style
        .font_size
        .set_value(f64::from(settings.font_size.unwrap_or(13)));
    set_color_dropdown(&ui.style.foreground, settings.foreground.as_deref());
    set_color_dropdown(&ui.style.background, settings.background.as_deref());
    ui.style.opacity.set_value(settings.opacity.unwrap_or(1.0));
    ui.style
        .border_radius
        .set_value(f64::from(settings.border_radius.unwrap_or(0)));
    ui.style
        .border_width
        .set_value(f64::from(settings.border_width.unwrap_or(0)));
    ui.style
        .border_color
        .set_selected(color_index(settings.border_color.as_deref()));
    ui.style
        .border_style
        .set_selected(match settings.border_style.as_deref() {
            Some("solid") => 1,
            Some("dashed") => 2,
            Some("dotted") => 3,
            Some("double") => 4,
            _ => 0,
        });
    ui.style.shadow.set_active(settings.shadow.unwrap_or(false));
    ui.style
        .module_padding_vertical
        .set_value(f64::from(settings.module_padding_vertical.unwrap_or(0)));
    ui.style
        .module_padding_horizontal
        .set_value(f64::from(settings.module_padding_horizontal.unwrap_or(0)));
    ui.style
        .module_margin_horizontal
        .set_value(f64::from(settings.module_margin_horizontal.unwrap_or(0)));
    if let Some((spacing, top, bottom, left, right)) = geometry {
        ui.style.spacing.set_value(spacing);
        ui.style.height.set_value(bar_height);
        ui.style.margin_top.set_value(top);
        ui.style.margin_bottom.set_value(bottom);
        ui.style.margin_left.set_value(left);
        ui.style.margin_right.set_value(right);
    }
    if update_advanced {
        ui.style.advanced_css.buffer().set_text(&style_source);
    }
    refresh_style_preview(ui);
}

fn refresh_style_module_choices(ui: &Rc<EditorUi>) {
    let state = ui.state.borrow();
    let modules = state
        .document
        .config
        .bars()
        .get(state.bar_index)
        .map(|bar| bar.zones().iter().flatten().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    drop(state);
    let labels = modules.iter().map(String::as_str).collect::<Vec<_>>();
    ui.style
        .module
        .set_model(Some(&gtk::StringList::new(&labels)));
    if !modules.is_empty() {
        ui.style.module.set_selected(0);
    }
    refresh_module_override_controls(ui);
}

fn refresh_module_override_controls(ui: &Rc<EditorUi>) {
    let module_name = dropdown_string(&ui.style.module);
    let state_name = dropdown_string(&ui.style.state).unwrap_or_else(|| "normal".to_owned());
    let override_style = module_name.as_ref().and_then(|module_name| {
        ui.state
            .borrow()
            .document
            .style_settings
            .modules
            .get(module_name)
            .and_then(|module| module.states.get(&state_name))
            .cloned()
    });
    set_color_dropdown(
        &ui.style.module_foreground,
        override_style
            .as_ref()
            .and_then(|style| style.foreground.as_deref()),
    );
    set_color_dropdown(
        &ui.style.module_background,
        override_style
            .as_ref()
            .and_then(|style| style.background.as_deref()),
    );
    refresh_module_css_preview(ui, module_name.as_deref(), &state_name);
}

fn refresh_style_preview(ui: &Rc<EditorUi>) {
    let state = ui.state.borrow();
    let rules = style::generated_rules(&state.document.style_settings, &state.document.config);
    let use_palette = style::uses_palette_tokens(&state.document.style_settings);
    drop(state);
    let text = if use_palette {
        match style::palette_import_rule() {
            Ok(import) => format!("{import}\n\n{rules}"),
            Err(error) => {
                set_status(
                    ui,
                    &format!("No se pudo generar la importación de colores: {error:#}"),
                );
                rules
            }
        }
    } else {
        rules
    };
    ui.style.generated_css.buffer().set_text(&text);
}

fn refresh_module_css_preview(ui: &Rc<EditorUi>, module: Option<&str>, state_name: &str) {
    let css = module.map_or_else(
        || "Selecciona un módulo para ver su CSS.".to_owned(),
        |module_name| {
            let state = ui.state.borrow();
            let mut settings = StyleSettings::default();
            if let Some(module_override) = state
                .document
                .style_settings
                .modules
                .get(module_name)
                .cloned()
            {
                if let Some(value) = module_override.states.get(state_name) {
                    settings.modules.insert(
                        module_name.to_owned(),
                        ModuleOverrides {
                            states: [(state_name.to_owned(), value.clone())].into(),
                        },
                    );
                }
            }
            let css = style::generated_rules(&settings, &state.document.config);
            if css.is_empty() {
                "/* Sin override para este módulo y estado. */".to_owned()
            } else {
                css
            }
        },
    );
    ui.style.module_css.buffer().set_text(&css);
}

fn save_module_override(ui: &Rc<EditorUi>) {
    let Some(module_name) = dropdown_string(&ui.style.module) else {
        set_status(ui, "Añade un módulo antes de configurar overrides.");
        return;
    };
    let state_name = dropdown_string(&ui.style.state).unwrap_or_else(|| "normal".to_owned());
    let override_style = StateOverride {
        foreground: selected_color(&ui.style.module_foreground),
        background: selected_color(&ui.style.module_background),
    };
    let mut state = ui.state.borrow_mut();
    if override_style.foreground.is_none() && override_style.background.is_none() {
        if let Some(module) = state.document.style_settings.modules.get_mut(&module_name) {
            module.states.remove(&state_name);
            if module.states.is_empty() {
                state.document.style_settings.modules.remove(&module_name);
            }
        }
    } else {
        state
            .document
            .style_settings
            .modules
            .entry(module_name)
            .or_default()
            .states
            .insert(state_name, override_style);
    }
    drop(state);
    refresh_style_preview(ui);
    refresh_module_override_controls(ui);
    set_status(ui, "Override aplicado; guarda la configuración.");
}

fn clear_module_override(ui: &Rc<EditorUi>) {
    let Some(module_name) = dropdown_string(&ui.style.module) else {
        return;
    };
    let state_name = dropdown_string(&ui.style.state).unwrap_or_else(|| "normal".to_owned());
    let mut state = ui.state.borrow_mut();
    if let Some(module) = state.document.style_settings.modules.get_mut(&module_name) {
        module.states.remove(&state_name);
        if module.states.is_empty() {
            state.document.style_settings.modules.remove(&module_name);
        }
    }
    drop(state);
    set_color_dropdown(&ui.style.module_foreground, None);
    set_color_dropdown(&ui.style.module_background, None);
    refresh_style_preview(ui);
    refresh_module_override_controls(ui);
    set_status(ui, "Override quitado; guarda la configuración.");
}

fn save_advanced_css(ui: &Rc<EditorUi>) {
    let input = text_view_text(&ui.style.advanced_css);
    let result = (|| {
        let (base, embedded_settings) = style::parse(&input)?;
        if base != input || embedded_settings != StyleSettings::default() {
            bail!("el CSS avanzado no puede editar los bloques reservados de aspire-bar");
        }
        let mut state = ui.state.borrow_mut();
        let mut candidate = state.document.clone();
        candidate.style = input;
        apply::validate_document(&candidate)?;
        state.document.style = candidate.style;
        Ok::<(), anyhow::Error>(())
    })();
    match result {
        Ok(()) => {
            refresh_style_preview(ui);
            set_status(ui, "CSS avanzado validado; guarda la configuración.");
        }
        Err(error) => set_status(ui, &format!("CSS avanzado no válido: {error:#}")),
    }
}

fn apply_retro_preset(ui: &Rc<EditorUi>) {
    let modules = ui.state.borrow().document.style_settings.modules.clone();
    let settings = StyleSettings {
        use_aspire_palette: Some(true),
        font_family: Some("JetBrainsMono Nerd Font, monospace".to_owned()),
        font_size: Some(12),
        foreground: Some("foreground".to_owned()),
        background: Some("background".to_owned()),
        border_radius: Some(0),
        border_width: Some(1),
        border_color: Some("active".to_owned()),
        border_style: Some("solid".to_owned()),
        shadow: Some(false),
        module_padding_vertical: Some(3),
        module_padding_horizontal: Some(6),
        module_margin_horizontal: Some(2),
        modules,
        ..StyleSettings::default()
    };
    ui.state.borrow_mut().document.style_settings = settings;
    refresh_style_controls(ui, false);
    refresh_module_override_controls(ui);
    set_status(
        ui,
        "Estilo Retro terminal aplicado; guarda la configuración para conservarlo.",
    );
}

fn start_test_apply(ui: &Rc<EditorUi>) {
    if ui.pending_test.borrow().is_some() {
        set_status(
            ui,
            "Ya hay una prueba activa; confírmala o reviértela primero.",
        );
        return;
    }
    let document = ui.state.borrow().document.clone();
    match apply::apply_for_test(&document) {
        Ok(pending) => {
            *ui.pending_test.borrow_mut() = Some(pending);
            *ui.pending_document.borrow_mut() = Some(document);
            ui.test_seconds.set(10);
            let generation = ui.test_generation.get().wrapping_add(1);
            ui.test_generation.set(generation);
            ui.style.confirm_test.set_sensitive(true);
            ui.style.rollback_test.set_sensitive(true);
            ui.style.countdown.set_text("Reversión automática en 10 s");
            set_status(ui, "Cambios aplicados temporalmente a Waybar.");
            start_test_countdown(ui, generation);
        }
        Err(error) => set_status(ui, &format!("No se pudo aplicar la prueba: {error:#}")),
    }
}

fn start_test_countdown(ui: &Rc<EditorUi>, generation: u64) {
    let weak_ui = Rc::downgrade(ui);
    gtk::glib::timeout_add_seconds_local(1, move || {
        let Some(ui) = weak_ui.upgrade() else {
            return gtk::glib::ControlFlow::Break;
        };
        if ui.test_generation.get() != generation {
            return gtk::glib::ControlFlow::Break;
        }
        if ui.pending_test.borrow().is_none() {
            return gtk::glib::ControlFlow::Break;
        }
        let remaining = ui.test_seconds.get().saturating_sub(1);
        ui.test_seconds.set(remaining);
        if remaining > 0 {
            ui.style
                .countdown
                .set_text(&format!("Reversión automática en {remaining} s"));
            return gtk::glib::ControlFlow::Continue;
        }

        let mut pending = ui.pending_test.borrow_mut();
        if let Some(pending_apply) = pending.as_mut() {
            match pending_apply.rollback() {
                Ok(()) => {
                    pending.take();
                    ui.pending_document.borrow_mut().take();
                    ui.style.confirm_test.set_sensitive(false);
                    ui.style.rollback_test.set_sensitive(false);
                    ui.style
                        .countdown
                        .set_text("Prueba revertida automáticamente.");
                    set_status(
                        &ui,
                        "La prueba terminó; Waybar se revirtió automáticamente.",
                    );
                }
                Err(error) => {
                    ui.style
                        .countdown
                        .set_text("Falló la reversión; reintenta manualmente.");
                    set_status(&ui, &format!("Falló la reversión automática: {error:#}"));
                }
            }
        }
        gtk::glib::ControlFlow::Break
    });
}

fn selected_color(dropdown: &DropDown) -> Option<String> {
    dropdown_string(dropdown)
        .filter(|value| value != "Sin cambios")
        .map(|value| value.trim_start_matches('@').to_owned())
}

fn color_index(color: Option<&str>) -> u32 {
    color
        .and_then(|color| COLOR_TOKENS.iter().position(|token| *token == color))
        .map_or(0, |index| index as u32 + 1)
}

fn set_color_dropdown(dropdown: &DropDown, color: Option<&str>) {
    dropdown.set_selected(color_index(color));
}

fn json_number(value: &Option<Value>, default: f64) -> f64 {
    value.as_ref().and_then(Value::as_f64).unwrap_or(default)
}

fn connect_zone_lists(ui: &Rc<EditorUi>) {
    let ui_for_selection = Rc::clone(ui);
    ui.active_modules.connect_row_selected(move |_, row| {
        if ui_for_selection.selection_sync.get() {
            return;
        }
        let Some(row) = row else {
            return;
        };
        let row_name = row.widget_name();
        let Some(name) = row_name.strip_prefix("aspire-active:") else {
            return;
        };
        let Some((zone, module)) = name.split_once(':') else {
            return;
        };
        if let Ok(zone) = zone.parse::<usize>() {
            set_module_selection(&ui_for_selection, zone, module);
        }
    });
}

fn connect_drop_targets(ui: &Rc<EditorUi>) {
    for (zone_index, zone) in ui.preview.zone_widgets().iter().enumerate() {
        let target = gtk::DropTarget::new(String::static_type(), DragAction::MOVE);
        let zone_for_enter = zone.clone();
        target.connect_enter(move |_, _, _| {
            zone_for_enter.add_css_class("aspire-drop-hover");
            gtk::gdk::DragAction::MOVE
        });
        let zone_for_leave = zone.clone();
        target.connect_leave(move |_| {
            zone_for_leave.remove_css_class("aspire-drop-hover");
        });
        let zone_for_drop = zone.clone();
        let ui_for_drop = Rc::clone(ui);
        target.connect_drop(move |_, value, _, _| {
            zone_for_drop.remove_css_class("aspire-drop-hover");
            match value.get::<String>() {
                Ok(module) => match place_module_in_zone(&ui_for_drop, &module, zone_index) {
                    Ok(()) => true,
                    Err(error) => {
                        set_status(&ui_for_drop, &format!("No se pudo mover: {error:#}"));
                        false
                    }
                },
                Err(error) => {
                    set_status(&ui_for_drop, &format!("Arrastre inválido: {error}"));
                    false
                }
            }
        });
        zone.add_controller(target);
    }
}

fn connect_toolbar(
    ui: &Rc<EditorUi>,
    window: &ApplicationWindow,
    add_module: &Button,
    add_bar: &Button,
    remove_bar: &Button,
    validate_button: &Button,
    save_button: &Button,
) {
    let ui_for_add = Rc::clone(ui);
    add_module.connect_clicked(move |_| {
        let selected = dropdown_string(&ui_for_add.module_dropdown).unwrap_or_default();
        let custom = ui_for_add.custom_name.text().trim().to_owned();
        if selected == "custom/new-module" && custom.is_empty() {
            set_status(
                &ui_for_add,
                "Escribe un ID para el módulo personalizado (por ejemplo, custom/cava).",
            );
            return;
        }
        let module = resolve_module_id(&selected, &custom);
        let zone = ui_for_add
            .state
            .borrow()
            .selection
            .as_ref()
            .map_or(0, |selection| selection.zone);
        match place_module_in_zone(&ui_for_add, &module, zone) {
            Ok(()) => {
                ui_for_add.custom_name.set_text("");
                set_status(
                    &ui_for_add,
                    "Módulo añadido; guarda la configuración para conservarlo.",
                );
            }
            Err(error) => set_status(&ui_for_add, &format!("No se pudo añadir: {error:#}")),
        }
    });

    let ui_for_add_bar = Rc::clone(ui);
    add_bar.connect_clicked(move |_| {
        {
            let mut state = ui_for_add_bar.state.borrow_mut();
            state.document.config.push_bar();
            state.bar_index = state.document.config.bars().len() - 1;
            state.selection = None;
        }
        refresh_bar_dropdown(&ui_for_add_bar);
        refresh_zones(&ui_for_add_bar);
        refresh_style_module_choices(&ui_for_add_bar);
        refresh_style_controls(&ui_for_add_bar, false);
        show_empty_form(&ui_for_add_bar);
        set_status(
            &ui_for_add_bar,
            "Barra nueva; guarda la configuración para conservarla.",
        );
    });

    let ui_for_remove_bar = Rc::clone(ui);
    remove_bar.connect_clicked(move |_| {
        let bar_count = ui_for_remove_bar.state.borrow().document.config.bars().len();
        if bar_count <= 1 {
            set_status(&ui_for_remove_bar, "Debe quedar al menos una barra.");
            return;
        }
        let index = ui_for_remove_bar.state.borrow().bar_index;
        {
            let mut state = ui_for_remove_bar.state.borrow_mut();
            state.document.config.remove_bar(index);
            state.bar_index = state.bar_index.min(state.document.config.bars().len().saturating_sub(1));
            state.selection = first_module_selection(&state.document, state.bar_index);
        }
        refresh_bar_dropdown(&ui_for_remove_bar);
        refresh_zones(&ui_for_remove_bar);
        refresh_style_module_choices(&ui_for_remove_bar);
        refresh_style_controls(&ui_for_remove_bar, false);
        show_empty_form(&ui_for_remove_bar);
        set_status(&ui_for_remove_bar, "Barra eliminada; guarda la configuración.");
    });

    let ui_for_validate = Rc::clone(ui);
    validate_button.connect_clicked(move |_| {
        let state = ui_for_validate.state.borrow();
        match apply::validate_document(&state.document) {
            Ok(()) => {
                drop(state);
                ui_for_validate.error_button.set_visible(false);
                ui_for_validate.error_label.set_text("");
                set_status(&ui_for_validate, "Configuración y CSS válidos.");
            }
            Err(error) => {
                drop(state);
                let detail = format!("{error:#}");
                ui_for_validate.error_label.set_text(&detail);
                ui_for_validate.error_button.set_visible(true);
                set_status(&ui_for_validate, "Validación con errores.");
            }
        }
    });

    let ui_for_save = Rc::clone(ui);
    let window_for_save = window.clone();
    save_button.connect_clicked(move |_| {
        save_and_apply(&ui_for_save, &window_for_save);
    });

    let key_controller = gtk::EventControllerKey::new();
    let ui_for_key = Rc::clone(ui);
    let window_for_key = window.clone();
    key_controller.connect_key_pressed(move |_, key, _, state| {
        let control = state.contains(gtk::gdk::ModifierType::CONTROL_MASK);
        if key == gtk::gdk::Key::s && control {
            save_and_apply(&ui_for_key, &window_for_key);
            return gtk::glib::Propagation::Stop;
        }
        if key == gtk::gdk::Key::z && control {
            let previous = ui_for_key.previous_theme.borrow().clone();
            if let Some(previous) = previous {
                apply_theme_with_guard(&ui_for_key, &previous);
            } else {
                set_status(&ui_for_key, "No hay un cambio de tema que deshacer.");
            }
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
    window.add_controller(key_controller);
}

fn save_and_apply(ui: &Rc<EditorUi>, window: &ApplicationWindow) {
    let document = ui.state.borrow().document.clone();
    let active = themes::active_theme().ok().flatten();
    let is_read_only = active
        .as_deref()
        .and_then(|name| themes::find_package(name).ok())
        .map(|package| package.is_default)
        .unwrap_or(true);
    if is_read_only {
        let ui_for_submit = Rc::clone(ui);
        prompt_text(
            window,
            "Guardar como tema nuevo",
            "Nombre del tema",
            active.as_deref().unwrap_or("Mi tema"),
            move |display_name| match themes::slugify(&display_name).and_then(|slug| {
                let manifest = gallery_manifest(&ui_for_submit, &slug, &display_name)?;
                let created = themes::create_theme(&manifest, &document)?;
                themes::set_active_theme(&slug)?;
                apply::apply_document(&document, &display_name)?;
                Ok::<_, anyhow::Error>(created)
            }) {
                Ok(path) => {
                    notify_app("Guardado", &display_name);
                    refresh_theme_gallery(&ui_for_submit);
                    refresh_quick_theme_menu(&ui_for_submit);
                    *ui_for_submit.saved_document.borrow_mut() = document.clone();
                    set_status(&ui_for_submit, &format!("Guardado: {}.", path.display()));
                }
                Err(error) => set_status(
                    &ui_for_submit,
                    &format!("No se pudo guardar: {error:#}"),
                ),
            },
        );
        return;
    }
    match apply::save_project(&document)
        .and_then(|_| apply::apply_document(&document, active.as_deref().unwrap_or("Waybar")))
    {
        Ok(()) => {
            *ui.saved_document.borrow_mut() = document;
            notify_app("Guardado", active.as_deref().unwrap_or("Waybar"));
            set_status(ui, "Guardado y aplicado.");
        }
        Err(error) => set_status(ui, &format!("No se pudo guardar: {error:#}")),
    }
}

fn resolve_module_id(selected: &str, custom: &str) -> String {
    if selected != "custom/new-module" || custom.is_empty() {
        return selected.to_owned();
    }
    if custom.contains('/') {
        custom.to_owned()
    } else {
        format!("custom/{custom}")
    }
}

fn connect_module_actions(
    ui: &Rc<EditorUi>,
    move_up: &Button,
    move_down: &Button,
    duplicate: &Button,
    remove: &Button,
) {
    let ui_up = Rc::clone(ui);
    move_up.connect_clicked(move |_| {
        if let Err(error) = move_selected(&ui_up, -1) {
            set_status(&ui_up, &format!("No se pudo reordenar: {error:#}"));
        }
    });

    let ui_down = Rc::clone(ui);
    move_down.connect_clicked(move |_| {
        if let Err(error) = move_selected(&ui_down, 1) {
            set_status(&ui_down, &format!("No se pudo reordenar: {error:#}"));
        }
    });

    let ui_duplicate = Rc::clone(ui);
    duplicate.connect_clicked(move |_| match duplicate_selected(&ui_duplicate) {
        Ok(()) => {
            refresh_zones(&ui_duplicate);
            refresh_style_module_choices(&ui_duplicate);
            refresh_form(&ui_duplicate);
            set_status(&ui_duplicate, "Módulo duplicado.");
        }
        Err(error) => set_status(&ui_duplicate, &format!("No se pudo duplicar: {error:#}")),
    });

    let ui_remove = Rc::clone(ui);
    remove.connect_clicked(move |_| match remove_selected(&ui_remove) {
        Ok(()) => {
            refresh_zones(&ui_remove);
            refresh_style_module_choices(&ui_remove);
            refresh_form(&ui_remove);
            set_status(&ui_remove, "Módulo quitado de la zona.");
        }
        Err(error) => set_status(&ui_remove, &format!("No se pudo quitar: {error:#}")),
    });
}

fn refresh_bar_dropdown(ui: &Rc<EditorUi>) {
    let (labels, selected) = {
        let state = ui.state.borrow();
        let labels = state
            .document
            .config
            .bars()
            .iter()
            .enumerate()
            .map(|(index, bar)| {
                let name = bar
                    .name
                    .as_ref()
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| {
                        bar.position
                            .as_ref()
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "Waybar".to_owned());
                format!("Barra {} · {name}", index + 1)
            })
            .collect::<Vec<_>>();
        (labels, state.bar_index)
    };
    let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
    ui.bar_dropdown
        .set_model(Some(&gtk::StringList::new(&label_refs)));
    ui.bar_dropdown
        .set_selected(selected.min(labels.len().saturating_sub(1)) as u32);
}

fn refresh_zones(ui: &Rc<EditorUi>) {
    let (modules, selected, summary) = {
        let state = ui.state.borrow();
        let Some(bar) = state.document.config.bars().get(state.bar_index) else {
            return;
        };
        let modules =
            std::array::from_fn::<_, 3, _>(|index| bar.zone(index).unwrap_or_default().to_vec());
        let position = bar
            .position
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or("top");
        let height = bar.height.as_ref().and_then(Value::as_u64).unwrap_or(36);
        let layer = bar.layer.as_ref().and_then(Value::as_str).unwrap_or("top");
        (
            modules,
            state.selection.clone(),
            format!("Posición: {position}  ·  Altura: {height}px  ·  Capa: {layer}"),
        )
    };
    ui.zone_summary.set_text(&format!(
        "L {}  ·  C {}  ·  R {}",
        modules[0].len(),
        modules[1].len(),
        modules[2].len()
    ));
    ui.summary.set_text(&summary);
    ui.selection_sync.set(true);
    clear_list(&ui.active_modules);
    for (zone_index, zone) in modules.iter().enumerate() {
        for module in zone {
            let row = ListBoxRow::new();
            row.set_widget_name(&format!("aspire-active:{zone_index}:{module}"));
            row.set_tooltip_text(Some(module));
            row.set_size_request(-1, 26);
            let title = Label::new(Some(&format!(
                "{} · {}",
                ["Izq", "Centro", "Der"][zone_index],
                display_module_name(module)
            )));
            title.set_halign(gtk::Align::Start);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            title.set_max_width_chars(20);
            title.set_margin_top(2);
            title.set_margin_bottom(2);
            title.set_margin_start(4);
            title.set_margin_end(4);
            row.set_child(Some(&title));
            attach_drag_source(&row, module);
            ui.active_modules.append(&row);
            if selected
                .as_ref()
                .is_some_and(|item| item.zone == zone_index && item.module == *module)
            {
                ui.active_modules.select_row(Some(&row));
            }
        }
    }
    ui.selection_sync.set(false);
    refresh_catalog(ui);
    refresh_preview(ui);
}

fn first_module_selection(document: &BarDocument, bar_index: usize) -> Option<Selection> {
    let bar = document.config.bars().get(bar_index)?;
    (0..3).find_map(|zone| {
        bar.zone(zone)
            .and_then(|modules| modules.first().cloned())
            .map(|module| Selection {
                zone,
                module,
            })
    })
}

fn attach_drag_source(widget: &impl IsA<gtk::Widget>, module: &str) {
    let source = gtk::DragSource::new();
    source.set_actions(DragAction::MOVE);
    let module = module.to_owned();
    source.connect_prepare(move |_, _, _| {
        let value = module.to_value();
        Some(ContentProvider::for_value(&value))
    });
    widget.add_controller(source);
}

fn clear_list(list: &ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn refresh_catalog(ui: &Rc<EditorUi>) {
    while let Some(child) = ui.catalog.first_child() {
        ui.catalog.remove(&child);
    }
    let search = ui.search.text().trim().to_lowercase();
    let (mut specs, active, selected) = {
        let state = ui.state.borrow();
        let Some(bar) = state.document.config.bars().get(state.bar_index) else {
            return;
        };
        let active = (0..3)
            .flat_map(|zone| {
                bar.zone(zone)
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |module| (module, zone))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        (catalog::modules(), active, state.selection.clone())
    };
    for (id, label, description) in [
        (
            "custom/window-title",
            "Ventana activa (script)",
            "MangoWM no ofrece un módulo Waybar nativo para el título; crea un script propio en Scripts.",
        ),
        (
            "custom/mango-submap",
            "Submapa (script)",
            "MangoWM no ofrece un módulo Waybar nativo para el submap; configura aquí un script compatible.",
        ),
    ] {
        let mut spec = catalog::module_spec(id);
        spec.label = label.to_owned();
        spec.description = description.to_owned();
        specs.push(spec);
    }
    {
        let state = ui.state.borrow();
        if let Some(bar) = state.document.config.bars().get(state.bar_index) {
            for module in bar.zones().iter().flatten() {
                if !specs.iter().any(|spec| spec.id == *module) {
                    specs.push(catalog::module_spec(module));
                }
            }
        }
    }
    let mut ids = std::collections::BTreeSet::new();
    specs.retain(|spec| {
        ids.insert(spec.id.clone())
            && spec.id != "custom/new-module"
            && !is_other_compositor_module(&spec.id)
            && (search.is_empty()
                || spec.id.to_lowercase().contains(&search)
                || spec.label.to_lowercase().contains(&search)
                || spec.description.to_lowercase().contains(&search))
    });
    let categories = ["MangoWM", "Sistema", "Media", "Red", "Otros"];
    for category in categories {
        let grouped = specs
            .iter()
            .filter(|spec| module_category(&spec.id) == category)
            .collect::<Vec<_>>();
        if grouped.is_empty() {
            continue;
        }
        let expander = gtk::Expander::new(Some(&format!("{category} · {}", grouped.len())));
        expander.set_expanded(search.is_empty() || category == "MangoWM");
        let contents = gtk::FlowBox::new();
        contents.set_row_spacing(4);
        contents.set_column_spacing(4);
        contents.set_hexpand(true);
        for spec in grouped {
            let card = GtkBox::new(Orientation::Vertical, 3);
            card.add_css_class("module-card");
            card.set_size_request(118, -1);
            card.set_margin_top(2);
            card.set_margin_bottom(2);
            card.set_margin_start(3);
            card.set_margin_end(3);
            if selected
                .as_ref()
                .is_some_and(|selection| selection.module == spec.id)
            {
                card.add_css_class("selected");
            }
            let heading = GtkBox::new(Orientation::Horizontal, 6);
            let icon = gtk::Image::from_icon_name(module_icon_name(&spec.id));
            icon.set_pixel_size(14);
            heading.append(&icon);
            let select = Button::new();
            select.add_css_class("flat");
            select.set_halign(gtk::Align::Start);
            select.set_hexpand(false);
            let name_label = Label::new(Some(&spec.id));
            name_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            name_label.set_max_width_chars(18);
            name_label.set_halign(gtk::Align::Start);
            name_label.set_xalign(0.0);
            select.set_child(Some(&name_label));
            select.set_tooltip_text(Some(&spec.description));
            let module = spec.id.clone();
            let zone = active.get(&module).copied().unwrap_or(0);
            let ui_for_select = Rc::clone(ui);
            select.connect_clicked(move |_| set_module_selection(&ui_for_select, zone, &module));
            heading.append(&select);
            card.append(&heading);
            let buttons = GtkBox::new(Orientation::Horizontal, 2);
            for (target_zone, text, tooltip) in [
                (0, "←", "Mover a la zona izquierda"),
                (1, "↕", "Mover a la zona central"),
                (2, "→", "Mover a la zona derecha"),
            ] {
                let button = Button::with_label(text);
                button.set_tooltip_text(Some(tooltip));
                button.add_css_class("flat");
                button.update_property(&[gtk::accessible::Property::Label(tooltip)]);
                let ui_for_add = Rc::clone(ui);
                let module = spec.id.clone();
                button.connect_clicked(move |_| {
                    if let Err(error) = place_module_in_zone(&ui_for_add, &module, target_zone) {
                        set_status(&ui_for_add, &format!("No se pudo colocar: {error:#}"));
                    } else {
                        set_status(
                            &ui_for_add,
                            "Módulo colocado; guarda la configuración para conservarlo.",
                        );
                    }
                });
                buttons.append(&button);
            }
            card.append(&buttons);
            attach_drag_source(&card, &spec.id);
            contents.append(&card);
        }
        expander.set_child(Some(&contents));
        ui.catalog.append(&expander);
    }
}

fn module_category(id: &str) -> &'static str {
    if matches!(
        id,
        "ext/workspaces" | "custom/window-title" | "custom/mango-submap"
    ) {
        "MangoWM"
    } else if id == "mpris" || id == "mpd" {
        "Media"
    } else if matches!(id, "network" | "bluetooth") {
        "Red"
    } else if matches!(
        id,
        "cpu"
            | "memory"
            | "temperature"
            | "battery"
            | "backlight"
            | "pulseaudio"
            | "wireplumber"
            | "disk"
            | "load"
            | "power-profiles-daemon"
            | "pulseaudio/slider"
            | "wireplumber/slider"
    ) {
        "Sistema"
    } else {
        "Otros"
    }
}

fn is_other_compositor_module(id: &str) -> bool {
    ["sway/", "dwl/", "hyprland/", "river/", "niri/"]
        .iter()
        .any(|prefix| id.starts_with(prefix))
}

fn display_module_name(name: &str) -> String {
    catalog::module_spec(name).label
}

fn set_module_selection(ui: &Rc<EditorUi>, zone: usize, module: &str) {
    ui.state.borrow_mut().selection = Some(Selection {
        zone,
        module: module.to_owned(),
    });
    refresh_zones(ui);
    refresh_form(ui);
}

fn place_module_in_zone(ui: &Rc<EditorUi>, module: &str, destination: usize) -> Result<()> {
    if module.trim().is_empty() || module.contains('\\') {
        bail!("identificador vacío o inválido");
    }
    if destination >= 3 {
        bail!("zona de destino inválida");
    }
    let mut state = ui.state.borrow_mut();
    let bar_index = state.bar_index;
    let bar = state
        .document
        .config
        .bar_mut(bar_index)
        .context("no existe la barra seleccionada")?;
    let source = (0..3).find(|zone| {
        bar.zone(*zone)
            .is_some_and(|items| items.iter().any(|item| item == module))
    });
    if source == Some(destination) {
        state.selection = Some(Selection {
            zone: destination,
            module: module.to_owned(),
        });
        drop(state);
        refresh_zones(ui);
        refresh_form(ui);
        return Ok(());
    }
    if !bar.extra.contains_key(module) {
        let spec = catalog::module_spec(module);
        bar.extra
            .insert(module.to_owned(), catalog::defaults(&spec));
    }
    if let Some(source) = source {
        let items = bar.zone_mut(source).context("zona de origen inválida")?;
        let index = items
            .iter()
            .position(|item| item == module)
            .context("el módulo ya no está en la zona de origen")?;
        items.remove(index);
    }
    bar.zone_mut(destination)
        .context("zona de módulos inválida")?
        .push(module.to_owned());
    state.selection = Some(Selection {
        zone: destination,
        module: module.to_owned(),
    });
    drop(state);
    refresh_zones(ui);
    refresh_style_module_choices(ui);
    refresh_form(ui);
    Ok(())
}

fn move_selected(ui: &Rc<EditorUi>, direction: isize) -> Result<()> {
    let mut state = ui.state.borrow_mut();
    let selection = state
        .selection
        .clone()
        .context("selecciona primero un módulo")?;
    let bar_index = state.bar_index;
    let bar = state
        .document
        .config
        .bar_mut(bar_index)
        .context("no existe la barra seleccionada")?;
    let items = bar
        .zone_mut(selection.zone)
        .context("zona de módulos inválida")?;
    let index = items
        .iter()
        .position(|item| item == &selection.module)
        .context("el módulo ya no está en la zona")?;
    let destination = index as isize + direction;
    if destination < 0 || destination >= items.len() as isize {
        return Ok(());
    }
    items.swap(index, destination as usize);
    drop(state);
    refresh_zones(ui);
    set_status(
        ui,
        "Orden actualizado; guarda la configuración para conservarlo.",
    );
    Ok(())
}

fn duplicate_selected(ui: &Rc<EditorUi>) -> Result<()> {
    let mut state = ui.state.borrow_mut();
    let selection = state
        .selection
        .clone()
        .context("selecciona primero un módulo")?;
    let bar_index = state.bar_index;
    let bar = state
        .document
        .config
        .bar_mut(bar_index)
        .context("no existe la barra seleccionada")?;
    let items = bar
        .zone(selection.zone)
        .context("zona de módulos inválida")?;
    let index = items
        .iter()
        .position(|item| item == &selection.module)
        .context("el módulo ya no está en la zona")?;
    let base_config = bar
        .extra
        .get(&selection.module)
        .cloned()
        .unwrap_or_else(|| json!({}));
    let new_name = unique_module_name(bar, &selection.module);
    bar.extra.insert(new_name.clone(), base_config);
    let items = bar
        .zone_mut(selection.zone)
        .context("zona de módulos inválida")?;
    items.insert(index + 1, new_name.clone());
    state.selection = Some(Selection {
        zone: selection.zone,
        module: new_name,
    });
    Ok(())
}

fn unique_module_name(bar: &BarConfig, original: &str) -> String {
    let is_named_instance = !original.starts_with("custom/") && !original.starts_with("group/");
    let separator = if is_named_instance { "#" } else { "-copy" };
    for index in 1.. {
        let candidate = if separator == "#" {
            format!("{original}#{index}")
        } else {
            format!(
                "{original}{separator}{}",
                if index == 1 {
                    String::new()
                } else {
                    format!("-{index}")
                }
            )
        };
        let used_in_zone = bar
            .zones()
            .iter()
            .any(|zone| zone.iter().any(|item| item == &candidate));
        if !used_in_zone && !bar.extra.contains_key(&candidate) {
            return candidate;
        }
    }
    unreachable!("la secuencia de identificadores siempre tiene un nombre libre")
}

fn remove_selected(ui: &Rc<EditorUi>) -> Result<()> {
    let mut state = ui.state.borrow_mut();
    let selection = state
        .selection
        .clone()
        .context("selecciona primero un módulo")?;
    let bar_index = state.bar_index;
    let bar = state
        .document
        .config
        .bar_mut(bar_index)
        .context("no existe la barra seleccionada")?;
    let items = bar
        .zone_mut(selection.zone)
        .context("zona de módulos inválida")?;
    let index = items
        .iter()
        .position(|item| item == &selection.module)
        .context("el módulo ya no está en la zona")?;
    items.remove(index);
    state.selection = first_module_selection(&state.document, bar_index);
    Ok(())
}

fn refresh_form(ui: &Rc<EditorUi>) {
    clear_box(&ui.form);
    let state = ui.state.borrow();
    let Some(selection) = state.selection.clone() else {
        drop(state);
        show_empty_form(ui);
        return;
    };
    let Some(bar) = state.document.config.bars().get(state.bar_index) else {
        return;
    };
    let spec = catalog::module_spec(&selection.module);
    let heading = Label::new(Some(&spec.label));
    heading.add_css_class("title-3");
    heading.set_halign(gtk::Align::Start);
    let description = Label::new(Some(&spec.description));
    description.set_wrap(true);
    description.set_halign(gtk::Align::Start);
    description.add_css_class("dim-label");
    ui.form.append(&heading);
    ui.form.append(&description);

    let object = bar
        .module_definition(&selection.module)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for field in &spec.fields {
        let value = object
            .get(&field.key)
            .cloned()
            .unwrap_or_else(|| field.default.clone());
        append_field(ui, &selection.module, field, value);
    }
    if selection.module == "clock" {
        let presets = GtkBox::new(Orientation::Horizontal, 4);
        let heading = Label::new(Some("Formato rápido"));
        heading.add_css_class("heading");
        heading.set_halign(gtk::Align::Start);
        ui.form.append(&heading);
        for (label, format) in [
            ("24 h", "{:%H:%M}"),
            ("12 h", "{:%I:%M %p}"),
            ("Fecha", "{:%a %d %b  %H:%M}"),
            ("Segundos", "{:%H:%M:%S}"),
        ] {
            let button = Button::with_label(label);
            let ui_for_click = Rc::clone(ui);
            let format = format.to_owned();
            button.connect_clicked(move |_| {
                update_module_field(&ui_for_click, "clock", "format", Value::String(format.clone()));
                refresh_form(&ui_for_click);
            });
            presets.append(&button);
        }
        ui.form.append(&presets);
    }
    append_unknown_editor(ui, &selection.module, &spec, &object);
    append_full_module_json_editor(ui, &selection.module, &Value::Object(object));
}

fn append_full_module_json_editor(ui: &Rc<EditorUi>, module: &str, definition: &Value) {
    let heading = Label::new(Some("Edición manual completa"));
    heading.add_css_class("heading");
    heading.set_halign(gtk::Align::Start);
    let note = Label::new(Some(
        "Edita aquí el objeto completo del módulo. El JSON inválido no modifica la configuración.",
    ));
    note.set_wrap(true);
    note.set_halign(gtk::Align::Start);
    note.add_css_class("dim-label");

    let text_view = TextView::new();
    text_view.set_monospace(true);
    text_view.set_wrap_mode(gtk::WrapMode::WordChar);
    text_view.set_size_request(-1, 200);
    text_view.buffer().set_text(&pretty_json(definition));
    let scroll = ScrolledWindow::new();
    scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    scroll.set_min_content_height(180);
    scroll.set_child(Some(&text_view));

    let actions = GtkBox::new(Orientation::Horizontal, 8);
    let reload = Button::with_label("Actualizar desde controles");
    let apply = Button::with_label("Aplicar JSON");
    let ui_for_reload = Rc::clone(ui);
    let module_for_reload = module.to_owned();
    let text_for_reload = text_view.clone();
    reload.connect_clicked(move |_| {
        let state = ui_for_reload.state.borrow();
        let value = state
            .document
            .config
            .bars()
            .get(state.bar_index)
            .and_then(|bar| bar.module_definition(&module_for_reload))
            .cloned()
            .unwrap_or_else(|| json!({}));
        text_for_reload.buffer().set_text(&pretty_json(&value));
        set_status(&ui_for_reload, "JSON actualizado desde los controles.");
    });

    let ui_for_apply = Rc::clone(ui);
    let module_for_apply = module.to_owned();
    apply.connect_clicked(move |_| {
        let definition = match parse_module_json(&text_view_text(&text_view)) {
            Ok(definition) => definition,
            Err(error) => {
                set_status(
                    &ui_for_apply,
                    &format!("Definición JSON inválida: {error:#}"),
                );
                return;
            }
        };
        let mut state = ui_for_apply.state.borrow_mut();
        let bar_index = state.bar_index;
        let Some(bar) = state.document.config.bar_mut(bar_index) else {
            drop(state);
            set_status(&ui_for_apply, "No existe la barra seleccionada.");
            return;
        };
        bar.extra.insert(module_for_apply.clone(), definition);
        drop(state);
        refresh_form(&ui_for_apply);
        set_status(
            &ui_for_apply,
            "Definición completa aplicada; guarda la configuración.",
        );
    });

    actions.append(&reload);
    actions.append(&apply);
    ui.form.append(&heading);
    ui.form.append(&note);
    ui.form.append(&scroll);
    ui.form.append(&actions);
}

fn parse_module_json(input: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(input).context("no se pudo analizar el JSON")?;
    if !value.is_object() {
        bail!("la definición completa debe ser un objeto JSON");
    }
    Ok(value)
}

fn append_field(ui: &Rc<EditorUi>, module: &str, field: &FieldSpec, value: Value) {
    let field_box = GtkBox::new(Orientation::Vertical, 4);
    let label = Label::new(Some(&field.label));
    label.set_halign(gtk::Align::Start);
    label.set_tooltip_text(Some(&field.description));
    field_box.append(&label);
    match field.kind {
        FieldKind::Text => {
            let entry = Entry::new();
            let text = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            entry.set_text(&text);
            entry.set_tooltip_text(Some(&field.description));
            let ui_for_change = Rc::clone(ui);
            let module = module.to_owned();
            let key = field.key.clone();
            entry.connect_changed(move |entry| {
                update_module_field(
                    &ui_for_change,
                    &module,
                    &key,
                    Value::String(entry.text().to_string()),
                );
            });
            field_box.append(&entry);
        }
        FieldKind::Number { min, max } => {
            let spin = SpinButton::with_range(min, max, 1.0);
            spin.set_value(value.as_f64().unwrap_or(0.0));
            let ui_for_change = Rc::clone(ui);
            let module = module.to_owned();
            let key = field.key.clone();
            spin.connect_value_changed(move |spin| {
                update_module_field(&ui_for_change, &module, &key, json!(spin.value_as_int()));
            });
            field_box.append(&spin);
        }
        FieldKind::Boolean => {
            let check = CheckButton::with_label(&field.description);
            check.set_active(value.as_bool().unwrap_or(false));
            let ui_for_change = Rc::clone(ui);
            let module = module.to_owned();
            let key = field.key.clone();
            check.connect_toggled(move |check| {
                update_module_field(
                    &ui_for_change,
                    &module,
                    &key,
                    Value::Bool(check.is_active()),
                );
            });
            field_box.append(&check);
        }
        FieldKind::Json => {
            let text_view = TextView::new();
            text_view.set_monospace(true);
            text_view.set_wrap_mode(gtk::WrapMode::WordChar);
            text_view.set_size_request(-1, 84);
            text_view.buffer().set_text(&pretty_json(&value));
            let scroll = ScrolledWindow::new();
            scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
            scroll.set_min_content_height(84);
            scroll.set_child(Some(&text_view));
            field_box.append(&scroll);
            let save_json = Button::with_label("Guardar JSON");
            let ui_for_save = Rc::clone(ui);
            let module = module.to_owned();
            let key = field.key.clone();
            save_json.connect_clicked(move |_| {
                let text = text_view_text(&text_view);
                match serde_json::from_str::<Value>(&text) {
                    Ok(value) => update_module_field(&ui_for_save, &module, &key, value),
                    Err(error) => {
                        set_status(&ui_for_save, &format!("{key}: JSON inválido: {error}"))
                    }
                }
            });
            field_box.append(&save_json);
        }
    }
    ui.form.append(&field_box);
}

fn append_unknown_editor(
    ui: &Rc<EditorUi>,
    module: &str,
    spec: &ModuleSpec,
    object: &Map<String, Value>,
) {
    let known: Vec<&str> = spec.fields.iter().map(|field| field.key.as_str()).collect();
    let unknown: Map<String, Value> = object
        .iter()
        .filter(|(key, _)| !known.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let heading = Label::new(Some("Propiedades adicionales"));
    heading.set_halign(gtk::Align::Start);
    heading.add_css_class("heading");
    let note = Label::new(Some(
        "JSON para claves desconocidas; se conservan al guardar.",
    ));
    note.set_wrap(true);
    note.set_halign(gtk::Align::Start);
    note.add_css_class("dim-label");
    let text_view = TextView::new();
    text_view.set_monospace(true);
    text_view.set_wrap_mode(gtk::WrapMode::WordChar);
    text_view.set_size_request(-1, 130);
    text_view
        .buffer()
        .set_text(&pretty_json(&Value::Object(unknown)));
    let scroll = ScrolledWindow::new();
    scroll.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    scroll.set_min_content_height(130);
    scroll.set_child(Some(&text_view));
    let button = Button::with_label("Guardar propiedades adicionales");
    let ui_for_save = Rc::clone(ui);
    let module = module.to_owned();
    let known_keys: Vec<String> = known.iter().map(|key| (*key).to_owned()).collect();
    button.connect_clicked(move |_| {
        let input = text_view_text(&text_view);
        let parsed = match serde_json::from_str::<Value>(&input) {
            Ok(Value::Object(values)) => values,
            Ok(_) => {
                set_status(
                    &ui_for_save,
                    "Las propiedades adicionales deben ser un objeto JSON.",
                );
                return;
            }
            Err(error) => {
                set_status(&ui_for_save, &format!("JSON adicional inválido: {error}"));
                return;
            }
        };
        if parsed.keys().any(|key| known_keys.contains(key)) {
            set_status(
                &ui_for_save,
                "No repitas aquí claves que ya aparecen como controles del esquema.",
            );
            return;
        }
        let mut state = ui_for_save.state.borrow_mut();
        let bar_index = state.bar_index;
        let Some(bar) = state.document.config.bar_mut(bar_index) else {
            set_status(&ui_for_save, "No existe la barra seleccionada.");
            return;
        };
        let entry = bar
            .extra
            .entry(module.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        let Some(object) = entry.as_object_mut() else {
            set_status(
                &ui_for_save,
                "La definición del módulo no es un objeto JSON.",
            );
            return;
        };
        object.retain(|key, _| known_keys.contains(key));
        object.extend(parsed);
        drop(state);
        set_status(
            &ui_for_save,
            "Propiedades adicionales actualizadas; guarda la configuración.",
        );
    });
    ui.form.append(&heading);
    ui.form.append(&note);
    ui.form.append(&scroll);
    ui.form.append(&button);
}

fn update_module_field(ui: &Rc<EditorUi>, module: &str, key: &str, value: Value) {
    let mut state = ui.state.borrow_mut();
    let bar_index = state.bar_index;
    let Some(bar) = state.document.config.bar_mut(bar_index) else {
        set_status(ui, "No existe la barra seleccionada.");
        return;
    };
    let entry = bar
        .extra
        .entry(module.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(object) = entry.as_object_mut() else {
        set_status(ui, "La definición del módulo no es un objeto JSON.");
        return;
    };
    object.insert(key.to_owned(), value);
    drop(state);
    refresh_preview(ui);
    set_status(ui, "Cambios sin guardar.");
}

fn remove_module_field(ui: &Rc<EditorUi>, module: &str, key: &str) {
    let mut state = ui.state.borrow_mut();
    let bar_index = state.bar_index;
    let Some(bar) = state.document.config.bar_mut(bar_index) else {
        set_status(ui, "No existe la barra seleccionada.");
        return;
    };
    if let Some(Value::Object(object)) = bar.extra.get_mut(module) {
        object.remove(key);
    }
    drop(state);
}

fn show_empty_form(ui: &Rc<EditorUi>) {
    clear_box(&ui.form);
    let label = Label::new(Some("Selecciona un módulo para editar sus propiedades."));
    label.set_wrap(true);
    label.set_halign(gtk::Align::Start);
    label.add_css_class("dim-label");
    ui.form.append(&label);
}

fn clear_box(container: &GtkBox) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn dropdown_string(dropdown: &DropDown) -> Option<String> {
    dropdown
        .selected_item()
        .and_then(|item| item.downcast::<gtk::StringObject>().ok())
        .map(|item| item.string().to_string())
}

fn text_view_text(view: &TextView) -> String {
    let buffer: TextBuffer = view.buffer();
    let (start, end) = buffer.bounds();
    buffer.text(&start, &end, false).to_string()
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|error| {
        format!(
            "{{\"error\":{}}}",
            serde_json::to_string(&error.to_string()).unwrap()
        )
    })
}

fn set_status(ui: &Rc<EditorUi>, message: &str) {
    ui.status.set_text(message);
    refresh_preview(ui);
}

fn has_unsaved_changes(ui: &Rc<EditorUi>) -> bool {
    let saved = ui.saved_document.borrow().clone();
    let current = ui.state.borrow().document.clone();
    let diff = current != saved;
    if diff {
        eprintln!("has_unsaved_changes: true");
        eprintln!("current: {:#?}", current);
        eprintln!("saved:   {:#?}", saved);
    }
    diff
}

fn update_dirty(ui: &Rc<EditorUi>) {
    let dirty = has_unsaved_changes(ui);
    ui.save_button.set_label(if dirty { "● Guardar" } else { "Guardar" });
    ui.save_button.set_tooltip_text(Some(if dirty {
        "Hay cambios sin guardar · Ctrl+S para guardar y aplicar"
    } else {
        "Guardar en el tema activo y aplicar a Waybar (Ctrl+S)"
    }));
}

thread_local! {
    static EDITOR_PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

fn system_palette_css() -> Option<String> {
    let config = dirs::config_dir()?.join("theme-engine/current");
    let mut palette = String::new();
    // `gtk.css` aporta los nombres clásicos y de libadwaita; `colors.css` los
    // nombres semánticos de aspire. Solo usamos la paleta del sistema.
    for name in ["gtk.css", "colors.css"] {
        if let Ok(text) = std::fs::read_to_string(config.join(name)) {
            palette.push_str(&text);
            palette.push('\n');
        }
    }
    palette
        .contains("@define-color background")
        .then_some(palette)
}

fn install_monitors(ui: &Rc<EditorUi>) {
    if let Ok(root) = themes::user_root() {
        let _ = std::fs::create_dir_all(&root);
        if let Ok(monitor) = gtk::gio::File::for_path(&root).monitor_directory(
            gtk::gio::FileMonitorFlags::WATCH_MOVES,
            gtk::gio::Cancellable::NONE,
        ) {
            let ui_for_change = Rc::clone(ui);
            monitor.connect_changed(move |_, _, _, _| {
                refresh_theme_gallery(&ui_for_change);
            });
            ui.monitors.borrow_mut().push(monitor);
        }
    }
    if let Some(theme_dir) = dirs::config_dir().map(|config| config.join("theme-engine")) {
        if let Ok(monitor) = gtk::gio::File::for_path(&theme_dir).monitor_directory(
            gtk::gio::FileMonitorFlags::WATCH_MOVES,
            gtk::gio::Cancellable::NONE,
        ) {
            let ui_for_palette = Rc::clone(ui);
            monitor.connect_changed(move |_, _, _, _| {
                let _ = install_editor_css();
                refresh_preview(&ui_for_palette);
                ui_for_palette.window.queue_draw();
            });
            ui.monitors.borrow_mut().push(monitor);
        }
    }
}

fn install_editor_css() -> Result<()> {
    let Some(palette) = system_palette_css() else {
        return Ok(());
    };
    let css = editor_css(&palette);
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&css);
    let display = gtk::gdk::Display::default().context("no hay un display GTK disponible")?;
    let previous = EDITOR_PROVIDER.with(|slot| slot.borrow_mut().take());
    if let Some(previous) = previous {
        gtk::style_context_remove_provider_for_display(&display, &previous);
    }
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
    EDITOR_PROVIDER.with(|slot| *slot.borrow_mut() = Some(provider));
    Ok(())
}

fn editor_css(palette: &str) -> String {
    format!(
        r#"
        {palette}

        window, notebook, notebook > stack {{
            background-color: @background;
            color: @foreground;
            font-family: "JetBrainsMono Nerd Font", monospace;
        }}
        label {{ color: @foreground; }}
        .dim-label {{ opacity: 0.72; }}
        .title-1, .title-2, .title-3, .heading {{ color: @selected; }}
        #aspire-header {{ background-color: @background-alt; }}
        #aspire-footer {{ background-color: @background-alt; }}
        #aspire-footer label {{ font-size: 11px; opacity: 0.8; }}
        #aspire-preview-top {{ background-color: @background-alt; }}
        notebook > header {{
            background-color: @background-alt;
            border-right: 1px solid alpha(@foreground, 0.10);
            padding: 6px 3px;
        }}
        notebook > header > tabs > tab {{
            background-color: transparent;
            border: none;
            border-left: 2px solid transparent;
            padding: 9px 8px;
        }}
        notebook > header > tabs > tab:checked {{
            color: @selected;
            border-left-color: @selected;
        }}
        .preview-bar-mini {{
            background-color: @background-alt;
            border: 1px solid alpha(@foreground, 0.18);
            border-radius: 12px;
            padding: 4px 8px;
        }}
        .success {{ color: @active; }}
        .module-card {{
            background-color: @background-alt;
            border: 1px solid alpha(@foreground, 0.10);
            border-radius: 7px;
            padding: 4px;
        }}
        .module-card.selected {{
            background-color: alpha(@selected, 0.16);
            border-color: @selected;
        }}
        button.flat {{
            background-color: transparent;
            border-color: transparent;
            padding: 3px 5px;
        }}
        entry, spinbutton, dropdown, textview, listbox {{
            background-color: @background-alt;
            color: @foreground;
            border: 1px solid alpha(@foreground, 0.12);
            border-radius: 5px;
        }}
        entry:focus, spinbutton:focus, dropdown:focus, textview:focus {{
            border-color: @selected;
        }}
        listbox, listview {{
            background-color: transparent;
        }}
        listbox row {{
            background-color: transparent;
            border-bottom: 1px solid alpha(@foreground, 0.08);
            padding: 2px;
        }}
        listbox row:selected {{
            background-color: alpha(@selected, 0.18);
        }}
        .boxed-list, .boxed-list > row {{
            background-color: @background-alt;
            color: @foreground;
        }}
        .boxed-list > row {{
            border: 1px solid alpha(@foreground, 0.08);
            border-radius: 6px;
            margin: 2px 0;
        }}
        button {{
            color: @foreground;
            background-image: none;
            background-color: @background-alt;
            border: 1px solid alpha(@foreground, 0.14);
            border-radius: 5px;
            padding: 2px 7px;
            font-size: 12px;
        }}
        button:hover {{
            background-color: alpha(@active, 0.22);
            border-color: @active;
        }}
        button.suggested-action {{
            color: @selectedtext;
            background-image: none;
            background-color: @selected;
            border-color: @selected;
        }}
        button.suggested-action:hover {{
            background-color: alpha(@selected, 0.86);
            border-color: @selected;
        }}
        button.suggested-action:active {{
            background-color: alpha(@selected, 0.72);
        }}
        button.suggested-action:disabled {{
            color: alpha(@selectedtext, 0.6);
            background-color: alpha(@selected, 0.4);
            border-color: transparent;
        }}
        button.destructive-action {{
            color: @urgent;
            border-color: alpha(@urgent, 0.55);
        }}
        separator {{ background-color: alpha(@foreground, 0.14); }}
        scrollbar slider {{ background-color: alpha(@foreground, 0.22); }}
        "#
    )
}

fn css_color(color: Option<aspire_core::colors::RgbColor>, fallback: &str) -> String {
    color.map_or_else(
        || fallback.to_owned(),
        |color| format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_module_id_supports_custom_and_compositor_namespaces() {
        assert_eq!(
            resolve_module_id("custom/new-module", "custom/cava"),
            "custom/cava"
        );
        assert_eq!(
            resolve_module_id("custom/new-module", "sway/window"),
            "sway/window"
        );
        assert_eq!(
            resolve_module_id("custom/new-module", "battery"),
            "custom/battery"
        );
    }

    #[test]
    fn full_module_json_accepts_unknown_properties_without_losing_values() {
        let definition = parse_module_json(
            r#"{"exec":"~/.config/waybar/cava.sh","interval":2,"return-type":"json","custom-option":{"nested":true}}"#,
        )
        .unwrap();
        assert_eq!(definition["custom-option"]["nested"], Value::Bool(true));
        assert_eq!(definition["return-type"], "json");
    }

    #[test]
    fn full_module_json_rejects_non_objects() {
        assert!(parse_module_json(r#"["not","an","object"]"#).is_err());
        assert!(parse_module_json("{").is_err());
    }

    #[test]
    fn catalog_exposes_real_mango_module_and_script_fallbacks() {
        assert_eq!(module_category("ext/workspaces"), "MangoWM");
        assert_eq!(module_category("custom/window-title"), "MangoWM");
        assert_eq!(module_category("custom/example"), "Otros");
        assert!(is_other_compositor_module("sway/window"));
        assert!(is_other_compositor_module("dwl/tags"));
        assert!(!is_other_compositor_module("ext/workspaces"));
        assert!(catalog::module_spec("custom/mango-submap")
            .description
            .contains("script propio"));
    }

    #[test]
    fn lazyvim_inspired_editor_stylesheet_is_valid_gtk_css() {
        let palette = "@define-color background #1e1e2e;\n\
                       @define-color background-alt #313244;\n\
                       @define-color foreground #cdd6f4;\n\
                       @define-color selected #cba6f7;\n\
                       @define-color selectedtext #1e1e2e;\n\
                       @define-color active #89b4fa;\n\
                       @define-color urgent #f38ba8;\n";
        let css = editor_css(palette);
        crate::css::validate_css(&css).unwrap();
        assert!(css.contains("@background"));
        assert!(css.contains("@selected"));
        assert!(css.contains("button.suggested-action"));
        assert!(css.contains("JetBrainsMono Nerd Font"));
        assert!(!css.contains("#cba6f7"));
    }
}
