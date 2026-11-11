# 06 — aspire-files (gestor de archivos)

Gestor de archivos en Rust (GTK4), inspirado en GNOME Files (Nautilus),
**sin libadwaita ni servicios de GNOME**, para compositores Wayland.
Su rasgo distintivo: **el aspecto lo controlan Matugen o Tinty**
(colores inyectados y recargados en caliente).

Repo propio: `~/Proyectos/aspire-files/` (no está dentro de `Mangowm_dots`).

---

## Qué es y estado actual

- Binario único en Rust + GTK4: `crates/aspire-files`.
- Workspace con crates:
  - `aspire-files`: binario GTK4 (ventana, vistas, acciones, tema).
  - `aspire-fs`: capa de archivos (`std::fs`, operaciones, papelera).
  - `aspire-config`: carga/guardado de `config.toml`.
  - `aspire-thumb`: miniaturas (imágenes, PDF, thumbnailers).
- Crates planificados (aún no existen): `aspire-search`, `aspire-mime`,
  `aspire-wallpaper`, `aspire-dbus`.
- Licencia GPL-3.0-or-later. No copia código de Nautilus: reimplementa
  a partir de los docs de `aspire-files-docs/`.

## Funcionalidades implementadas

Navegación: atrás/adelante/subir, breadcrumbs (pathbar + campo de
ubicación `Ctrl+L`), barra lateral (lugares XDG, marcadores, papelera),
pestañas con historial propio, nueva ventana/pestaña.

Vistas: lista virtualizada (`ColumnView`: Nombre/Tamaño/Modificado) y
cuadrícula (`GridView`) con celdas uniformes; alternar `Ctrl+1`/`Ctrl+2`;
ordenar (nombre/tamaño/tipo/fecha, inverso, carpetas primero); tamaños
de icono 48–160.

Operaciones de archivo: abrir (xdg-open; AppImages ejecutables directo),
nueva carpeta, renombrar, copiar/cortar/pegar (portapapeles interno),
papelera, borrado permanente con confirmación, propiedades, “Abrir con…”
(AppChooser + apps comunes para multiselección + guardar predeterminada),
hacer AppImage ejecutable.

Papelera: listar, restaurar, borrar permanentemente, vaciar; la vista
permanece en la papelera.

Búsqueda: en carpeta actual (`Ctrl+F`) y global (`Ctrl+Shift+F`),
debounced, con cancelación por generación; la global omite
`/dev /proc /run /sys`.

Marcadores: añadir carpeta actual/seleccionada, reordenar (arrastrar o
botones), eliminar con confirmación; decodificación porcentual y
recuperación de entradas malformadas.

Preferencias: diálogo persistente en `config.toml` (vista por defecto,
tamaño de icono, ocultos, barra lateral, orden, carpetas primero,
motor/colores de tema).

Tema: Matugen/Tinty vía `~/.config/theme-engine/colors.css`, recarga en
caliente con `notify`; tema oscuro por defecto si falta el archivo.

Miniaturas: imágenes, primera página de PDF (`pdftoppm`), thumbnailers
registrados (audio/vídeo/Office), caché LRU, solo celdas visibles.

## Configuración

`~/.config/aspire-files/config.toml`:

```toml
[theme]
engine      = "auto"          # auto | matugen | tinty | none
colors_file = "~/.config/aspire-files/colors.css"

[general]
folders_first = true
expandable_folders = false
open_action = "double"
optional_actions = ["create_link", "delete_permanently"]

[view]
view_mode = "grid"            # grid | list
icon_size = 128
show_hidden = false
sort_by = "modified"          # name | size | type | modified
sort_descending = false
search_subfolders = false
show_thumbnails = true
count_items_in_folders = true
date_format = "simple"        # simple | detailed

[window]
sidebar_visible = true
```

`engine = "auto"` resuelve a `matugen` leyendo
`~/.config/color-engine/active`.

Marcadores: `~/.config/gtk-3.0/bookmarks` (no hay `gtk-4.0/bookmarks`).

## Compilar y ejecutar

```bash
cd ~/Proyectos/aspire-files
cargo build --release
./target/release/aspire-files [DIRECTORIO]
```

Modos CLI: `--bench`, `--time-open`, `--shot`, `--act`, `--stress`.

Tests: `cargo test --workspace` (3 + 15 + 8 + 8 tests).

## Pendiente / limitaciones conocidas

- Portapapeles interno (una sola instancia).
- Cambios de tema en Preferencias aplican solo tras reiniciar.
- Miniaturas de Office dependen de un thumbnailer compatible instalado.
- Búsqueda global puede ser lenta en discos de red (límites por evaluar).
- Antes de estable: pruebas en el mundo real y ajuste de controles nuevos.
- `desktop.rs` (parser de `.desktop`) eliminado; el lanzamiento de apps
  usa `AppInfo` de GIO.

## Integración con el dotfile

- Lee la paleta de `~/.config/theme-engine/colors.css` (generada por
  `aspire-bg`), igual que el resto de apps.
- No aparece aún en `aspire-menu` (no hay acción `aspire_files`); se
  lanza desde terminal o bind manual.
- No depende de GNOME; usa GTK4 puro + CSS propio (`aspire-base.css`).