# Aspire BG

Aspire BG es el selector de fondos y la interfaz gráfica para la gestión de
temas del entorno Aspire. Es una aplicación Rust con Relm4 0.11 para el
componente y GTK4/`gtk4-layer-shell` para los widgets y la ventana Wayland. El
código de la paleta y su aplicación al escritorio pertenece a
`aspire-core::theme_engine`; Aspire BG orquesta esa API cuando se cambia un
fondo, se selecciona un motor o se invoca un comando de color.

Este README describe el comportamiento actual y sirve de referencia para
desarrollo y cambios asistidos por IA. La guía de uso está en el
[Manual de Aspire BG](../../Manual/05-aspire-bg.md).

## Responsabilidades y estructura

```text
aspire-bg/
├── Cargo.toml
├── Cargo.lock
├── README.md
└── src/
    ├── main.rs       # CLI, configuración, UI GTK, motores de fondos y flujo de color
    └── wallhaven.rs  # API, búsqueda, miniaturas, descargas y caché de Wallhaven

../core/src/theme_engine.rs  # Matugen, Tinty, paleta y actualización del escritorio
```

La mayor parte de la aplicación vive en `src/main.rs`: configuración, lectura
de archivos, selección, lanzamiento de backends, widgets GTK, controles de
teclado, carrusel, cachés y CLI. `src/wallhaven.rs` solo se compila con la
característica opcional `wallhaven`.

## Requisitos

- Rust 2021 y Cargo.
- Relm4 0.11.
- GTK 4.12 o posterior, `gtk4-layer-shell` y sus dependencias del sistema.
- Sesión Wayland con soporte layer-shell.
- `aspire-core` en `../core`.
- Opcionales según función: `aspire-files` para seleccionar carpetas,
  `notify-send` y un daemon de notificaciones para notificaciones,
  `ffmpeg` para vistas previas animadas de vídeo, Matugen o Tinty según el
  motor de color elegido, y uno o más backends de fondos.
- Para vídeo como fondo se requiere `mpvpaper` o `gslapper`; `mpv` sin
  `mpvpaper` no es un backend de Aspire BG.

Los backends que la aplicación puede reconocer son `swaybg`, `swww`, `awww`,
`wallutils` (`setwallpaper`), `hyprpaper` (`hyprctl`), `mpvpaper`, `gslapper`,
`xwallpaper`, `feh` y `linux-wallpaperengine`. La lista de Options filtra los
que no están disponibles en `PATH`.

## Construcción y pruebas

```bash
cd ~/.dots/aspire/aspire-bg
cargo fmt --check
cargo test
cargo test --features wallhaven
cargo build --release
cargo build --release --features wallhaven
```

Para instalar la compilación con Wallhaven:

```bash
install -m 755 target/release/aspire-bg ~/.local/bin/aspire-bg
```

El perfil `release` activa optimización, LTO, `strip`, una unidad de
generación de código y `panic = "abort"`. La característica `wallhaven`
activa `ureq` con TLS Rustls y `serde_json`; sin ella no se compilan las
dependencias de red ni la interfaz Wallhaven.

Un smoke test con Xvfb puede confirmar que GTK inicia, pero no valida el
comportamiento visual de layer-shell. Los bordes, el centrado y los anclajes
deben comprobarse en un compositor Wayland real.

## Modos de interfaz

### Selector local

- La ventana normal mide aproximadamente `1000×640` y presenta una
  cuadrícula de tres columnas con tarjetas de `300×168`.
- Se muestran como máximo 30 elementos por selección.
- En `Random` se usa muestreo de reservorio para no conservar toda la lista
  en memoria. `Last modified`, `Date added` y `Oldest first` seleccionan los
  30 archivos correspondientes por metadatos.
- No hay un campo de búsqueda local en la interfaz actual. La búsqueda de
  texto corresponde a Wallhaven.
- Un clic activa una tarjeta; `Enter` aplica el elemento enfocado. La
  aplicación permanece abierta tras aplicar el fondo.
- `Random` aplica una imagen de las visibles. `Refresh` vuelve a explorar y
  genera una selección nueva según el orden activo.
- `Folder` ejecuta `aspire-files --select-folder <carpeta-actual>` y espera
  la ruta seleccionada por `stdout`. La elección se guarda y vuelve a
  muestrear la carpeta.
