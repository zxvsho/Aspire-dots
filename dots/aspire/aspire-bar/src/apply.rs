use crate::css::validate_css;
use crate::document::BarDocument;
use crate::{aspire_bar_dir, waybar_dir};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BACKUP_LIMIT: usize = 10;

pub struct PendingApply {
    config_path: PathBuf,
    style_path: PathBuf,
    previous_config: PathSnapshot,
    previous_style: PathSnapshot,
    snapshot_dir: Option<PathBuf>,
    resolved: bool,
}

#[derive(Clone, Debug)]
struct PathSnapshot {
    contents: Option<Vec<u8>>,
    symlink_target: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct StoredPathState {
    exists: bool,
    symlink_target: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct StoredPairState {
    config: StoredPathState,
    style: StoredPathState,
}

impl PendingApply {
    pub fn confirm(&mut self) {
        self.resolved = true;
    }

    pub fn rollback(&mut self) -> Result<()> {
        if self.resolved {
            return Ok(());
        }
        let current_config = capture_path(&self.config_path)?;
        let current_style = capture_path(&self.style_path)?;
        let restore_result = restore_snapshot(&self.config_path, &self.previous_config)
            .and_then(|()| restore_snapshot(&self.style_path, &self.previous_style));
        if let Err(error) = restore_result {
            let config_rollback = restore_snapshot(&self.config_path, &current_config);
            let style_rollback = restore_snapshot(&self.style_path, &current_style);
            return match (config_rollback, style_rollback) {
                (Ok(()), Ok(())) => Err(error).context("falló la reversión de la prueba; se revirtió"),
                (config, style) => bail!(
                    "falló la reversión de la prueba ({error:#}); la restauración parcial tampoco se pudo deshacer: config={config:?}, style={style:?}"
                ),
            };
        }
        reload_waybar().context("se restauraron los archivos, pero Waybar no pudo recargarlos")?;
        if let Some(snapshot_dir) = &self.snapshot_dir {
            fs::remove_dir_all(snapshot_dir)
                .with_context(|| format!("no se pudo limpiar {}", snapshot_dir.display()))?;
        }
        self.resolved = true;
        Ok(())
    }
}

pub fn project_paths() -> Result<(PathBuf, PathBuf)> {
    let root = aspire_bar_dir()?;
    Ok((root.join("config.jsonc"), root.join("style.css")))
}

pub fn live_paths() -> Result<(PathBuf, PathBuf)> {
    let root = waybar_dir()?;
    Ok((root.join("config.jsonc"), root.join("style.css")))
}

pub fn load_project_or_live() -> Result<BarDocument> {
    let (project_config, project_style) = project_paths()?;
    if project_config.is_file() && project_style.is_file() {
        BarDocument::read(&project_config, &project_style)
    } else {
        let (live_config, live_style) = live_paths()?;
        BarDocument::read(&live_config, &live_style)
    }
}

pub fn import(config_path: &Path, style_path: &Path) -> Result<()> {
    let document = BarDocument::read(config_path, style_path)?;
    validate_document(&document)?;
    save_project(&document)
}

pub fn save_project(document: &BarDocument) -> Result<()> {
    validate_document(document)?;
    let (config_path, style_path) = project_paths()?;
    let config = document.generated_config()?;
    let style = document.generated_style()?;
    atomic_write_pair(
        &config_path,
        config.as_bytes(),
        &style_path,
        style.as_bytes(),
    )
}

pub fn check(config_path: &Path, style_path: &Path) -> Result<BarDocument> {
    let document = BarDocument::read(config_path, style_path)?;
    validate_document(&document)?;
    Ok(document)
}

pub fn validate_document(document: &BarDocument) -> Result<()> {
    for (index, bar) in document.config.bars().iter().enumerate() {
        for (zone, modules) in [
            ("modules-left", bar.modules_left.as_ref()),
            ("modules-center", bar.modules_center.as_ref()),
            ("modules-right", bar.modules_right.as_ref()),
        ] {
            if let Some(modules) = modules {
                for module in modules {
                    if module.trim().is_empty() {
                        bail!("barra {}: hay un nombre vacío en {zone}", index + 1);
                    }
                }
            }
        }
    }
    validate_css(&document.generated_style()?)
}

pub fn list_themes() -> Result<Vec<(String, PathBuf)>> {
    Ok(crate::themes::list_packages()?
        .into_iter()
        .map(|package| (package.manifest.name, package.path))
        .collect())
}

pub fn find_theme(name: &str) -> Result<PathBuf> {
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        bail!("nombre de tema inválido: {name}");
    }
    list_themes()?
        .into_iter()
        .find_map(|(theme, path)| (theme == name).then_some(path))
        .with_context(|| format!("no existe el tema «{name}»"))
}

pub fn apply_theme(name: &str) -> Result<()> {
    let package = crate::themes::find_package(name)?;
    apply_document(&package.document, name)?;
    crate::themes::set_active_theme(name)?;
    println!("Tema aplicado: {name}");
    Ok(())
}

pub fn apply_document(document: &BarDocument, label: &str) -> Result<()> {
    validate_document(document)?;
    let (config_path, style_path) = live_paths()?;
    let previous_config = capture_path(&config_path)?;
    let previous_style = capture_path(&style_path)?;
    backup_live_pair(&config_path, &style_path)?;
    let snapshot_dir = publish_snapshot(label, document, &config_path, &style_path)?;
    let mut pending = PendingApply {
        config_path,
        style_path,
        previous_config,
        previous_style,
        snapshot_dir: Some(snapshot_dir),
        resolved: false,
    };
    if let Err(error) = reload_waybar() {
        return match pending.rollback() {
            Ok(()) => Err(error).context("Waybar no pudo recargar el tema; se revirtió"),
            Err(rollback_error) => bail!(
                "Waybar no pudo recargar el tema ({error:#}) y tampoco se pudo revertir ({rollback_error:#})"
            ),
        };
    }
    pending.confirm();
    Ok(())
}

pub fn apply_for_test(document: &BarDocument) -> Result<PendingApply> {
    validate_document(document)?;
    let (config_path, style_path) = live_paths()?;
    let previous_config = capture_path(&config_path)?;
    let previous_style = capture_path(&style_path)?;
    backup_live_pair(&config_path, &style_path)?;
    let snapshot_dir = publish_snapshot("prueba", document, &config_path, &style_path)?;
    let mut pending = PendingApply {
        config_path,
        style_path,
        previous_config,
        previous_style,
        snapshot_dir: Some(snapshot_dir),
        resolved: false,
    };
    if let Err(reload_error) = reload_waybar() {
        return match pending.rollback() {
            Ok(()) => Err(reload_error).context("Waybar no pudo recargar la prueba; se revirtió"),
            Err(rollback_error) => bail!(
                "Waybar no pudo recargar la prueba ({reload_error:#}) y tampoco se pudo revertir ({rollback_error:#})"
            ),
        };
    }
    Ok(pending)
}

pub fn export(destination: &Path) -> Result<()> {
    let document = load_project_or_live()?;
    validate_document(&document)?;
    fs::create_dir_all(destination)
        .with_context(|| format!("no se pudo crear {}", destination.display()))?;
    atomic_write(
        &destination.join("config.jsonc"),
        document.generated_config()?.as_bytes(),
    )?;
    atomic_write(
        &destination.join("style.css"),
        document.generated_style()?.as_bytes(),
    )?;
    println!("Configuración exportada a {}", destination.display());
    Ok(())
}

pub fn restore() -> Result<()> {
    let (config_path, style_path) = live_paths()?;
    let backup_dir = backup_dir()?;
    let backups = backup_pairs(&backup_dir)?;
    let Some((_, config_backup, style_backup, state_backup)) = backups.last() else {
        bail!("no hay copias de seguridad de aspire-bar para restaurar");
    };

    let state = if state_backup.is_file() {
        let state_source = fs::read(state_backup)?;
        serde_json::from_slice::<StoredPairState>(&state_source)
            .with_context(|| format!("estado de backup inválido: {}", state_backup.display()))?
    } else {
        StoredPairState {
            config: StoredPathState {
                exists: true,
                symlink_target: None,
            },
            style: StoredPathState {
                exists: true,
                symlink_target: None,
            },
        }
    };
    let config = snapshot_from_backup(config_backup, &state.config)?;
    let style = snapshot_from_backup(style_backup, &state.style)?;
    backup_live_pair(&config_path, &style_path)?;
    let current_config = capture_path(&config_path)?;
    let current_style = capture_path(&style_path)?;
    let restore_result = restore_snapshot(&config_path, &config)
        .and_then(|()| restore_snapshot(&style_path, &style));
    if let Err(error) = restore_result {
        let config_rollback = restore_snapshot(&config_path, &current_config);
        let style_rollback = restore_snapshot(&style_path, &current_style);
        return match (config_rollback, style_rollback) {
            (Ok(()), Ok(())) => Err(error).context("falló la restauración; se revirtió"),
            (config, style) => bail!(
                "falló la restauración ({error:#}); la reversión también falló: config={config:?}, style={style:?}"
            ),
        };
    }
    reload_waybar()?;
    println!("Se restauró la última copia de aspire-bar.");
    Ok(())
}

fn backup_live_pair(config_path: &Path, style_path: &Path) -> Result<()> {
    let config = capture_path(config_path)?;
    let style = capture_path(style_path)?;
    let backup_dir = backup_dir()?;
    fs::create_dir_all(&backup_dir)
        .with_context(|| format!("no se pudo crear {}", backup_dir.display()))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("el reloj del sistema está antes de Unix epoch")?
        .as_nanos();
    let stem = format!("{stamp}");
    if let Some(contents) = &config.contents {
        atomic_write(&backup_dir.join(format!("{stem}.config.jsonc")), contents)?;
    }
    if let Some(contents) = &style.contents {
        atomic_write(&backup_dir.join(format!("{stem}.style.css")), contents)?;
    }
    let state = StoredPairState {
        config: store_path_state(&config),
        style: store_path_state(&style),
    };
    atomic_write(
        &backup_dir.join(format!("{stem}.state.json")),
        &serde_json::to_vec_pretty(&state)?,
    )?;
    rotate_backups(&backup_dir)?;
    Ok(())
}

fn backup_dir() -> Result<PathBuf> {
    Ok(waybar_dir()?.join("backups/aspire-bar"))
}

fn backup_pairs(dir: &Path) -> Result<Vec<(String, PathBuf, PathBuf, PathBuf)>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut pairs = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("no se pudo leer {}", dir.display()))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".config.jsonc") else {
            continue;
        };
        let style = dir.join(format!("{stem}.style.css"));
        let state = dir.join(format!("{stem}.state.json"));
        if style.is_file() || state.is_file() {
            pairs.push((stem.to_owned(), entry.path(), style, state));
        }
    }
    pairs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(pairs)
}

