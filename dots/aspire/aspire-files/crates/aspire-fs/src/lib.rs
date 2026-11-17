//! Capa de archivos de aspire-files: listado, ordenación, operaciones,
//! marcadores y papelera Freedesktop.
//!
//! No depende de GTK. Todas las funciones devuelven `Result` y no silencian
//! errores.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{bail, Context, Result};
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};

/// Caracteres que se codifican al escribir rutas en `.trashinfo`.
const PATH_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Size,
    Type,
    Modified,
}

impl SortBy {
    pub fn parse(value: &str) -> Self {
        match value {
            "name" => Self::Name,
            "size" => Self::Size,
            "type" => Self::Type,
            _ => Self::Modified,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Size => "size",
            Self::Type => "type",
            Self::Modified => "modified",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ListOptions {
    pub show_hidden: bool,
    pub folders_first: bool,
    pub sort_by: SortBy,
    pub sort_descending: bool,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self {
            show_hidden: false,
            folders_first: true,
            sort_by: SortBy::Modified,
            sort_descending: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl Entry {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let symlink_meta = fs::symlink_metadata(&path).ok()?;
        let is_symlink = symlink_meta.file_type().is_symlink();
        let meta = fs::metadata(&path).unwrap_or(symlink_meta);
        Some(Self {
            is_dir: meta.is_dir(),
            is_symlink,
            is_hidden: name.starts_with('.'),
            size: if meta.is_dir() { 0 } else { meta.len() },
            modified: meta.modified().ok(),
            path,
            name,
        })
    }

    pub fn extension(&self) -> String {
        self.path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }
}

/// Lista un directorio aplicando las opciones de vista.
pub fn list_dir(dir: &Path, options: ListOptions) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let read = fs::read_dir(dir).with_context(|| format!("no se pudo abrir {}", dir.display()))?;
    for item in read {
        let Ok(item) = item else { continue };
        let path = item.path();
        let Some(entry) = Entry::from_path(path) else {
            continue;
        };
        if entry.is_hidden && !options.show_hidden {
            continue;
        }
        entries.push(entry);
    }
    sort_entries(&mut entries, options);
    Ok(entries)
}

pub fn sort_entries(entries: &mut [Entry], options: ListOptions) {
    entries.sort_by(|a, b| {
        let folder_order = match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        };
        if options.folders_first && folder_order != std::cmp::Ordering::Equal {
            return folder_order;
        }
        let base = match options.sort_by {
            SortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortBy::Size => a.size.cmp(&b.size),
            SortBy::Type => a
                .extension()
                .cmp(&b.extension())
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            SortBy::Modified => a
                .modified
                .cmp(&b.modified)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        };
        if options.sort_descending {
            base.reverse()
        } else {
            base
        }
    });
}

/// Busca por subcadena, insensible a mayúsculas. `recursive = false` limita
/// al directorio actual. Omite los árboles virtuales del sistema.
pub fn search(root: &Path, query: &str, recursive: bool, show_hidden: bool) -> Result<Vec<Entry>> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    let mut walker = walkdir::WalkDir::new(root).follow_links(false);
    if !recursive {
        walker = walker.max_depth(1);
    }
    for item in walker.into_iter().filter_entry(|e| {
        if !show_hidden && e.depth() > 0 {
            if let Some(name) = e.file_name().to_str() {
                if name.starts_with('.') {
                    return false;
                }
            }
        }
        true
    }) {
        let Ok(item) = item else { continue };
        if item.depth() == 0 {
            continue;
        }
        let path = item.path();
        if path.starts_with("/proc")
            || path.starts_with("/sys")
            || path.starts_with("/dev")
            || path.starts_with("/run")
        {
            continue;
        }
        let name = item.file_name().to_string_lossy().to_lowercase();
        if name.contains(&needle) {
            if let Some(entry) = Entry::from_path(path.to_path_buf()) {
                result.push(entry);
            }
        }
        if result.len() >= 5000 {
            break;
        }
    }
    result.sort_by_key(|entry| entry.name.to_lowercase());
    Ok(result)
}

// ---------------------------------------------------------------------------
// Operaciones de archivo
// ---------------------------------------------------------------------------

pub fn create_dir(parent: &Path, name: &str) -> Result<PathBuf> {
    let destination = unique_path(parent, name);
    fs::create_dir(&destination)
        .with_context(|| format!("no se pudo crear {}", destination.display()))?;
    Ok(destination)
}

pub fn rename_entry(path: &Path, new_name: &str) -> Result<PathBuf> {
    let new_name = new_name.trim();
    if new_name.is_empty() || new_name.contains('/') {
        bail!("nombre inválido");
    }
    let parent = path.parent().context("la ruta no tiene padre")?;
    let destination = parent.join(new_name);
    if destination == path {
        return Ok(destination);
    }
    if destination.exists() {
        bail!("ya existe {}", destination.display());
    }
    fs::rename(path, &destination)
        .with_context(|| format!("no se pudo renombrar {}", path.display()))?;
    Ok(destination)
}

/// Copia un archivo, directorio o enlace simbólico de forma recursiva.
pub fn copy_path(source: &Path, destination: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(source)
        .with_context(|| format!("no se pudo leer {}", source.display()))?;
    if meta.file_type().is_symlink() {
        let target = fs::read_link(source)?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, destination)?;
        #[cfg(not(unix))]
        fs::copy(source, destination)?;
        return Ok(());
    }
    if meta.is_dir() {
        fs::create_dir_all(destination)?;
        for item in fs::read_dir(source)? {
            let item = item?;
            copy_path(&item.path(), &destination.join(item.file_name()))?;
        }
        return Ok(());
    }
    fs::copy(source, destination)
        .with_context(|| format!("no se pudo copiar {}", source.display()))?;
    Ok(())
}