- `Options → Monitor` define el destino del siguiente fondo. Las opciones de
  backend, ajuste y monitor se guardan; cambiar backend o ajuste reaplica la
  asignación actual, mientras que cambiar monitor elige el destino del
  siguiente cambio.

### Carrusel

El carrusel usa la misma selección local: una imagen principal de `520×400` y
cuatro laterales de `80/130/130/80` píxeles. La ventana layer-shell se ancla a
los cuatro bordes en este modo; la fila del carrusel queda centrada. La rueda
desplaza la selección. Un clic en el centro aplica el elemento actual y un
clic en cualquiera de los laterales mueve la selección a ese elemento y lo
aplica. Al aplicar, el modo vuelve a la cuadrícula sin cerrar la aplicación.

`Ctrl+S` alterna cuadrícula/carrusel, `Ctrl+D` vuelve a muestrear el carrusel.
El estado inicial del modo se persiste en `carousel_mode`.

### Wallhaven

Wallhaven se incorpora como segunda página del mismo `GtkStack` y solo existe
con `--features wallhaven`. La UI empieza con orden **Hot**, consulta
categorías `011` (General y Anime), nivel de pureza `100`, proporción `16:9` y
resolución mínima `1920x1080`. Las categorías y la pureza están fijadas en el
código: no se exponen interruptores. La consulta de texto se aplica después
de 350 ms sin cambios.

Los órdenes visibles son **Hot**, **Toplist**, **Uploads**, **Random** y
**Views**. **Refresh** inicia una consulta nueva en `Random`; **More** agrega
la página siguiente usando la página/semilla que devuelve la API. Se deduplica
por ID al añadir páginas. Al activar una tarjeta se descarga el archivo
completo, se aplica y se informa el resultado. `Esc` o `Exit` cierra la
aplicación; no hay una acción de regreso a la página local.

`wallhaven.rs` consulta `https://wallhaven.cc/api/v1/search` con un timeout
global de 15 segundos, usa `thumbs.large` con fallback a `thumbs.small`, y
añade `X-API-Key` solo cuando la clave de configuración no está vacía. La
clave se guarda en texto plano en el INI, por lo que debe protegerse. Los
archivos completos se guardan en
`~/.config/aspire-bg/wallhaven/`; la caché `wh-*` en
`~/.cache/aspire-bg/` se recorta a 200 archivos o 50 MiB.

## Controles de teclado

El `EventControllerKey` se conecta a la ventana en fase de captura y la
ventana usa `KeyboardMode::Exclusive`, para que la navegación funcione sin
tener el puntero sobre una miniatura.

| Control | Comportamiento |
|---|---|
| Flechas | Mueven selección; arriba/abajo avanzan por fila |
| `Alt+H/J/K/L` | Navegan izquierda/abajo/arriba/derecha en todos los modos |
| `Enter` | Aplica el fondo enfocado en el modo local |
| `Ctrl+R` | Actualiza el conjunto local |
| `Ctrl+S` | Alterna cuadrícula/carrusel fuera de Wallhaven |
| `Ctrl+D` | Actualiza el conjunto en carrusel |
| `Ctrl+Z` | Alterna la visibilidad de la barra local y persiste `zen_mode` |
| `Esc` | Cierra Aspire BG; en Wallhaven es la salida del modo |

No documentar letras simples (`h`, `j`, `k`, `l`, `r`, etc.) como atajos:
fueron retiradas para que escribir en el campo de Wallhaven no dispare
acciones de navegación.

## Configuración y datos persistentes

El INI se encuentra en `~/.config/aspire-bg/config.ini`; `ASPIRE_BG_CONFIG`
puede señalar otro archivo. Si el INI nuevo aún no existe, la primera carga
intenta importar `~/.config/waypaper/config.ini`. Se conserva el resto de
claves durante una escritura y el `post_command` que apuntaba a
`cambio_color.sh` se elimina en esa migración.

Claves principales de `[Settings]`:

| Clave | Uso |
|---|---|
| `folder` | Carpeta local |
| `backend` | Backend seleccionado |
| `monitors` | `All` o el conector de salida seleccionado |
| `monitor_wallpapers` | Asignaciones independientes por salida |
| `fill` | Modo de ajuste |
| `sort_mode` | `random`, `modified`, `created` u `oldest` |
| `subfolders`, `show_hidden` | Alcance del escaneo |
| `zen_mode`, `carousel_mode` | Visibilidad de controles y vista inicial |
| `wallpaper` | Fondo principal/actual |
| `color`, `post_command` | Compatibilidad del backend/configuración heredada |