fn rotate_backups(dir: &Path) -> Result<()> {
    let backups = backup_pairs(dir)?;
    for (_, config, style, state) in backups.into_iter().rev().skip(BACKUP_LIMIT) {
        for path in [config, style, state] {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("no se pudo eliminar {}", path.display()))
                }
            }
        }
    }
    Ok(())
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("la ruta destino no tiene directorio padre")?;
    fs::create_dir_all(parent).with_context(|| format!("no se pudo crear {}", parent.display()))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("el reloj del sistema está antes de Unix epoch")?
        .as_nanos();
    let temp_path = parent.join(format!(".aspire-bar-{}-{stamp}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .with_context(|| format!("no se pudo crear {}", temp_path.display()))?;
    let write_result = (|| -> Result<()> {
        file.write_all(contents)
            .with_context(|| format!("no se pudo escribir {}", temp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("no se pudo sincronizar {}", temp_path.display()))?;
        fs::rename(&temp_path, path)
            .with_context(|| format!("no se pudo publicar {}", path.display()))?;
        if let Ok(directory) = OpenOptions::new().read(true).open(parent) {
            directory
                .sync_all()
                .with_context(|| format!("no se pudo sincronizar {}", parent.display()))?;
        }
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

fn atomic_write_pair(
    first_path: &Path,
    first_contents: &[u8],
    second_path: &Path,
    second_contents: &[u8],
) -> Result<()> {
    let previous_first = capture_path(first_path)?;
    let previous_second = capture_path(second_path)?;
    let write_result = atomic_write(first_path, first_contents)
        .and_then(|()| atomic_write(second_path, second_contents));
    if let Err(error) = write_result {
        let rollback_first = restore_snapshot(first_path, &previous_first);
        let rollback_second = restore_snapshot(second_path, &previous_second);
        return match (rollback_first, rollback_second) {
            (Ok(()), Ok(())) => Err(error).context("falló la escritura; se revirtió la pareja"),
            (first, second) => bail!(
                "falló la escritura ({error:#}); la reversión también falló: config={first:?}, style={second:?}"
            ),
        };
    }
    Ok(())
}

fn capture_path(path: &Path) -> Result<PathSnapshot> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PathSnapshot {
                contents: None,
                symlink_target: None,
            })
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("no se pudo inspeccionar {}", path.display()))
        }
    };
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(path)
            .with_context(|| format!("no se pudo leer el enlace {}", path.display()))?;
        let contents = fs::read(path)
            .with_context(|| format!("no se pudo leer el destino del enlace {}", path.display()))?;
        Ok(PathSnapshot {
            contents: Some(contents),
            symlink_target: Some(target),
        })
    } else if metadata.is_file() {
        Ok(PathSnapshot {
            contents: Some(
                fs::read(path).with_context(|| format!("no se pudo leer {}", path.display()))?,
            ),
            symlink_target: None,
        })
    } else {
        bail!(
            "la ruta no es archivo regular ni symlink: {}",
            path.display()
        )
    }
}

