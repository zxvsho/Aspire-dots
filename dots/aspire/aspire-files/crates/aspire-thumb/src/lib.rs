//! Miniaturas para aspire-files.
//!
//! Genera PNG en caché (`~/.cache/aspire-files/thumbnails`) a partir de
//! imágenes (gdk-pixbuf), primera página de PDF (`pdftoppm`) y vídeo
//! (`ffmpegthumbnailer`, si está instalado). La caché es LRU por fecha de
//! modificación y se poda a 512 archivos.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use gdk_pixbuf::Pixbuf;
use gio::prelude::FileExt;

const CACHE_LIMIT: usize = 512;
const CACHE_TARGET: usize = 400;

pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".cache")
        })
        .join("aspire-files/thumbnails")
}

/// Devuelve la ruta de la miniatura en caché, generándola si hace falta.
pub fn thumbnail(path: &Path, size: i32) -> Option<PathBuf> {
    if size <= 0 {
        return None;
    }
    let meta = fs::metadata(path).ok()?;
    if meta.is_dir() {
        return None;
    }
    let key = cache_key(path, meta.len(), size);
    let cache = cache_dir();
    fs::create_dir_all(&cache).ok()?;
    let output = cache.join(format!("{key:016x}.png"));
    if output.exists() {
        // Tocar el archivo mantiene el criterio LRU.
        let _ = fs::OpenOptions::new().append(true).open(&output);
        return Some(output);
    }

    let extension = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let generated = if is_image(&extension) {
        generate_image(path, size, &output)
    } else if extension == "pdf" {
        generate_pdf(path, size, &output)
    } else if is_video(&extension) {
        generate_video(path, size, &output)
    } else {
        false
    };
    // Si no hay generador propio (o falla), se prueban los thumbnailers
    // registrados en el sistema (audio, vídeo, Office, RAW…).
    let generated = generated || generate_thumbnailer(path, size, &output);

    if generated && output.exists() {
        prune_cache(&cache);
        Some(output)
    } else {
        let _ = fs::remove_file(&output);
        None
    }
}

fn is_image(extension: &str) -> bool {
    matches!(
        extension,
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
    )
}

fn is_video(extension: &str) -> bool {
    matches!(
        extension,
        "mp4" | "m4v" | "webm" | "mkv" | "mov" | "avi" | "ogv" | "mpg" | "mpeg" | "flv" | "wmv"
    )
}

fn generate_image(path: &Path, size: i32, output: &Path) -> bool {
    match Pixbuf::from_file_at_scale(path, size, size, true) {
        Ok(pixbuf) => pixbuf.savev(output, "png", &[]).is_ok(),
        Err(_) => false,
    }
}

fn generate_pdf(path: &Path, size: i32, output: &Path) -> bool {
    let temporary = output.with_extension("pdf.png");
    let prefix = temporary.with_extension("");
    let status = Command::new("pdftoppm")
        .arg("-f")
        .arg("1")
        .arg("-l")
        .arg("1")
        .arg("-singlefile")
        .arg("-scale-to")
        .arg(size.to_string())
        .arg("-png")
        .arg(path)
        .arg(&prefix)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let produced = prefix.with_extension("png");
    let ok = matches!(status, Ok(status) if status.success()) && produced.exists();
    if ok {
        let _ = fs::rename(&produced, output);
    } else {
        let _ = fs::remove_file(&produced);
    }
    let _ = fs::remove_file(&temporary);
    ok
}

fn generate_video(path: &Path, size: i32, output: &Path) -> bool {
    let status = Command::new("ffmpegthumbnailer")
        .arg("-i")
        .arg(path)
        .arg("-o")
        .arg(output)
        .arg("-s")
        .arg(size.to_string())
        .arg("-q")
        .arg("6")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    matches!(status, Ok(status) if status.success()) && output.exists()
}

// ---------------------------------------------------------------------------
// Thumbnailers XDG
// ---------------------------------------------------------------------------

/// Busca un thumbnailer registrado que acepte el tipo MIME del archivo.
fn generate_thumbnailer(path: &Path, size: i32, output: &Path) -> bool {
    let (mime, _) = gio::content_type_guess(Some(path), None::<&[u8]>);
    let mime = mime.to_string();
    if mime.is_empty() || mime == "application/octet-stream" {
        return false;
    }
    let uri = gio::File::for_path(path).uri().to_string();

    for directory in thumbnailer_dirs() {
        let Ok(read) = fs::read_dir(&directory) else {
            continue;
        };
        for item in read.flatten() {
            let definition = item.path();
            if definition
                .extension()
                .map(|extension| extension != "thumbnailer")
                .unwrap_or(true)
            {
                continue;
            }
            let Ok(text) = fs::read_to_string(&definition) else {
                continue;
            };
            let Some((mime_types, exec)) = parse_thumbnailer(&text) else {
                continue;
            };
            if !mime_types
                .iter()
                .any(|pattern| mime_matches(pattern, &mime))
            {
                continue;
            }
            let command = substitute_exec(&exec, path, output, size, &uri);
            if run_with_timeout(&command, 15) && output.exists() {
                return true;
            }
            let _ = fs::remove_file(output);
        }
    }
    false
}

fn thumbnailer_dirs() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        });
    if let Some(data_home) = data_home {
        directories.push(data_home.join("thumbnailers"));
    }
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    for directory in data_dirs.split(':').filter(|part| !part.is_empty()) {
        directories.push(PathBuf::from(directory).join("thumbnailers"));
    }
    directories
}