`monitor_wallpapers` serializa pares monitor/ruta separados por comas. Cada
componente escapa bytes no seguros como `%HH`; no cambiar el formato sin
actualizar sus pruebas de ida y vuelta. Elegir `All` borra esas asignaciones.
Al aplicar a un monitor concreto por primera vez, el código inicializa las
otras salidas con el fondo global anterior antes de cambiar solo el destino
seleccionado.

La sección `[wallhaven]` contiene `api_key` y `atleast`. El código normaliza
`categories` a `011` y `purity` a `100`; no tratarlos como opciones editables
de la UI. Otros datos:

- `~/.config/color-engine/active`, `last-wallpaper` y `tinty-theme` son
  estado compartido con `aspire-core`.
- Miniaturas locales y Wallhaven: `~/.cache/aspire-bg/`.
- Descargas completas de Wallhaven: `~/.config/aspire-bg/wallhaven/`.

## Flujo de aplicación de fondos

1. La UI selecciona una ruta local o una descarga completa de Wallhaven.
2. `record_wallpaper_assignment` actualiza el mapa global o la asignación del
   monitor, preservando los fondos de otras salidas.
3. `set_wallpaper` valida compatibilidad y ejecuta el backend. Los procesos
   persistentes que gestiona Aspire BG se detienen antes de iniciar el nuevo
   backend; procesos de vídeo se lanzan desacoplados con `setsid`.
4. La configuración y `color-engine/last-wallpaper` se guardan tras aplicar.
5. Se envía una notificación de escritorio. Las imágenes activan el trabajo
   del motor de color; los vídeos omiten el análisis de paleta.

Compatibilidad de aplicación por monitor:

- `swaybg`, `swww`/`awww`, `hyprpaper`, `mpvpaper` y `gslapper` reciben
  destinos de salida en esta implementación.
- `wallutils`, `linux-wallpaperengine`, `feh` y `xwallpaper` requieren la
  misma imagen en todos los destinos.
- El modo `fill` se traduce a las opciones del backend donde están
  disponibles; algunos backends tienen menos modos de ajuste.

Los errores de backend se devuelven al llamador, se escriben en `stderr` y se
notifican con urgencia crítica. No ocultar un error de aplicación como si
fuera éxito.

## Flujo de miniaturas y concurrencia

- `walk_images` acepta PNG, JPG/JPEG, WebP, BMP, GIF y los formatos de vídeo
  MP4, M4V, WebM, MKV, MOV, AVI y OGV. El escaneo puede ser recursivo y
  filtrar nombres ocultos.
- La vista previa raster se limita a 720 px en su dimensión mayor y se guarda
  como PNG bajo `~/.cache/aspire-bg/`. La clave considera ruta, mtime, tamaño
  y resolución de miniatura.
- Para vídeo, un trabajador ejecuta `ffmpeg` desde 0.5 s, codifica 1.5 s a
  8 fps en GIF y el hilo GTK anima la miniatura seleccionada y el carrusel.
- El trabajo de disco/red se hace fuera del hilo GTK. Los trabajadores envían
  mensajes por canales; callbacks GLib drenan los mensajes y modifican
  widgets solo en el hilo principal.
- `Rc<RefCell<...>>` mantiene el estado de UI en el hilo principal;
  `Arc<Mutex<...>>` y atómicos se reservan para estado compartido con
  trabajadores.
- Los contadores `thumb_gen`, `sample_gen`, `search_gen` y `WhState::gen`
  descartan resultados atrasados después de cambiar carpeta, selección o
  consulta. Mantener estas comprobaciones al modificar carga asíncrona.
- La UI muestra una cuadrícula homogénea; no cargar toda la imagen original
  en cada tarjeta. El almacenamiento en caché y la escala máxima son parte
  del presupuesto de memoria.

## Motor de color compartido

No duplicar generación de paletas en `main.rs`. `aspire-core` es responsable
de Matugen, Tinty, escritura de colores y aplicación a otros componentes.

- `Engine::as_str()` produce los identificadores de máquina minúsculos
  (`matugen`, `tinty`) que se usan en CLI y persistencia. El indicador gráfico
  convierte esos valores a `Matugen`/`Tinty` de forma explícita.