fn restore_snapshot(path: &Path, snapshot: &PathSnapshot) -> Result<()> {
    if let Some(target) = &snapshot.symlink_target {
        replace_symlink(path, target)
    } else if let Some(contents) = &snapshot.contents {
        atomic_write(path, contents)
    } else {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => {
                Err(error).with_context(|| format!("no se pudo quitar {}", path.display()))
            }
        }
    }
}

fn store_path_state(snapshot: &PathSnapshot) -> StoredPathState {
    StoredPathState {
        exists: snapshot.contents.is_some(),
        symlink_target: snapshot
            .symlink_target
            .as_ref()
            .map(|target| target.to_string_lossy().into_owned()),
    }
}

fn snapshot_from_backup(path: &Path, state: &StoredPathState) -> Result<PathSnapshot> {
    let contents = if state.exists {
        Some(fs::read(path).with_context(|| format!("no se pudo leer {}", path.display()))?)
    } else {
        None
    };
    Ok(PathSnapshot {
        contents,
        symlink_target: state.symlink_target.as_deref().map(PathBuf::from),
    })
}

fn publish_snapshot(
    label: &str,
    document: &BarDocument,
    config_path: &Path,
    style_path: &Path,
) -> Result<PathBuf> {
    let safe_label = label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let root = aspire_bar_dir()?.join("applied");
    fs::create_dir_all(&root).with_context(|| format!("no se pudo crear {}", root.display()))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("el reloj del sistema está antes de Unix epoch")?
        .as_nanos();
    let snapshot_dir = root.join(format!("{safe_label}-{stamp}"));
    fs::create_dir(&snapshot_dir)
        .with_context(|| format!("no se pudo crear {}", snapshot_dir.display()))?;
    let result = (|| {
        atomic_write(
            &snapshot_dir.join("config.jsonc"),
            document.generated_config()?.as_bytes(),
        )?;
        atomic_write(
            &snapshot_dir.join("style.css"),
            document.generated_style()?.as_bytes(),
        )?;
        switch_live_pair(
            config_path,
            &snapshot_dir.join("config.jsonc"),
            style_path,
            &snapshot_dir.join("style.css"),
        )
    })();
    if let Err(error) = result {
        fs::remove_dir_all(&snapshot_dir)
            .with_context(|| format!("no se pudo limpiar {}", snapshot_dir.display()))?;
        return Err(error);
    }
    Ok(snapshot_dir)
}

