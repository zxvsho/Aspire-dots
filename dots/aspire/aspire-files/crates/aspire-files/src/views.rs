//! Fábricas de las vistas de lista y cuadrícula.

use std::rc::Rc;

use gtk::prelude::*;
use gtk::{glib, pango};

use aspire_fs::Entry;

use crate::app::App;

pub fn icon_for(entry: &Entry) -> &'static str {
    if entry.is_dir {
        return "folder-symbolic";
    }
    match entry.extension().as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" | "tif" | "tiff" | "avif" => {
            "folder-pictures-symbolic"
        }
        "pdf" => "text-x-generic-symbolic",
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "ogv" | "mpg" | "mpeg" => {
            "media-playback-start-symbolic"
        }
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "opus" => "audio-volume-high-symbolic",
        "zip" | "tar" | "gz" | "xz" | "zst" | "7z" | "rar" => "text-x-generic-symbolic",
        "rs" | "py" | "js" | "ts" | "c" | "h" | "cpp" | "sh" | "toml" | "yaml" | "yml" | "json" => {
            "application-x-executable-symbolic"
        }
        "txt" | "md" | "rst" | "log" => "text-x-generic-symbolic",
        _ => "text-x-generic-symbolic",
    }
}

fn entry_of(item: &gtk::ListItem) -> Option<Entry> {
    let object = item.item()?.downcast::<glib::BoxedAnyObject>().ok()?;
    let entry = object.try_borrow::<Entry>().ok()?.clone();
    Some(entry)
}

pub fn name_factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let icon = gtk::Image::new();
        icon.set_pixel_size(16);
        let label = gtk::Label::new(None);
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(pango::EllipsizeMode::Middle);
        row.append(&icon);
        row.append(&label);
        item.set_child(Some(&row));
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(entry) = entry_of(item) else {
            return;
        };
        let Some(row) = item.child().and_downcast::<gtk::Box>() else {
            return;
        };
        if let Some(icon) = row.first_child().and_downcast::<gtk::Image>() {
            icon.set_icon_name(Some(icon_for(&entry)));
        }
        if let Some(label) = row.last_child().and_downcast::<gtk::Label>() {
            label.set_text(&entry.name);
        }
    });
    factory
}

pub fn size_factory() -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let label = gtk::Label::new(None);
        label.set_xalign(1.0);
        item.set_child(Some(&label));
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(entry) = entry_of(item) else {
            return;
        };
        if let Some(label) = item.child().and_downcast::<gtk::Label>() {
            let text = if entry.is_dir {
                "—".to_string()
            } else {
                aspire_fs::format_size(entry.size)
            };
            label.set_text(&text);
        }
    });
    factory
}

pub fn modified_factory(detailed: std::rc::Rc<std::cell::Cell<bool>>) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let label = gtk::Label::new(None);
        label.set_xalign(1.0);
        item.set_child(Some(&label));
    });
    factory.connect_bind(move |_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(entry) = entry_of(item) else {
            return;
        };
        if let Some(label) = item.child().and_downcast::<gtk::Label>() {
            label.set_text(&format_modified(entry.modified, detailed.get()));
        }
    });
    factory
}

fn format_modified(time: Option<std::time::SystemTime>, detailed: bool) -> String {
    let Some(time) = time else {
        return "—".into();
    };
    let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) else {
        return "—".into();
    };
    let Ok(datetime) = glib::DateTime::from_unix_local(duration.as_secs() as i64) else {
        return "—".into();
    };
    let format = if detailed {
        "%a, %d %b %Y · %H:%M:%S"
    } else {
        "%Y-%m-%d %H:%M"
    };
    datetime
        .format(format)
        .map(|text| text.to_string())
        .unwrap_or_else(|_| "—".into())
}

pub fn grid_factory(app: &Rc<App>, icon_size: i32) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    let cell_width = icon_size + 44;
    factory.connect_setup(move |_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 6);
        cell.add_css_class("grid-cell");
        cell.set_size_request(cell_width, -1);
        cell.set_halign(gtk::Align::Center);
        let image = gtk::Image::new();
        image.set_pixel_size(icon_size);
        image.set_size_request(icon_size, icon_size);
        let label = gtk::Label::new(None);
        label.add_css_class("grid-label");
        label.set_wrap(true);
        label.set_wrap_mode(pango::WrapMode::WordChar);
        label.set_lines(2);
        label.set_justify(gtk::Justification::Center);
        label.set_ellipsize(pango::EllipsizeMode::Middle);
        label.set_max_width_chars(18);
        cell.append(&image);
        cell.append(&label);
        item.set_child(Some(&cell));
    });
    factory.connect_bind({
        let app = app.clone();
        move |_, item| {
            let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let Some(entry) = entry_of(item) else {
                return;
            };
            let Some(cell) = item.child().and_downcast::<gtk::Box>() else {
                return;
            };
            let Some(image) = cell.first_child().and_downcast::<gtk::Image>() else {
                return;
            };
            if let Some(label) = cell.last_child().and_downcast::<gtk::Label>() {
                label.set_text(&entry.name);
            }
            let tooltip = format!(
                "{}\n{}\n{}",
                entry.name,
                if entry.is_dir {
                    "Folder".to_string()
                } else {
                    aspire_fs::format_size(entry.size)
                },
                entry.path.display()
            );
            cell.set_tooltip_text(Some(&tooltip));

            if entry.is_dir {
                image.set_icon_name(Some("folder-symbolic"));
                return;
            }
            if app.show_thumbnails() {
                if let Some(path) = app.thumbnail_path(&entry) {
                    image.set_from_file(Some(path));
                    return;
                }
            }
            image.set_icon_name(Some(icon_for(&entry)));
        }
    });
    factory
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(name: &str, is_dir: bool) -> Entry {
        Entry {
            path: PathBuf::from("/tmp").join(name),
            name: name.to_string(),
            is_dir,
            is_symlink: false,
            is_hidden: false,
            size: 0,
            modified: None,
        }
    }

    #[test]
    fn folders_use_folder_icon() {
        assert_eq!(icon_for(&entry("Manual", true)), "folder-symbolic");
    }

    #[test]
    fn code_and_scripts_use_executable_icon() {
        assert_eq!(
            icon_for(&entry("build.sh", false)),
            "application-x-executable-symbolic"
        );
        assert_eq!(
            icon_for(&entry("main.rs", false)),
            "application-x-executable-symbolic"
        );
    }

    #[test]
    fn media_and_documents_have_specific_icons() {
        assert_eq!(
            icon_for(&entry("foto.png", false)),
            "folder-pictures-symbolic"
        );
        assert_eq!(
            icon_for(&entry("manual.pdf", false)),
            "text-x-generic-symbolic"
        );
        assert_eq!(
            icon_for(&entry("video.mkv", false)),
            "media-playback-start-symbolic"
        );
    }
}