/// Mueve con `rename` y, si cruza sistemas de archivos, copia y borra.
pub fn move_path(source: &Path, destination: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(err) if err.raw_os_error() == Some(18) => {
            copy_path(source, destination)?;
            remove_path(source)
        }
        Err(err) => Err(err)
            .with_context(|| format!("no se pudo mover {}", source.display())),
    }
}

pub fn remove_path(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .with_context(|| format!("no se pudo borrar {}", path.display()))
}

/// Copia o mueve una lista de rutas a un directorio, resolviendo colisiones.
pub fn transfer(paths: &[PathBuf], target_dir: &Path, cut: bool) -> Result<Vec<PathBuf>> {
    let mut done = Vec::new();
    for source in paths {
        if source == target_dir || target_dir.starts_with(source) {
            continue;
        }
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "archivo".into());
        let destination = unique_path(target_dir, &name);
        if cut {
            move_path(source, &destination)?;
        } else {
            copy_path(source, &destination)?;
        }
        done.push(destination);
    }
    Ok(done)
}

/// Devuelve una ruta libre dentro de `dir`, añadiendo ` (copy N)`.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());
    let extension = path.extension().map(|e| e.to_string_lossy().to_string());
    for index in 2..10_000 {
        let file_name = match &extension {
            Some(ext) => format!("{stem} (copy {index}).{ext}"),
            None => format!("{stem} (copy {index})"),
        };
        let candidate = dir.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem} (copy)"))
}

// ---------------------------------------------------------------------------
// Marcadores (~/.config/gtk-3.0/bookmarks)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bookmark {
    pub path: PathBuf,
    pub label: String,
}

pub fn bookmarks_file() -> PathBuf {
    config_home().join("gtk-3.0/bookmarks")
}

pub fn read_bookmarks() -> Vec<Bookmark> {
    let Ok(text) = fs::read_to_string(bookmarks_file()) else {
        return Vec::new();
    };
    parse_bookmarks(&text)
}

pub fn parse_bookmarks(text: &str) -> Vec<Bookmark> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let mut parts = line.splitn(2, ' ');
            let uri = parts.next()?;
            let label = parts.next().unwrap_or("").trim();
            let path = uri_to_path(uri)?;
            let label = if label.is_empty() {
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string())
            } else {
                label.to_string()
            };
            Some(Bookmark { path, label })
        })
        .collect()
}