fn switch_live_pair(
    config_path: &Path,
    config_target: &Path,
    style_path: &Path,
    style_target: &Path,
) -> Result<()> {
    let previous_config = capture_path(config_path)?;
    let previous_style = capture_path(style_path)?;
    let result = replace_symlink(config_path, config_target)
        .and_then(|()| replace_symlink(style_path, style_target));
    if let Err(error) = result {
        let config_rollback = restore_snapshot(config_path, &previous_config);
        let style_rollback = restore_snapshot(style_path, &previous_style);
        return match (config_rollback, style_rollback) {
            (Ok(()), Ok(())) => Err(error).context("falló el cambio de symlinks; se revirtió"),
            (config, style) => bail!(
                "falló el cambio de symlinks ({error:#}); la reversión también falló: config={config:?}, style={style:?}"
            ),
        };
    }
    Ok(())
}

fn replace_symlink(path: &Path, target: &Path) -> Result<()> {
    let parent = path
        .parent()
        .context("la ruta destino no tiene directorio padre")?;
    fs::create_dir_all(parent)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("el reloj del sistema está antes de Unix epoch")?
        .as_nanos();
    let temp_path = parent.join(format!(".aspire-bar-{}-{stamp}.link", std::process::id()));
    symlink(target, &temp_path).with_context(|| {
        format!(
            "no se pudo crear el enlace temporal {}",
            temp_path.display()
        )
    })?;
    if let Err(error) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("no se pudo publicar el enlace {}", path.display()));
    }
    Ok(())
}