- Matugen se ejecuta en respuesta a cambios de imágenes y escribe la paleta
  canónica; Tinty mantiene la paleta mientras solo cambia el fondo.
- `aspire-core::theme_engine::Palette::apply_system` actualiza los archivos
  presentes para Mango, Mako y Walker, recarga sus servicios/clientes y
  reinicia Waybar únicamente si ya se estaba ejecutando.
- La paleta canónica está en `~/.config/theme-engine/colors.rasi` y
  `colors.css`; otros consumidores del dotfile leen esos archivos o sus
  plantillas.
- `--restore` restaura el fondo sin recalcular la paleta. `--no-colors`
  desactiva el trabajo de color para acciones de aplicación desde CLI.

## Geometría y estilos GTK

La tarjeta `bg-card` debe permanecer como hijo principal de `GtkOverlay`, no
como overlay no medido. El hijo principal participa en la medición del tamaño
de la ventana normal y evita recortar los bordes laterales. El tamaño solicitado
de la tarjeta es `WIN_W × WIN_H`; sus bordes CSS de un píxel forman parte de
la tarjeta.

En carrusel, `set_carousel_mode` ancla la misma ventana a los cuatro bordes
para ocupar la salida y centra la vista en la tarjeta. Si se cambia el árbol
de widgets, verificar ambos modos en un compositor Wayland real: Xvfb solo
sirve como smoke test y no valida layer-shell ni bordes.

Los estilos de `build_css` toman la paleta actual de `aspire-core::colors`.
Mantener los botones de la barra coherentes y el borde Options en un solo nivel:
el contenedor `MenuButton` no dibuja marco; el botón GTK interno dibuja uno.

## CLI

`print_help()` en `src/main.rs` es la fuente de verdad para argumentos y
`main()` define su efecto. Las acciones principales son:

- GUI: sin opción; `--zen-mode` inicia local con la barra oculta.
- Aplicación: `--wallpaper IMG`, `--random`, `--next`, `--restore`.
- Color: `--apply-colors`, `--set-engine matugen|tinty`,
  `--set-tinty-theme NAME`, `--create-tinty-theme [base16|base24]`,
  `--generate-tinty-theme IMG` y `--theme-system base16|base24`.
- Parámetros: `--folder DIR`, `--backend X`, `--monitor X`, `--fill X`.
- Inspección: `--list` (JSON), `--status`, `--help`, `--version`.
- Modificadores: `--no-colors`, `--no-post-command`.

`--restore` reaplica la asignación guardada sin recalcular colores.
`--folder` selecciona la carpeta para esa invocación; los cambios hechos con
el selector de `aspire-files` sí se guardan. `--backend`, `--monitor` y
`--fill` actualizan el INI.

## Guía para cambios asistidos por IA

1. Lee el comportamiento real en `main.rs` y `wallhaven.rs` antes de cambiar
   esta documentación; no reintroduzcas atajos retirados ni búsqueda local.
2. Mantén `wallhaven` compilable y opcional; usa `#[cfg(feature =
   "wallhaven")]` para símbolos y red de esa funcionalidad.
3. No muevas operaciones GTK a hilos de trabajo. Envía datos por canales y
   modifica widgets desde GLib en el hilo principal.
4. Conserva los números de generación que previenen aplicar resultados
   obsoletos.
5. Mantén `monitor_wallpapers` compatible con Unicode, comas, `=` y escapes
   `%HH`; actualiza las pruebas si cambia el formato.
6. No cambies `Engine::as_str()` para resolver capitalización visual: es un
   identificador usado por configuración y CLI. Ajusta únicamente el texto de
   presentación.
7. Conserva los comportamientos de CLI `--restore`, la migración de Waypaper
   y la distinción entre backend de fondo y motor de color.
8. Al cambiar geometría, CSS, anclajes o selección, valida cuadrícula,
   carrusel, foco de teclado, aplicación por monitor y ejecución con y sin
   `wallhaven`.

## Validación de cambios

```bash
cargo fmt --check
cargo test
cargo test --features wallhaven
cargo build --release
cargo build --release --features wallhaven
```

La capa visual requiere una comprobación final en Wayland. Para cambios de
geometría, no considerar una ejecución Xvfb como evidencia de que los bordes,
el centrado o los controles se ven correctamente.