pub fn write_bookmarks(bookmarks: &[Bookmark]) -> Result<()> {
    let path = bookmarks_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = String::new();
    for bookmark in bookmarks {
        text.push_str(&format!(
            "{} {}\n",
            path_to_uri(&bookmark.path),
            bookmark.label
        ));
    }
    fs::write(&path, text).with_context(|| format!("no se pudo escribir {}", path.display()))
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let raw = uri.strip_prefix("file://").unwrap_or(uri);
    let decoded = percent_decode_str(raw).decode_utf8().ok()?;
    Some(PathBuf::from(decoded.to_string()))
}

fn path_to_uri(path: &Path) -> String {
    let encoded = utf8_percent_encode(&path.to_string_lossy(), PATH_ENCODE_SET).to_string();
    format!("file://{encoded}")
}

// ---------------------------------------------------------------------------
// Lugares XDG
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Place {
    pub name: String,
    pub path: PathBuf,
    pub icon: &'static str,
    pub is_trash: bool,
}

pub fn places() -> Vec<Place> {
    let home = home_dir();
    let mut places = vec![Place {
        name: "Home".into(),
        path: home.clone(),
        icon: "user-home-symbolic",
        is_trash: false,
    }];
    for (name, dir, icon) in [
        ("Desktop", "Desktop", "user-desktop-symbolic"),
        ("Documents", "Documents", "folder-documents-symbolic"),
        ("Downloads", "Downloads", "folder-download-symbolic"),
        ("Music", "Music", "folder-music-symbolic"),
        ("Pictures", "Pictures", "folder-pictures-symbolic"),
        ("Videos", "Videos", "folder-videos-symbolic"),
        ("Public", "Public", "folder-publicshare-symbolic"),
        ("Templates", "Templates", "folder-templates-symbolic"),
    ] {
        let path = home.join(dir);
        if path.exists() {
            places.push(Place {
                name: name.into(),
                path,
                icon,
                is_trash: false,
            });
        }
    }
    places.push(Place {
        name: "Trash".into(),
        path: trash_root(),
        icon: "user-trash-symbolic",
        is_trash: true,
    });
    places
}

// ---------------------------------------------------------------------------
// Papelera Freedesktop
// ---------------------------------------------------------------------------

pub fn trash_root() -> PathBuf {
    data_home().join("Trash")
}

pub fn trash_files_dir() -> PathBuf {
    trash_root().join("files")
}

pub fn trash_info_dir() -> PathBuf {
    trash_root().join("info")
}

#[derive(Clone, Debug)]
pub struct TrashEntry {
    pub entry: Entry,
    pub original: PathBuf,
    pub info_path: PathBuf,
    pub stored_name: String,
}

/// Mueve una ruta a la papelera del usuario y escribe su `.trashinfo`.
pub fn trash_path(path: &Path) -> Result<()> {
    trash_path_at(path, &trash_root())
}

/// Igual que [`trash_path`], pero con una raíz de papelera explícita.
pub fn trash_path_at(path: &Path, root: &Path) -> Result<()> {
    let files_dir = root.join("files");
    let info_dir = root.join("info");
    fs::create_dir_all(&files_dir)?;
    fs::create_dir_all(&info_dir)?;

    let original = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .context("ruta sin nombre")?;
    let stored_name = unique_stored_name(&files_dir, &name);
    let destination = files_dir.join(&stored_name);

    move_path(path, &destination)?;

    let info_path = info_dir.join(format!("{stored_name}.trashinfo"));
    let encoded = utf8_percent_encode(&original.to_string_lossy(), PATH_ENCODE_SET).to_string();
    let date = deletion_date();
    let info = format!("[Trash Info]\nPath={encoded}\nDeletionDate={date}\n");
    if let Err(err) = fs::write(&info_path, info) {
        // Si falla el .trashinfo, devolvemos el archivo a su sitio.
        let _ = move_path(&destination, path);
        return Err(err).context("no se pudo escribir el .trashinfo");
    }
    Ok(())
}

