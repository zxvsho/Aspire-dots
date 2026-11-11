use crate::apply;
use crate::document::BarDocument;
use crate::palette::{self, DYNAMIC_ID};
use crate::templates;
use anyhow::{bail, Context, Result};
use gtk::gio::prelude::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_THEME_FILE: u64 = 8 * 1024 * 1024;
const MAX_THEME_SIZE: u64 = 24 * 1024 * 1024;
const PACKAGE_FILES: [&str; 4] = ["theme.toml", "config.jsonc", "style.css", "preview.png"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThemeManifest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    #[serde(default = "default_palette")]
    pub palette: String,
    #[serde(default, rename = "required-modules")]
    pub required_modules: Vec<String>,
    #[serde(default, rename = "optional-dependencies")]
    pub optional_dependencies: Vec<String>,
    #[serde(default, rename = "allow-fixed-colors")]
    pub allow_fixed_colors: bool,
}

#[derive(Clone, Debug)]
pub struct ThemePackage {
    pub manifest: ThemeManifest,
    pub document: BarDocument,
    pub path: PathBuf,
    pub is_default: bool,
}

/// Tema presente en disco que no se pudo cargar. Se muestra en la galería con
/// el motivo en lugar de desaparecer en silencio.
#[derive(Clone, Debug)]
pub struct InvalidTheme {
    pub slug: String,
    pub reason: String,
}

#[derive(Deserialize)]
struct DefaultCatalog {
    theme: Vec<DefaultTheme>,
}

#[derive(Deserialize)]
struct DefaultTheme {
    name: String,
    #[serde(rename = "display-name")]
    display_name: String,
    description: String,
    palette: String,
    template: String,
    radius: u16,
    gap: u16,
    padding: u16,
}

fn default_palette() -> String {
    DYNAMIC_ID.to_owned()
}

pub fn package_root() -> Result<PathBuf> {
    let home = dirs::home_dir().context("no se pudo determinar HOME")?;
    Ok(home.join(".dots/assets/waybar-themes"))
}

pub fn user_root() -> Result<PathBuf> {
    Ok(config_theme_root()?.join("themes"))
}

fn config_theme_root() -> Result<PathBuf> {
    let config = dirs::config_dir().context("no se pudo determinar XDG_CONFIG_HOME")?;
    Ok(config.join("aspire-bar"))
}

pub fn list_packages() -> Result<Vec<ThemePackage>> {
    Ok(list_all()?.0)
}

