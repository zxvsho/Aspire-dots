# aspire-files

Gestor de archivos GTK4 para Wayland, sin libadwaita ni servicios de GNOME.
El aspecto lo controlan Matugen o Tinty mediante
`~/.config/theme-engine/colors.css`, con recarga en caliente.

> **Nota de reconstrucción.** Este árbol se reconstruyó a partir de la
> descripción funcional de `~/.dots/Manual/06-aspire-files.md` después de
> perder el código original. No es una copia byte a byte del proyecto
> anterior: reproduce la estructura, la configuración y el comportamiento
> documentados. Las diferencias conocidas se listan al final.

## Estado

- Workspace Rust con cuatro crates: `aspire-files`, `aspire-fs`,
  `aspire-config` y `aspire-thumb`.
- `cargo check --workspace` y `cargo build --release` pasan.
- `cargo test --workspace` pasa 36 pruebas (8 config, 15 fs, 10 miniaturas,
  3 interfaz).
- Probado en Wayland (MangoWM) con GTK 4.24 y Rust 1.99.

## Estructura

```
aspire-files/
├── Cargo.toml
├── crates/
│   ├── aspire-files/                binario GTK4
│   │   ├── assets/aspire-base.css   CSS propio de la interfaz
│   │   └── src/{main,app,views,dialogs,theme}.rs
│   ├── aspire-fs/                   archivos, búsqueda, marcadores, papelera
│   ├── aspire-config/               config.toml y migración de formato
│   └── aspire-thumb/                miniaturas y caché LRU
└── scripts/install.sh
```

## Dependencias

Obligatorias:

- Rust y Cargo.
- GTK 4 con cabeceras de desarrollo (`gtk4` en Arch).
- `gdk-pixbuf2`.

Opcionales:

- `poppler` (`pdftoppm`) para miniaturas de PDF.
- `ffmpegthumbnailer` para miniaturas de vídeo.
- Cualquier thumbnailer XDG instalado en
  `~/.local/share/thumbnailers` o `/usr/share/thumbnailers` (audio, vídeo,
  Office, RAW…).
- `grim` para el modo `--shot`.

## Compilar y ejecutar

```bash
cd ~/.dots/aspire/aspire-files
cargo build --release
./target/release/aspire-files [DIRECTORIO]
```

Instalar siguiendo la convención de las apps aspire (staging en
`~/.dots/bin` y enlace en `~/.local/bin`):

```bash
./scripts/install.sh
# equivale a:
#   cargo build --release -p aspire-files
#   install -Dm755 target/release/aspire-files ~/.dots/bin/aspire-files
#   ln -sfn ~/.dots/bin/aspire-files ~/.local/bin/aspire-files
```

## CLI

```
aspire-files [DIRECTORIO]

--help, -h              Ayuda
--version, -V           Versión
--bench [DIR]           Mide un listado y sale
--time-open [DIR]       Igual que --bench
--stress [DIR]          Repite el listado 50 veces y reporta tiempos
--shot ARCHIVO          Captura la pantalla con grim y sale
--act open DIR          Abre la interfaz en DIR
--act new-folder DIR    Crea "New Folder" en DIR
```

## Atajos

| Atajo | Acción |
| --- | --- |
| `Ctrl+L` | Campo de ubicación |
| `Ctrl+F` | Buscar en la carpeta actual |
| `Ctrl+Shift+F` | Buscar en subcarpetas |
| `Ctrl+1` / `Ctrl+2` | Vista de cuadrícula / lista |
| `Ctrl+T` / `Ctrl+W` | Nueva pestaña / cerrar pestaña |
| `Ctrl+Shift+N` | Nueva carpeta |
| `F2` | Renombrar |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copiar / cortar / pegar |
| `Ctrl+A` | Seleccionar todo |
| `Delete` | Mover a la papelera |
| `Shift+Delete` | Borrar permanentemente |
| `F5` / `Ctrl+R` | Actualizar |
| `Ctrl+,` | Preferencias |
| `F9` | Mostrar/ocultar barra lateral |
| `Alt+Home` | Ir a Home |