pub fn list_trash() -> Result<Vec<TrashEntry>> {
    list_trash_at(&trash_root())
}

pub fn list_trash_at(root: &Path) -> Result<Vec<TrashEntry>> {
    let files_dir = root.join("files");
    let info_dir = root.join("info");
    let mut result = Vec::new();
    let Ok(read) = fs::read_dir(&info_dir) else {
        return Ok(result);
    };
    for item in read {
        let Ok(item) = item else { continue };
        let info_path = item.path();
        if info_path.extension().map(|e| e != "trashinfo").unwrap_or(true) {
            continue;
        }
        let stored_name = info_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let stored_path = files_dir.join(&stored_name);
        if !stored_path.exists() {
            continue;
        }
        let Some(entry) = Entry::from_path(stored_path) else {
            continue;
        };
        let original = fs::read_to_string(&info_path)
            .ok()
            .and_then(|text| {
                text.lines()
                    .find_map(|line| line.strip_prefix("Path=").map(str::to_string))
            })
            .map(|raw| percent_decode_str(&raw).decode_utf8_lossy().to_string())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&stored_name));
        result.push(TrashEntry {
            entry,
            original,
            info_path,
            stored_name,
        });
    }
    result.sort_by_key(|item| std::cmp::Reverse(item.entry.modified));
    Ok(result)
}

pub fn restore_trash(item: &TrashEntry) -> Result<PathBuf> {
    let parent = item
        .original
        .parent()
        .context("la ruta original no tiene padre")?;
    fs::create_dir_all(parent)?;
    let name = item
        .original
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| item.stored_name.clone());
    let destination = unique_path(parent, &name);
    move_path(&item.entry.path, &destination)?;
    let _ = fs::remove_file(&item.info_path);
    Ok(destination)
}

pub fn delete_trash_item(item: &TrashEntry) -> Result<()> {
    remove_path(&item.entry.path)?;
    let _ = fs::remove_file(&item.info_path);
    Ok(())
}

pub fn empty_trash() -> Result<()> {
    empty_trash_at(&trash_root())
}

pub fn empty_trash_at(root: &Path) -> Result<()> {
    let files_dir = root.join("files");
    let info_dir = root.join("info");
    if files_dir.exists() {
        for item in fs::read_dir(&files_dir)? {
            let item = item?;
            remove_path(&item.path())?;
        }
    }
    if info_dir.exists() {
        for item in fs::read_dir(&info_dir)? {
            let item = item?;
            let path = item.path();
            if path.extension().map(|e| e == "trashinfo").unwrap_or(false) {
                let _ = fs::remove_file(path);
            }
        }
    }
    Ok(())
}

fn unique_stored_name(files_dir: &Path, name: &str) -> String {
    if !files_dir.join(name).exists() {
        return name.to_string();
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());
    let extension = path.extension().map(|e| e.to_string_lossy().to_string());
    for index in 1..10_000 {
        let candidate = match &extension {
            Some(ext) => format!("{stem}.{index}.{ext}"),
            None => format!("{stem}.{index}"),
        };
        if !files_dir.join(&candidate).exists() {
            return candidate;
        }
    }
    format!("{name}.{}", std::process::id())
}

fn deletion_date() -> String {
    // Fecha local en formato ISO 8601 sin dependencias externas.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (year, month, day, hour, minute, second) = unix_to_local(now as i64);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
}

/// Conversión UTC→local usando la zona horaria del sistema vía `localtime_r`.
#[cfg(unix)]
fn unix_to_local(seconds: i64) -> (i32, u32, u32, u32, u32, u32) {
    #[repr(C)]
    struct Tm {
        tm_sec: i32,
        tm_min: i32,
        tm_hour: i32,
        tm_mday: i32,
        tm_mon: i32,
        tm_year: i32,
        tm_wday: i32,
        tm_yday: i32,
        tm_isdst: i32,
        tm_gmtoff: i64,
        tm_zone: *const i8,
    }
    extern "C" {
        fn localtime_r(time: *const i64, result: *mut Tm) -> *mut Tm;
    }
    let mut tm = Tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };
    unsafe {
        localtime_r(&seconds, &mut tm);
    }
    (
        tm.tm_year + 1900,
        (tm.tm_mon + 1) as u32,
        tm.tm_mday as u32,
        tm.tm_hour as u32,
        tm.tm_min as u32,
        tm.tm_sec as u32,
    )
}