fn reload_waybar() -> Result<()> {
    if process_running("waybar") {
        let status = Command::new("pkill")
            .args(["-USR2", "-x", "waybar"])
            .status()
            .context("no se pudo enviar SIGUSR2 a Waybar")?;
        if !status.success() {
            bail!("pkill no pudo enviar SIGUSR2 a Waybar");
        }
        std::thread::sleep(Duration::from_millis(500));
        if process_running("waybar") {
            println!("Waybar recibió SIGUSR2.");
            return Ok(());
        }
    }

    let selector = dirs::home_dir()
        .context("no se pudo obtener HOME")?
        .join(".dots/scripts/Utils/status-bar-selector.sh");
    if !selector.is_file() {
        bail!(
            "Waybar no está activo y no se encontró el selector de barras {}",
            selector.display()
        );
    }
    let status = Command::new("bash")
        .arg(&selector)
        .arg("auto")
        .status()
        .with_context(|| format!("no se pudo iniciar {}", selector.display()))?;
    if !status.success() || !process_running("waybar") {
        bail!("el selector de barras no consiguió iniciar Waybar");
    }
    println!("Waybar se inició mediante el selector de MangoWM.");
    Ok(())
}

fn process_running(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", name])
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symlink_pair_switch_and_rollback_preserve_original_targets() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_root = aspire_bar_dir()
            .unwrap()
            .join(format!("test-symlink-{}-{nonce}", std::process::id()));
        fs::create_dir_all(test_root.join("themes/old")).unwrap();
        fs::create_dir_all(test_root.join("snapshots/new")).unwrap();
        fs::write(test_root.join("themes/old/config.jsonc"), "{}").unwrap();
        fs::write(test_root.join("themes/old/style.css"), "window {}").unwrap();
        fs::write(test_root.join("snapshots/new/config.jsonc"), "[]").unwrap();
        fs::write(
            test_root.join("snapshots/new/style.css"),
            "window#waybar {}",
        )
        .unwrap();
        let config_path = test_root.join("config.jsonc");
        let style_path = test_root.join("style.css");
        symlink("themes/old/config.jsonc", &config_path).unwrap();
        symlink("themes/old/style.css", &style_path).unwrap();

        let original_config = capture_path(&config_path).unwrap();
        let original_style = capture_path(&style_path).unwrap();
        let config_target = test_root.join("snapshots/new/config.jsonc");
        let style_target = test_root.join("snapshots/new/style.css");
        switch_live_pair(&config_path, &config_target, &style_path, &style_target).unwrap();
        assert_eq!(fs::read_link(&config_path).unwrap(), config_target);
        assert_eq!(fs::read_link(&style_path).unwrap(), style_target);

        restore_snapshot(&config_path, &original_config).unwrap();
        restore_snapshot(&style_path, &original_style).unwrap();
        assert_eq!(
            fs::read_link(&config_path).unwrap(),
            PathBuf::from("themes/old/config.jsonc")
        );
        assert_eq!(
            fs::read_link(&style_path).unwrap(),
            PathBuf::from("themes/old/style.css")
        );
        assert_eq!(fs::read(&config_path).unwrap(), b"{}");
        assert_eq!(fs::read(&style_path).unwrap(), b"window {}");
        fs::remove_dir_all(test_root).unwrap();
    }
}