## Configuración

Archivo: `~/.config/aspire-files/config.toml`.

```toml
[theme]
engine      = "auto"          # auto | matugen | tinty | none
colors_file = "~/.config/theme-engine/colors.css"

[general]
folders_first = true
expandable_folders = false
open_action = "double"        # double | single
optional_actions = ["create_link", "delete_permanently"]

[view]
view_mode = "grid"            # grid | list
icon_size = 128               # 48–160
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

`engine = "auto"` resuelve a `matugen` o `tinty` leyendo
`~/.config/color-engine/active`. Si el archivo de colores no existe se usa
una paleta oscura de emergencia.

El formato antiguo con `[preferences]` se migra en memoria al cargar, así que
la configuración existente sigue funcionando sin editarla.

Marcadores: `~/.config/gtk-3.0/bookmarks` (formato Freedesktop, con
decodificación porcentual).

## Tema

- La paleta se lee de `colors_file`.
- El CSS de la interfaz vive en
  `crates/aspire-files/assets/aspire-base.css` y solo usa colores con nombre
  (`@background`, `@foreground`, `@selected`, …).
- La recarga en caliente usa `gio::FileMonitor` sobre el archivo y su
  directorio (equivalente funcional al `notify` documentado).
- Los cambios de tamaño de icono o de disposición se aplican al reiniciar;
  los colores se aplican al instante.

## Funcionalidades

- Navegación con historial por pestaña, breadcrumbs, campo de ubicación,
  barra lateral con lugares XDG, marcadores y papelera.
- Vista de lista virtualizada (`ColumnView`: Nombre/Tamaño/Modificado) y
  cuadrícula (`GridView`) con celdas uniformes, tamaños de icono 48–160.
- Ordenación por nombre, tamaño, tipo o fecha; orden inverso y carpetas
  primero.
- Abrir carpetas/archivos, AppImages ejecutables directamente, nueva carpeta,
  renombrar, copiar/cortar/pegar, borrado permanente con confirmación,
  propiedades, “Abrir con…” para uno o varios archivos (con opción de guardar
  la aplicación predeterminada) y hacer ejecutable.
- Papelera Freedesktop del usuario: mover, listar, restaurar, borrar
  permanentemente y vaciar; la vista permanece en la papelera.
- Búsqueda con debounce y cancelación por generación; la búsqueda global omite
  `/dev`, `/proc`, `/run` y `/sys`.
- Marcadores: añadir la carpeta actual o las carpetas seleccionadas,
  reordenar con botones, eliminar con confirmación.
- Preferencias persistentes en `config.toml`.
- Miniaturas de imágenes, primera página de PDF, vídeo y thumbnailers XDG,
  con caché LRU en `~/.cache/aspire-files/thumbnails`.

## Diferencias y limitaciones conocidas

- **Reconstrucción, no código original.** La lógica se reescribió desde el
  manual; pueden existir detalles de comportamiento distintos.
- La papelera implementada es la del usuario (`~/.local/share/Trash`). No se
  gestionan papeleras de otros volúmenes montados.
- El portapapeles es interno del proceso. Las ventanas creadas con “Nueva
  ventana” no comparten el portapapeles.
- `count_items_in_folders` y `expandable_folders` están en la configuración
  pero todavía no cambian la vista.
- Las miniaturas de audio/vídeo/Office dependen de que haya un thumbnailer
  compatible instalado.
- “Abrir con…” usa `GtkAppChooserWidget`, marcado como obsoleto desde GTK
  4.10 pero todavía funcional; se mantiene para conservar el comportamiento
  documentado.
- `--shot` captura la pantalla completa con `grim`, no solo la ventana.
- No hay entrada en `aspire-menu` ni archivo `.desktop`; se lanza desde
  terminal o con un bind manual, como indica el manual.