#[cfg(not(unix))]
fn unix_to_local(seconds: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = seconds / 86_400;
    let rem = seconds % 86_400;
    (1970 + (days / 365) as i32, 1, 1, (rem / 3600) as u32, 0, 0)
}

// ---------------------------------------------------------------------------
// Utilidades
// ---------------------------------------------------------------------------

pub fn is_appimage(path: &Path) -> bool {
    path.extension()
        .map(|e| e.eq_ignore_ascii_case("appimage"))
        .unwrap_or(false)
}

#[cfg(unix)]
pub fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path)?;
    let mut permissions = meta.permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

pub fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local/share"))
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[allow(dead_code)]
fn io_error(message: &str, err: io::Error) -> anyhow::Error {
    anyhow::Error::new(err).context(message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aspire-fs-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entry(path: PathBuf, is_dir: bool, size: u64) -> Entry {
        Entry {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            path,
            is_dir,
            is_symlink: false,
            is_hidden: false,
            size,
            modified: None,
        }
    }

    #[test]
    fn parses_bookmarks() {
        let text = "file:///home/user/Documentos Documentos\nfile:///tmp/a%20b Sin%20nombre\n";
        let bookmarks = parse_bookmarks(text);
        assert_eq!(bookmarks.len(), 2);
        assert_eq!(bookmarks[0].path, PathBuf::from("/home/user/Documentos"));
        assert_eq!(bookmarks[1].path, PathBuf::from("/tmp/a b"));
    }

    #[test]
    fn bookmark_uris_roundtrip() {
        let path = PathBuf::from("/tmp/carpeta con espacios/ñandú#1");
        let uri = path_to_uri(&path);
        assert_eq!(uri_to_path(&uri), Some(path));
    }

    #[test]
    fn unique_names_do_not_collide() {
        let dir = temp_dir("unique");
        fs::write(dir.join("hola.txt"), b"x").unwrap();
        let unique = unique_path(&dir, "hola.txt");
        assert_eq!(unique, dir.join("hola (copy 2).txt"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(3 * 1024 * 1024), "3.0 MB");
    }

    #[test]
    fn sorts_folders_first_and_by_name() {
        let mut entries = vec![
            entry(PathBuf::from("/x/b.txt"), false, 2),
            entry(PathBuf::from("/x/a"), true, 0),
            entry(PathBuf::from("/x/A.txt"), false, 1),
        ];
        let options = ListOptions {
            folders_first: true,
            sort_by: SortBy::Name,
            sort_descending: false,
            ..ListOptions::default()
        };
        sort_entries(&mut entries, options);
        assert_eq!(entries[0].name, "a");
        assert_eq!(entries[1].name, "A.txt");
        assert_eq!(entries[2].name, "b.txt");
    }

    #[test]
    fn sorts_by_size_descending() {
        let mut entries = vec![
            entry(PathBuf::from("/x/small"), false, 1),
            entry(PathBuf::from("/x/big"), false, 100),
        ];
        let options = ListOptions {
            folders_first: false,
            sort_by: SortBy::Size,
            sort_descending: true,
            ..ListOptions::default()
        };
        sort_entries(&mut entries, options);
        assert_eq!(entries[0].name, "big");
    }

    #[test]
    fn sorts_by_type_then_name() {
        let mut entries = vec![
            entry(PathBuf::from("/x/b.txt"), false, 1),
            entry(PathBuf::from("/x/a.md"), false, 1),
            entry(PathBuf::from("/x/c.txt"), false, 1),
        ];
        let options = ListOptions {
            folders_first: false,
            sort_by: SortBy::Type,
            sort_descending: false,
            ..ListOptions::default()
        };
        sort_entries(&mut entries, options);
        assert_eq!(entries[0].name, "a.md");
        assert_eq!(entries[1].name, "b.txt");
        assert_eq!(entries[2].name, "c.txt");
    }

    #[test]
    fn lists_hidden_files_only_when_requested() {
        let dir = temp_dir("hidden");
        fs::write(dir.join("visible.txt"), b"x").unwrap();
        fs::write(dir.join(".oculto"), b"x").unwrap();

        let visible = list_dir(&dir, ListOptions::default()).unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "visible.txt");

        let all = list_dir(
            &dir,
            ListOptions {
                show_hidden: true,
                ..ListOptions::default()
            },
        )
        .unwrap();
        assert_eq!(all.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_respects_recursion() {
        let dir = temp_dir("search");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("nota.txt"), b"x").unwrap();
        fs::write(dir.join("sub/nota-oculta.txt"), b"x").unwrap();

        let shallow = search(&dir, "nota", false, false).unwrap();
        assert_eq!(shallow.len(), 1);
        let deep = search(&dir, "nota", true, false).unwrap();
        assert_eq!(deep.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn create_dir_avoids_collisions() {
        let dir = temp_dir("create");
        let first = create_dir(&dir, "Nueva").unwrap();
        let second = create_dir(&dir, "Nueva").unwrap();
        assert!(first.is_dir());
        assert!(second.is_dir());
        assert_ne!(first, second);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_rejects_collisions_and_slashes() {
        let dir = temp_dir("rename");
        fs::write(dir.join("a.txt"), b"a").unwrap();
        fs::write(dir.join("b.txt"), b"b").unwrap();
        assert!(rename_entry(&dir.join("a.txt"), "b.txt").is_err());
        assert!(rename_entry(&dir.join("a.txt"), "sub/a.txt").is_err());
        let renamed = rename_entry(&dir.join("a.txt"), "c.txt").unwrap();
        assert!(renamed.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_and_transfer_preserve_tree() {
        let dir = temp_dir("copy");
        let source = dir.join("origen");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("nested/file.txt"), b"contenido").unwrap();
        let destination = dir.join("destino");
        copy_path(&source, &destination).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("nested/file.txt")).unwrap(),
            "contenido"
        );

        let moved = transfer(std::slice::from_ref(&destination), &dir, true).unwrap();
        assert!(!destination.exists());
        assert!(moved[0].exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn trash_lifecycle() {
        let dir = temp_dir("trash");
        let root = dir.join("Trash");
        let work = dir.join("work");
        fs::create_dir_all(&work).unwrap();
        let file = work.join("documento.txt");
        fs::write(&file, b"hola").unwrap();
        let original = file.canonicalize().unwrap();

        trash_path_at(&file, &root).unwrap();
        assert!(!file.exists());
        let items = list_trash_at(&root).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].original, original);

        let restored = restore_trash(&items[0]).unwrap();
        assert_eq!(fs::read_to_string(&restored).unwrap(), "hola");
        assert!(list_trash_at(&root).unwrap().is_empty());

        trash_path_at(&restored, &root).unwrap();
        let items = list_trash_at(&root).unwrap();
        delete_trash_item(&items[0]).unwrap();
        assert!(list_trash_at(&root).unwrap().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_trash_removes_everything() {
        let dir = temp_dir("empty-trash");
        let root = dir.join("Trash");
        for name in ["uno.txt", "dos.txt"] {
            let file = dir.join(name);
            fs::write(&file, b"x").unwrap();
            trash_path_at(&file, &root).unwrap();
        }
        assert_eq!(list_trash_at(&root).unwrap().len(), 2);
        empty_trash_at(&root).unwrap();
        assert!(list_trash_at(&root).unwrap().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_appimages() {
        assert!(is_appimage(Path::new("/tmp/App.AppImage")));
        assert!(!is_appimage(Path::new("/tmp/app.bin")));
    }
}
