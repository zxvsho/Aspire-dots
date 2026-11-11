use anyhow::{bail, Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn scripts_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("no se pudo determinar XDG_CONFIG_HOME")?
        .join("aspire-bar/scripts"))
}

pub fn save(name: &str, source: &str) -> Result<PathBuf> {
    if source.trim().is_empty() {
        bail!("el contenido del script no puede estar vacío");
    }
    let slug = slug(name)?;
    save_in(&scripts_dir()?, &slug, source)
}

pub fn save_in(directory: &Path, slug: &str, source: &str) -> Result<PathBuf> {
    if !valid_slug(slug) {
        bail!("nombre de script no válido");
    }
    if source.trim().is_empty() {
        bail!("el contenido del script no puede estar vacío");
    }
    fs::create_dir_all(directory)
        .with_context(|| format!("no se pudo crear {}", directory.display()))?;
    let path = directory.join(format!("{slug}.sh"));
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("el reloj del sistema está antes de Unix epoch")?
        .as_nanos();
    let temp = directory.join(format!(".{slug}-{}-{stamp}.tmp", std::process::id()));
    let mut contents = String::from("#!/usr/bin/env bash\nset -euo pipefail\n\n");
    contents.push_str(source);
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    fs::write(&temp, contents)
        .with_context(|| format!("no se pudo escribir {}", temp.display()))?;

    if path.exists() {
        let backup = directory.join(format!("{slug}.sh.bak-{stamp}"));
        if let Err(error) = fs::copy(&path, &backup) {
            let _ = fs::remove_file(&temp);
            return Err(error).with_context(|| format!("no se pudo respaldar {}", path.display()));
        }
    }
    if let Err(error) = fs::rename(&temp, &path) {
        let _ = fs::remove_file(&temp);
        return Err(error).with_context(|| format!("no se pudo publicar {}", path.display()));
    }
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions)
        .with_context(|| format!("no se pudo hacer ejecutable {}", path.display()))?;
    Ok(path)
}

pub fn shell_quote(path: &Path) -> String {
    let path = path.to_string_lossy();
    format!("'{}'", path.replace('\'', "'\\''"))
}

fn slug(name: &str) -> Result<String> {
    let slug = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if !valid_slug(&slug) {
        bail!("usa un nombre de script con letras, números y guiones");
    }
    Ok(slug)
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 64
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_executable_script_and_backs_up_previous_version() {
        let directory = std::env::temp_dir().join(format!(
            "aspire-bar-scripts-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = save_in(&directory, "battery-status", "printf 'ok\\n'").unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "#!/usr/bin/env bash\nset -euo pipefail\n\nprintf 'ok\\n'\n"
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o755
        );
        save_in(&directory, "battery-status", "printf 'new\\n'").unwrap();
        assert_eq!(
            fs::read_dir(&directory).unwrap().count(),
            2,
            "la segunda escritura conserva el backup"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shell_quotes_paths_with_spaces_and_quotes() {
        assert_eq!(shell_quote(Path::new("/tmp/a b")), "'/tmp/a b'");
        assert_eq!(shell_quote(Path::new("/tmp/a'b")), "'/tmp/a'\\''b'");
    }
}