/// Extrae `MimeType` y `Exec` de un `.thumbnailer`.
fn parse_thumbnailer(text: &str) -> Option<(Vec<String>, String)> {
    let mut mime_types = Vec::new();
    let mut exec = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "MimeType" => {
                mime_types = value
                    .split(';')
                    .map(str::trim)
                    .filter(|entry| !entry.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            "Exec" => exec = Some(value.trim().to_string()),
            _ => {}
        }
    }
    let exec = exec?;
    if mime_types.is_empty() || exec.is_empty() {
        return None;
    }
    Some((mime_types, exec))
}

fn mime_matches(pattern: &str, mime: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return mime == prefix || mime.starts_with(&format!("{prefix}/"));
    }
    pattern == mime
}

fn substitute_exec(exec: &str, input: &Path, output: &Path, size: i32, uri: &str) -> String {
    exec.replace("%i", &shell_quote(&input.to_string_lossy()))
        .replace("%o", &shell_quote(&output.to_string_lossy()))
        .replace("%s", &size.to_string())
        .replace("%u", &shell_quote(uri))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn run_with_timeout(command: &str, seconds: u64) -> bool {
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}

fn cache_key(path: &Path, file_size: u64, thumb_size: i32) -> u64 {
    // FNV-1a determinista (DefaultHasher no garantiza estabilidad entre
    // versiones de Rust).
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut write = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    };
    write(path.to_string_lossy().as_bytes());
    write(&file_size.to_le_bytes());
    write(&thumb_size.to_le_bytes());
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                write(&duration.as_secs().to_le_bytes());
                write(&duration.subsec_nanos().to_le_bytes());
            }
        }
    }
    hash
}

fn prune_cache(cache: &Path) {
    let Ok(read) = fs::read_dir(cache) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = read
        .filter_map(|item| {
            let item = item.ok()?;
            let path = item.path();
            if path.extension().map(|e| e != "png").unwrap_or(true) {
                return None;
            }
            let modified = item.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect();
    if files.len() <= CACHE_LIMIT {
        return;
    }
    files.sort_by_key(|(modified, _)| *modified);
    for (_, path) in files.iter().take(files.len().saturating_sub(CACHE_TARGET)) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_images() {
        assert!(is_image("png"));
        assert!(is_image("webp"));
        assert!(!is_image("mp4"));
    }

    #[test]
    fn classifies_videos() {
        assert!(is_video("mkv"));
        assert!(is_video("webm"));
        assert!(!is_video("pdf"));
    }

    #[test]
    fn parses_thumbnailer_definitions() {
        let text = r#"
[Thumbnailer Entry]
TryExec=foo-thumbnailer
Exec=/usr/bin/foo-thumbnailer -i %i -o %o -s %s
MimeType=image/png;image/jpeg;
"#;
        let (mimes, exec) = parse_thumbnailer(text).unwrap();
        assert_eq!(mimes, vec!["image/png", "image/jpeg"]);
        assert!(exec.contains("%i"));
    }

    #[test]
    fn rejects_incomplete_thumbnailers() {
        assert!(parse_thumbnailer("Exec=foo %i %o\n").is_none());
        assert!(parse_thumbnailer("MimeType=image/png;\n").is_none());
    }

    #[test]
    fn matches_mime_patterns() {
        assert!(mime_matches("image/png", "image/png"));
        assert!(mime_matches("image/*", "image/jpeg"));
        assert!(mime_matches("*", "application/pdf"));
        assert!(!mime_matches("image/*", "video/mp4"));
    }

    #[test]
    fn substitutes_thumbnailer_placeholders() {
        let command = substitute_exec(
            "thumb -i %i -o %o -s %s -u %u",
            Path::new("/tmp/a b.png"),
            Path::new("/tmp/out.png"),
            128,
            "file:///tmp/a%20b.png",
        );
        assert!(command.contains("'/tmp/a b.png'"));
        assert!(command.contains("'/tmp/out.png'"));
        assert!(command.contains("128"));
        assert!(command.contains("'file:///tmp/a%20b.png'"));
    }

    #[test]
    fn cache_keys_are_stable_and_size_sensitive() {
        let path = Path::new("/tmp/does-not-exist.png");
        let first = cache_key(path, 100, 128);
        assert_eq!(first, cache_key(path, 100, 128));
        assert_ne!(first, cache_key(path, 100, 64));
        assert_ne!(first, cache_key(path, 101, 128));
    }

    #[test]
    fn missing_files_have_no_thumbnail() {
        assert!(thumbnail(Path::new("/tmp/aspire-thumb-missing.png"), 64).is_none());
    }

    #[test]
    fn directories_have_no_thumbnail() {
        let dir = std::env::temp_dir();
        assert!(thumbnail(&dir, 64).is_none());
    }

    #[test]
    fn generates_image_thumbnails() {
        let path = std::env::temp_dir().join(format!(
            "aspire-thumb-{}.png",
            std::process::id()
        ));
        let pixbuf = Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, false, 8, 8, 8).unwrap();
        pixbuf.savev(&path, "png", &[]).unwrap();
        let generated = thumbnail(&path, 32);
        assert!(generated.as_ref().map(|p| p.exists()).unwrap_or(false));
        if let Some(generated) = generated {
            let _ = fs::remove_file(generated);
        }
        let _ = fs::remove_file(&path);
    }
}