/// Devuelve los temas válidos junto con los que no se pudieron cargar.
pub fn list_all() -> Result<(Vec<ThemePackage>, Vec<InvalidTheme>)> {
    let mut packages = default_packages()?;
    let root = user_root()?;
    if !root.is_dir() {
        return Ok((packages, Vec::new()));
    }
    let mut custom = Vec::new();
    let mut invalid = Vec::new();
    for entry in
        fs::read_dir(&root).with_context(|| format!("no se pudo listar {}", root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        if slug.starts_with('.') {
            continue;
        }
        match read_package(&entry.path()) {
            Ok(package) => custom.push(package),
            Err(error) => invalid.push(InvalidTheme {
                slug,
                reason: format!("{error:#}"),
            }),
        }
    }
    custom.sort_by(|left, right| left.manifest.display_name.cmp(&right.manifest.display_name));
    invalid.sort_by(|left, right| left.slug.cmp(&right.slug));
    packages.extend(custom);
    Ok((packages, invalid))
}

pub fn read_package(path: &Path) -> Result<ThemePackage> {
    read_package_inner(path, true, false)
}

fn read_package_inner(
    path: &Path,
    match_directory: bool,
    is_default: bool,
) -> Result<ThemePackage> {
    let manifest_path = path.join("theme.toml");
    let manifest_source = fs::read_to_string(&manifest_path)
        .with_context(|| format!("no se pudo leer {}", manifest_path.display()))?;
    let mut manifest: ThemeManifest = toml::from_str(&manifest_source)
        .with_context(|| format!("theme.toml inválido en {}", path.display()))?;
    if manifest.display_name.is_empty() {
        manifest.display_name = manifest.name.replace('-', " ");
    }
    validate_name(&manifest.name)?;
    if match_directory && path.file_name().and_then(|name| name.to_str()) != Some(&manifest.name) {
        bail!("el directorio y el nombre de theme.toml no coinciden");
    }
    let mut document = BarDocument::read(&path.join("config.jsonc"), &path.join("style.css"))?;
    apply_manifest_palette(&mut document, &manifest)?;
    validate_manifest(&manifest, &document)?;
    Ok(ThemePackage {
        manifest,
        document,
        path: path.to_owned(),
        is_default,
    })
}

fn default_packages() -> Result<Vec<ThemePackage>> {
    let catalog_path = package_root()?.join("defaults.toml");
    let source = fs::read_to_string(&catalog_path)
        .with_context(|| format!("no se pudo leer {}", catalog_path.display()))?;
    let catalog: DefaultCatalog = toml::from_str(&source)
        .with_context(|| format!("catálogo de temas inválido: {}", catalog_path.display()))?;
    let shared_path = package_root()?.join("minimal");
    let mut packages = Vec::with_capacity(catalog.theme.len());
    for preset in catalog.theme {
        let template = templates::find(&preset.template)?;
        let mut document = BarDocument::read(
            &shared_path.join("config.jsonc"),
            &shared_path.join("style.css"),
        )?;
        document.config = template.config;
        document.style_settings.use_aspire_palette = Some(preset.palette != DYNAMIC_ID);
        document.style_settings.palette_id = Some(preset.palette.clone());
        document.style_settings.palette_name = if preset.palette == DYNAMIC_ID {
            None
        } else {
            Some(palette::find_by_id(&preset.palette)?.name)
        };
        document.style_settings.border_radius = Some(preset.radius);
        document.style_settings.module_padding_vertical = Some(0);
        document.style_settings.module_padding_horizontal = Some(preset.padding);
        if let Some(bar) = document.config.bar_mut(0) {
            bar.spacing = Some(serde_json::Value::from(preset.gap));
        }
        let required_modules = document
            .config
            .bars()
            .iter()
            .flat_map(|bar| bar.zones())
            .flatten()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let manifest = ThemeManifest {
            name: preset.name,
            display_name: preset.display_name,
            author: "Mangowm_dots".to_owned(),
            description: preset.description,
            version: "1.0.0".to_owned(),
            palette: preset.palette,
            required_modules,
            optional_dependencies: Vec::new(),
            allow_fixed_colors: true,
        };
        validate_manifest(&manifest, &document)?;
        packages.push(ThemePackage {
            manifest,
            document,
            path: shared_path.clone(),
            is_default: true,
        });
    }
    let island_path = package_root()?.join("dynamic-island");
    let mut island = read_package_inner(&island_path, true, true)?;
    island.manifest.display_name = "Isla dinámica".to_owned();
    packages.push(island);
    Ok(packages)
}

fn apply_manifest_palette(document: &mut BarDocument, manifest: &ThemeManifest) -> Result<()> {
    if manifest.palette == DYNAMIC_ID {
        document.style_settings.use_aspire_palette = Some(true);
        document.style_settings.palette_id = Some(DYNAMIC_ID.to_owned());
        document.style_settings.palette_name = None;
        document.style_settings.palette_colors.clear();
    } else {
        let palette = palette::find_by_id(&manifest.palette)?;
        document.style_settings.use_aspire_palette = Some(false);
        document.style_settings.palette_id = Some(palette.id);
        document.style_settings.palette_name = Some(palette.name);
        document.style_settings.palette_colors.clear();
    }
    Ok(())
}

pub fn validate_manifest(manifest: &ThemeManifest, document: &BarDocument) -> Result<()> {
    validate_name(&manifest.name)?;
    if manifest.author.trim().is_empty()
        || manifest.description.trim().is_empty()
        || manifest.version.trim().is_empty()
    {
        bail!("el tema necesita autor, descripción y versión");
    }
    let mut validated_document = document.clone();
    validated_document.use_dynamic_palette_when_unspecified();
    apply::validate_document(&validated_document)?;
    let modules = document
        .config
        .bars()
        .iter()
        .flat_map(|bar| bar.zones())
        .flatten()
        .collect::<Vec<_>>();
    for required in &manifest.required_modules {
        if !modules.iter().any(|module| module == required) {
            bail!("el tema declara el módulo requerido «{required}», pero no lo configura");
        }
    }
    if manifest.palette != DYNAMIC_ID {
        palette::find_by_id(&manifest.palette)?;
    }
    let has_fixed_palette = manifest.palette != DYNAMIC_ID
        || (document.style_settings.use_aspire_palette == Some(false)
            && !document.style_settings.palette_colors.is_empty());
    if has_fixed_palette && !manifest.allow_fixed_colors {
        bail!("la paleta fija requiere allow-fixed-colors = true en theme.toml");
    }
    if !manifest.allow_fixed_colors && contains_fixed_hex_color(&document.style) {
        bail!("style.css contiene colores hexadecimales fijos; decláralos en theme.toml o usa tokens Aspire");
    }
    Ok(())
}

pub fn create_theme(manifest: &ThemeManifest, document: &BarDocument) -> Result<PathBuf> {
    let mut document = document.clone();
    apply_manifest_palette(&mut document, manifest)?;
    let mut manifest = manifest.clone();
    if manifest.palette != DYNAMIC_ID
        || (document.style_settings.use_aspire_palette == Some(false)
            && !document.style_settings.palette_colors.is_empty())
    {
        manifest.allow_fixed_colors = true;
    }
    validate_manifest(&manifest, &document)?;
    let config = document.generated_config()?;
    let style = document.generated_style()?;
    create_package_files(&manifest, &config, &style, None)
}

pub fn duplicate_theme(source_name: &str, new_name: &str) -> Result<PathBuf> {
    validate_name(new_name)?;
    let source = find_package(source_name)?;
    let mut manifest = source.manifest;
    manifest.name = new_name.to_owned();
    manifest.display_name = new_name.replace('-', " ");
    manifest.description = format!("Copia personalizable de {}", manifest.description);
    create_theme(&manifest, &source.document)
}

pub fn slugify(display_name: &str) -> Result<String> {
    let slug = display_name
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
    validate_name(&slug)?;
    Ok(slug)
}

pub fn rename_theme(name: &str, new_name: &str, display_name: &str) -> Result<PathBuf> {
    validate_name(new_name)?;
    if display_name.trim().is_empty() {
        bail!("el tema necesita un nombre visible");
    }
    let package = find_package(name)?;
    if package.is_default {
        bail!("los temas predeterminados son de solo lectura; duplica primero el tema");
    }
    let root = user_root()?;
    let source = root.join(name);
    let target = root.join(new_name);
    if source != target && target.exists() {
        bail!("ya existe un tema con el nombre «{new_name}»");
    }
    let mut manifest = package.manifest;
    manifest.name = new_name.to_owned();
    manifest.display_name = display_name.trim().to_owned();
    let temp_manifest = source.join(".theme.toml.tmp");
    fs::write(&temp_manifest, toml::to_string_pretty(&manifest)?)
        .with_context(|| format!("no se pudo escribir {}", temp_manifest.display()))?;
    if source != target {
        if let Err(error) = fs::rename(&source, &target) {
            let _ = fs::remove_file(&temp_manifest);
            return Err(error)
                .with_context(|| format!("no se pudo renombrar {}", source.display()));
        }
    }
    let manifest_path = target.join("theme.toml");
    if let Err(error) = fs::rename(target.join(".theme.toml.tmp"), &manifest_path) {
        if source != target {
            let _ = fs::rename(&target, &source);
        }
        return Err(error)
            .with_context(|| format!("no se pudo publicar {}", manifest_path.display()));
    }
    if active_theme()?.as_deref() == Some(name) {
        set_active_theme(new_name)?;
    }
    Ok(target)
}

pub fn delete_theme(name: &str) -> Result<()> {
    let package = find_package(name)?;
    if package.is_default {
        bail!("los temas predeterminados no se pueden eliminar");
    }
    if active_theme()?.as_deref() == Some(name) {
        bail!("no se puede eliminar el tema activo");
    }
    let path = user_root()?.join(name);
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("no se pudo inspeccionar {}", path.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!(
            "la ruta del tema no es un directorio regular: {}",
            path.display()
        );
    }
    fs::remove_dir_all(&path).with_context(|| format!("no se pudo eliminar {}", path.display()))
}

/// Mueve el tema a la papelera del sistema; si el escritorio no la ofrece,
/// borra el directorio (la UI ya pidió confirmación explícita).
pub fn trash_theme(name: &str) -> Result<()> {
    let package = find_package(name)?;
    if package.is_default {
        bail!("los temas predeterminados no se pueden eliminar");
    }
    if active_theme()?.as_deref() == Some(name) {
        bail!("no se puede eliminar el tema activo");
    }
    let path = user_root()?.join(name);
    let metadata = fs::symlink_metadata(&path)
        .with_context(|| format!("no se pudo inspeccionar {}", path.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!(
            "la ruta del tema no es un directorio regular: {}",
            path.display()
        );
    }
    let file = gtk::gio::File::for_path(&path);
    if file.trash(gtk::gio::Cancellable::NONE).is_ok() {
        return Ok(());
    }
    fs::remove_dir_all(&path).with_context(|| format!("no se pudo eliminar {}", path.display()))
}

fn hidden_path() -> Result<PathBuf> {
    Ok(config_theme_root()?.join("hidden-themes"))
}

/// Nombres de los temas predeterminados ocultos (nunca toca el repo).
pub fn hidden_themes() -> Vec<String> {
    let Ok(path) = hidden_path() else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

pub fn is_hidden(name: &str) -> bool {
    hidden_themes().iter().any(|hidden| hidden == name)
}

pub fn set_hidden(name: &str, hidden: bool) -> Result<()> {
    validate_name(name)?;
    let mut names = hidden_themes();
    names.retain(|existing| existing != name);
    if hidden {
        names.push(name.to_owned());
    }
    names.sort();
    names.dedup();
    let mut body = names.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    atomic_write(&hidden_path()?, body.as_bytes())
}

pub fn restore_all_defaults() -> Result<()> {
    atomic_write(&hidden_path()?, b"")
}

/// Crea `themes/README` la primera vez que se abre la galería.
pub fn ensure_readme() -> Result<()> {
    let root = user_root()?;
    fs::create_dir_all(&root).with_context(|| format!("no se pudo crear {}", root.display()))?;
    let path = root.join("README");
    if path.is_file() {
        return Ok(());
    }
    let text = r#"# Temas de aspire-bar

Cada tema vive en su propia carpeta: `themes/<slug>/` con estos archivos:

- `theme.toml`   — metadatos (name, display-name, author, description, version, palette).
- `config.jsonc` — configuración de Waybar.
- `style.css`    — hoja de estilos de Waybar.
- `preview.png`  — opcional, miniatura.

`name` debe usar minúsculas, números y guiones, y coincidir con el nombre de la carpeta.
`palette` puede ser `dynamic` (wallpaper) o el id de una paleta incluida.

Para importar: usa los botones de la sección Temas (carpeta, `.tar.zst`, `.zip`
o dos archivos sueltos). Los temas copiados a mano aparecen automáticamente.
"#;
    fs::write(&path, text).with_context(|| format!("no se pudo escribir {}", path.display()))
}

/// Crea un tema a partir de dos archivos sueltos (`config`/`config.jsonc` + `style.css`).
pub fn import_files(config: &Path, style: &Path, display_name: &str) -> Result<PathBuf> {
    let config_text = fs::read_to_string(config)
        .with_context(|| format!("no se pudo leer {}", config.display()))?;
    let style_text = fs::read_to_string(style)
        .with_context(|| format!("no se pudo leer {}", style.display()))?;
    let mut document = BarDocument::from_config_source(&config_text, style_text)?;
    document.use_dynamic_palette_when_unspecified();
    validate_document_pair(&document)?;
    let slug = slugify(display_name)?;
    let manifest = ThemeManifest {
        name: slug,
        display_name: display_name.trim().to_owned(),
        author: "Importado".to_owned(),
        description: format!("Tema importado desde {}", config.display()),
        version: "1.0.0".to_owned(),
        palette: DYNAMIC_ID.to_owned(),
        required_modules: Vec::new(),
        optional_dependencies: Vec::new(),
        allow_fixed_colors: true,
    };
    create_theme(&manifest, &document)
}

fn validate_document_pair(document: &BarDocument) -> Result<()> {
    apply::validate_document(document)
}

/// Lista los nombres de los temas predeterminados incluidos en el repo.
pub fn default_theme_names() -> Result<Vec<String>> {
    Ok(default_packages()?
        .into_iter()
        .map(|package| package.manifest.name)
        .collect())
}

pub fn set_theme_palette(name: &str, palette_id: &str) -> Result<()> {
    if palette_id != DYNAMIC_ID {
        palette::find_by_id(palette_id)?;
    }
    let package = find_package(name)?;
    if package.is_default {
        bail!("duplica el tema predeterminado para cambiar su paleta");
    }
    let mut manifest = package.manifest;
    manifest.palette = palette_id.to_owned();
    let root = user_root()?;
    let path = root.join(name).join("theme.toml");
    atomic_write(&path, toml::to_string_pretty(&manifest)?.as_bytes())
}

pub fn active_theme() -> Result<Option<String>> {
    let path = config_theme_root()?.join("active-theme");
    match fs::read_to_string(&path) {
        Ok(name) => {
            let name = name.trim().to_owned();
            Ok((!name.is_empty()).then_some(name))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("no se pudo leer {}", path.display())),
    }
}

pub fn set_active_theme(name: &str) -> Result<()> {
    validate_name(name)?;
    let path = config_theme_root()?.join("active-theme");
    atomic_write(&path, format!("{name}\n").as_bytes())
}

pub fn find_package(name: &str) -> Result<ThemePackage> {
    validate_name(name)?;
    if let Some(package) = default_packages()?
        .into_iter()
        .find(|package| package.manifest.name == name)
    {
        return Ok(package);
    }
    read_package(&user_root()?.join(name))
        .with_context(|| format!("no se pudo cargar el paquete «{name}»"))
}

pub fn export_theme(name: &str, destination: &Path) -> Result<PathBuf> {
    let package = find_package(name)?;
    let files = [
        ("theme.toml", toml::to_string_pretty(&package.manifest)?),
        ("config.jsonc", package.document.generated_config()?),
        ("style.css", package.document.generated_style()?),
    ];
    let is_archive = destination
        .file_name()
        .and_then(|file| file.to_str())
        .is_some_and(|file| file.ends_with(".tar.zst"));
    if is_archive {
        if destination.exists() {
            bail!("el destino ya existe: {}", destination.display());
        }
        let parent = destination
            .parent()
            .context("el destino no tiene directorio padre")?;
        fs::create_dir_all(parent)?;
        let file = File::create(destination)
            .with_context(|| format!("no se pudo crear {}", destination.display()))?;
        let encoder = zstd::stream::Encoder::new(file, 3)?;
        let mut archive = tar::Builder::new(encoder);
        for (file_name, contents) in &files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive.append_data(
                &mut header,
                Path::new(&package.manifest.name).join(file_name),
                contents.as_bytes(),
            )?;
        }
        archive
            .into_inner()?
            .finish()
            .context("no se pudo finalizar el archivo tar.zst")?;
        return Ok(destination.to_owned());
    }

    let destination = if destination.is_dir() {
        destination.join(&package.manifest.name)
    } else {
        destination.to_owned()
    };
    if destination.exists() {
        bail!("el destino ya existe: {}", destination.display());
    }
    fs::create_dir_all(&destination)
        .with_context(|| format!("no se pudo crear {}", destination.display()))?;
    for (file_name, contents) in &files {
        atomic_write(&destination.join(file_name), contents.as_bytes())?;
    }
    Ok(destination)
}

pub fn import_theme(source: &Path) -> Result<PathBuf> {
    let files = if source.is_dir() {
        read_theme_directory(source)?
    } else {
        read_theme_archive(source)?
    };
    let manifest_bytes = files
        .get("theme.toml")
        .context("el paquete no contiene theme.toml")?;
    let manifest_source = std::str::from_utf8(manifest_bytes).context("theme.toml no es UTF-8")?;
    let mut manifest: ThemeManifest =
        toml::from_str(manifest_source).context("theme.toml inválido")?;
    if manifest.display_name.is_empty() {
        manifest.display_name = manifest.name.replace('-', " ");
    }
    validate_name(&manifest.name)?;
    let config = files
        .get("config.jsonc")
        .context("el paquete no contiene config.jsonc")?;
    let style = files
        .get("style.css")
        .context("el paquete no contiene style.css")?;
    let document = BarDocument::from_config_source(
        std::str::from_utf8(config).context("config.jsonc no es UTF-8")?,
        std::str::from_utf8(style)
            .context("style.css no es UTF-8")?
            .to_owned(),
    )?;
    validate_manifest(&manifest, &document)?;

    let root = user_root()?;
    fs::create_dir_all(&root).with_context(|| format!("no se pudo crear {}", root.display()))?;
    let target = root.join(&manifest.name);
    if target.exists() {
        bail!("ya existe un tema con ese nombre: {}", target.display());
    }
    let staging = staging_path(&root)?;
    let write_result = (|| {
        fs::create_dir(&staging)?;
        for (name, contents) in &files {
            fs::write(staging.join(name), contents)
                .with_context(|| format!("no se pudo escribir {name}"))?;
        }
        read_package_inner(&staging, false, false)?;
        fs::rename(&staging, &target)
            .with_context(|| format!("no se pudo publicar {}", target.display()))?;
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = write_result {
        if staging.is_dir() {
            fs::remove_dir_all(&staging)
                .with_context(|| format!("no se pudo limpiar {}", staging.display()))?;
        }
        return Err(error);
    }
    Ok(target)
}

fn create_package_files(
    manifest: &ThemeManifest,
    config: &str,
    style: &str,
    preview: Option<&[u8]>,
) -> Result<PathBuf> {
    let root = user_root()?;
    fs::create_dir_all(&root).with_context(|| format!("no se pudo crear {}", root.display()))?;
    let target = root.join(&manifest.name);
    if target.exists() {
        bail!("ya existe un tema con ese nombre: {}", target.display());
    }
    let staging = staging_path(&root)?;
    let write_result = (|| {
        fs::create_dir(&staging)?;
        fs::write(
            staging.join("theme.toml"),
            toml::to_string_pretty(manifest)?,
        )?;
        fs::write(staging.join("config.jsonc"), config)?;
        fs::write(staging.join("style.css"), style)?;
        if let Some(preview) = preview {
            fs::write(staging.join("preview.png"), preview)?;
        }
        read_package_inner(&staging, false, false)?;
        fs::rename(&staging, &target)
            .with_context(|| format!("no se pudo publicar {}", target.display()))?;
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = write_result {
        if staging.is_dir() {
            fs::remove_dir_all(&staging)
                .with_context(|| format!("no se pudo limpiar {}", staging.display()))?;
        }
        return Err(error);
    }
    Ok(target)
}

fn read_theme_directory(source: &Path) -> Result<std::collections::BTreeMap<String, Vec<u8>>> {
    let package = if source.join("theme.toml").is_file() {
        source.to_owned()
    } else {
        let mut candidates = Vec::new();
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            if entry.path().join("theme.toml").is_file() {
                candidates.push(entry.path());
            }
        }
        if candidates.len() != 1 {
            bail!("selecciona la carpeta del tema o una carpeta que contenga exactamente un tema");
        }
        candidates[0].clone()
    };
    let mut files = std::collections::BTreeMap::new();
    let mut total = 0_u64;
    for name in PACKAGE_FILES {
        let path = package.join(name);
        if !path.is_file() {
            if name == "preview.png" {
                continue;
            }
            bail!("falta {name} en {}", package.display());
        }
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_THEME_FILE {
            bail!("archivo de tema no válido: {}", path.display());
        }
        total += metadata.len();
        if total > MAX_THEME_SIZE {
            bail!("el paquete supera el tamaño máximo permitido");
        }
        files.insert(name.to_owned(), fs::read(&path)?);
    }
    Ok(files)
}

fn read_theme_archive(source: &Path) -> Result<std::collections::BTreeMap<String, Vec<u8>>> {
    let is_zip = source
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    if is_zip {
        return read_zip_archive(source);
    }
    let file =
        File::open(source).with_context(|| format!("no se pudo abrir {}", source.display()))?;
    let decoder = zstd::stream::Decoder::new(file)
        .with_context(|| format!("no es un archivo zstd válido: {}", source.display()))?;
    let mut archive = tar::Archive::new(decoder);
    let mut files = std::collections::BTreeMap::new();
    let mut package_root: Option<String> = None;
    let mut total = 0_u64;
    for entry in archive.entries().context("tar inválido")? {
        let entry = entry.context("entrada tar dañada")?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        if !entry.header().entry_type().is_file() {
            bail!("el paquete contiene enlaces o entradas especiales no permitidas");
        }
        let path = entry
            .path()
            .context("ruta de archivo inválida en tar")?
            .into_owned();
        if path.to_string_lossy().contains('\\') {
            bail!("el paquete contiene un separador de ruta no seguro");
        }
        let components = path.components().collect::<Vec<_>>();
        if components.is_empty()
            || components
                .iter()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            bail!("el paquete contiene una ruta no segura");
        }
        let (root, file_name) = match components.as_slice() {
            [Component::Normal(file_name)] => (None, file_name.to_string_lossy().to_string()),
            [Component::Normal(root), Component::Normal(file_name)] => (
                Some(root.to_string_lossy().to_string()),
                file_name.to_string_lossy().to_string(),
            ),
            _ => bail!("el paquete incluye carpetas internas no permitidas"),
        };
        if let Some(root) = root {
            if package_root
                .as_ref()
                .is_some_and(|current| current != &root)
            {
                bail!("el archivo contiene más de un tema");
            }
            package_root = Some(root);
        } else if package_root.is_some() {
            bail!("el archivo mezcla rutas raíz y carpeta de tema");
        }
        if !PACKAGE_FILES.contains(&file_name.as_str()) {
            bail!("archivo no admitido en el tema: {file_name}");
        }
        let file_size = entry.header().size().context("tamaño tar inválido")?;
        total += file_size;
        if file_size > MAX_THEME_FILE || total > MAX_THEME_SIZE {
            bail!("el paquete supera el tamaño máximo permitido");
        }
        let mut contents = Vec::with_capacity(file_size as usize);
        entry
            .take(MAX_THEME_FILE + 1)
            .read_to_end(&mut contents)
            .context("no se pudo leer una entrada del paquete")?;
        if contents.len() as u64 != file_size {
            bail!("el tamaño de una entrada tar no coincide");
        }
        if files.insert(file_name.clone(), contents).is_some() {
            bail!("el paquete repite el archivo {file_name}");
        }
    }
    Ok(files)
}

fn read_zip_archive(source: &Path) -> Result<std::collections::BTreeMap<String, Vec<u8>>> {
    let listing = std::process::Command::new("unzip")
        .args(["-Z1"])
        .arg(source)
        .output()
        .with_context(|| format!("no se pudo listar {}", source.display()))?;
    if !listing.status.success() {
        bail!(
            "no es un archivo zip válido: {} ({})",
            source.display(),
            String::from_utf8_lossy(&listing.stderr).trim()
        );
    }
    let mut files = std::collections::BTreeMap::new();
    let mut package_root: Option<String> = None;
    let mut total = 0_u64;
    for entry in String::from_utf8_lossy(&listing.stdout).lines() {
        let entry = entry.trim();
        if entry.is_empty() || entry.ends_with('/') || entry.ends_with('\\') {
            continue;
        }
        if entry.contains('\\') {
            bail!("el paquete contiene un separador de ruta no seguro");
        }
        let components = Path::new(entry).components().collect::<Vec<_>>();
        if components.is_empty()
            || components
                .iter()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            bail!("el paquete contiene una ruta no segura");
        }
        let (root, file_name) = match components.as_slice() {
            [Component::Normal(file_name)] => (None, file_name.to_string_lossy().to_string()),
            [Component::Normal(root), Component::Normal(file_name)] => (
                Some(root.to_string_lossy().to_string()),
                file_name.to_string_lossy().to_string(),
            ),
            _ => bail!("el paquete incluye carpetas internas no permitidas"),
        };
        if !PACKAGE_FILES.contains(&file_name.as_str()) {
            // Ignora metadatos típicos del zip (__MACOSX, .DS_Store).
            if file_name.starts_with('.') || components.len() == 1 {
                continue;
            }
            bail!("archivo no admitido en el tema: {file_name}");
        }
        if let Some(root) = root {
            if package_root
                .as_ref()
                .is_some_and(|current| current != &root)
            {
                bail!("el archivo contiene más de un tema");
            }
            package_root = Some(root);
        } else if package_root.is_some() {
            bail!("el archivo mezcla rutas raíz y carpeta de tema");
        }
        let output = std::process::Command::new("unzip")
            .args(["-p"])
            .arg(source)
            .arg(entry)
            .output()
            .with_context(|| format!("no se pudo leer {entry} de {}", source.display()))?;
        if !output.status.success() {
            bail!("no se pudo extraer {entry}");
        }
        if output.stdout.len() as u64 > MAX_THEME_FILE {
            bail!("archivo de tema demasiado grande: {entry}");
        }
        total += output.stdout.len() as u64;
        if total > MAX_THEME_SIZE {
            bail!("el paquete supera el tamaño máximo permitido");
        }
        if files.insert(file_name.clone(), output.stdout).is_some() {
            bail!("el paquete repite el archivo {file_name}");
        }
    }
    Ok(files)
}

fn staging_path(root: &Path) -> Result<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("el reloj del sistema está antes de Unix epoch")?
        .as_nanos();
    Ok(root.join(format!(".aspire-bar-import-{}-{nonce}", std::process::id())))
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .context("la ruta destino no tiene directorio padre")?;
    fs::create_dir_all(parent).with_context(|| format!("no se pudo crear {}", parent.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("el reloj del sistema está antes de Unix epoch")?
        .as_nanos();
    let temp = parent.join(format!(
        ".aspire-bar-theme-{}-{nonce}.tmp",
        std::process::id()
    ));
    fs::write(&temp, contents)
        .with_context(|| format!("no se pudo escribir {}", temp.display()))?;
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(error).with_context(|| format!("no se pudo publicar {}", path.display()));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || !name.as_bytes()[0].is_ascii_lowercase()
    {
        bail!("nombre de tema inválido: usa minúsculas, números y guiones");
    }
    Ok(())
}

fn contains_fixed_hex_color(css: &str) -> bool {
    let bytes = css.as_bytes();
    bytes.iter().enumerate().any(|(index, byte)| {
        if *byte != b'#' {
            return false;
        }
        let length = bytes[index + 1..]
            .iter()
            .take_while(|character| character.is_ascii_hexdigit())
            .count();
        matches!(length, 3 | 4 | 6 | 8)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_theme_packages_parse_and_validate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/waybar-themes")
            .canonicalize()
            .unwrap();
        assert!(root.join("defaults.toml").is_file());
        let packages = default_packages().unwrap();
        assert_eq!(packages.len(), 9);
        let expected = [
            ("catppuccin-mocha", 12, 4, 10),
            ("catppuccin-latte", 12, 4, 10),
            ("tokyo-night", 8, 4, 8),
            ("gruvbox-dark", 6, 2, 8),
            ("dracula", 10, 4, 10),
            ("nord", 8, 4, 8),
            ("rose-pine", 14, 6, 12),
            ("everforest", 10, 4, 10),
        ];
        for package in packages {
            assert!(package.is_default);
            assert!(package.document.style_settings.palette_id.is_some());
            assert!(package.document.generated_style().is_ok());
            if package.manifest.name == "dynamic-island" {
                assert_eq!(package.manifest.palette, DYNAMIC_ID);
                assert!(!package.manifest.allow_fixed_colors);
                assert_eq!(package.manifest.display_name, "Isla dinámica");
                continue;
            }
            assert!(package.manifest.allow_fixed_colors);
            let (_, radius, gap, padding) = expected
                .iter()
                .find(|(name, _, _, _)| *name == package.manifest.name)
                .unwrap();
            assert_eq!(package.manifest.palette, package.manifest.name);
            assert_eq!(package.document.style_settings.border_radius, Some(*radius));
            assert_eq!(
                package.document.style_settings.module_padding_horizontal,
                Some(*padding)
            );
            assert_eq!(
                package.document.config.bars()[0]
                    .spacing
                    .as_ref()
                    .and_then(serde_json::Value::as_u64),
                Some(*gap as u64)
            );
        }
    }

    #[test]
    fn display_names_produce_safe_theme_slugs() {
        assert_eq!(slugify("Tokyo Night 2").unwrap(), "tokyo-night-2");
        assert!(slugify("...").is_err());
    }
}
