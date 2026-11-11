# Aspire BG: selector de fondos y temas

Aspire BG es el selector de fondos de pantalla del entorno Aspire. Permite
aplicar imágenes o vídeos, mantener asignaciones distintas por monitor, navegar
por una vista de carrusel y, cuando el motor activo es Matugen, actualizar la
paleta del escritorio a partir del fondo. El componente de interfaz usa
Relm4 0.11 sobre GTK4 y layer-shell para Wayland.

## Abrir la aplicación

Ejecuta `aspire-bg` o ábrelo desde Aspire Menu. La aplicación conserva su
configuración entre ejecuciones y, al aplicar un fondo, permanece abierta en la
cuadrícula local.

La aplicación tiene dos fuentes de fondos:

- **Local**: archivos de una carpeta elegida con el botón **Folder**. El
  selector de carpetas lo proporciona `aspire-files`.
- **Wallhaven**: búsqueda en línea, disponible solo si Aspire BG se compiló
  con la característica `wallhaven`.

La búsqueda de texto está en Wallhaven; el modo local muestra una selección
de la carpeta y permite cambiarla, ordenarla y actualizarla desde la barra.

## Modo local

La cuadrícula muestra hasta 30 fondos de la carpeta configurada, en tres
columnas. Acepta imágenes PNG, JPEG, WebP, BMP y GIF, además de vídeos MP4,
M4V, WebM, MKV, MOV, AVI y OGV. La vista previa se carga en segundo plano y se
guarda en `~/.cache/aspire-bg/`.

Haz clic en una miniatura para aplicar el fondo. También puedes enfocar una
miniatura con el teclado y pulsar **Enter**. Después de aplicarlo, Aspire BG
envía una notificación de escritorio y, si está activo Matugen, actualiza la
paleta.

La barra contiene:

- **Matugen/Tinty**: indica el motor de color activo. El nombre aparece con
  inicial mayúscula.
- **Carousel**: cambia a la vista de carrusel.
- **Refresh**: vuelve a leer la carpeta y genera otra selección cuando el
  orden es aleatorio.
- **Random**: aplica al instante un elemento aleatorio de la selección visible.
- **Options**: abre los ajustes.

En **Options** se encuentran:

- **Wallpaper engine**: backend local detectado en el sistema.
- **Monitor**: `All` o una salida conectada.
- **Fit**: `fill`, `stretch`, `fit`, `center` o `tile`.
- **Sort by**: `Random`, `Last modified`, `Date added` o `Oldest first`.
- **Subfolders**: incluye subdirectorios.
- **Hidden files**: incluye entradas ocultas.
- **Folder**: abre `aspire-files` para elegir otra carpeta.
- **Exit**: cierra la aplicación.
- **Wallhaven**: abre el modo en línea si está compilado con esa característica.

La carpeta, el backend, los ajustes, el orden y las opciones de exploración se
guardan en `~/.config/aspire-bg/config.ini`.

### Orden y actualización

La selección aleatoria usa muestreo de la carpeta y muestra como máximo 30
elementos. Los otros órdenes muestran hasta 30 archivos según su fecha de
modificación o creación. **Refresh** vuelve a hacer esa selección; no vuelve a
mostrar la carpeta completa de una sola vez. **Random** aplica uno de los
elementos que ya están visibles.

## Monitores y motores de fondo

La lista de backends se construye comprobando qué ejecutables están disponibles
en `PATH`. Aspire BG reconoce:

` swaybg`, `swww`, `awww`, `wallutils`, `hyprpaper`, `mpvpaper`, `gslapper`,
`xwallpaper`, `feh` y `linux-wallpaperengine`.

`wallutils` se detecta mediante `setwallpaper`, y `hyprpaper` mediante
`hyprctl`. Si no hay un backend disponible, la cuadrícula indica que se debe
instalar uno.

La opción **Monitor** determina el destino del siguiente fondo que apliques.
Con `All`, el fondo se aplica a todas las salidas y se eliminan las
asignaciones independientes guardadas. Si eliges una salida concreta, Aspire
BG actualiza esa salida y conserva las asignaciones de las demás. La
posibilidad de mostrar fondos distintos depende del backend: `wallutils`,
`linux-wallpaperengine`, `feh` y `xwallpaper` no admiten asignaciones distintas
por salida en esta implementación y mostrarán un error si se solicitan.

Para fondos en movimiento se necesita `mpvpaper` o `gslapper`. Tener instalado
`mpv` por sí solo no añade un backend de fondos animados. La vista previa de
vídeos utiliza `ffmpeg`; si no está disponible, los fondos de vídeo pueden no
tener miniatura animada. Los GIF se muestran como animaciones cuando el
decodificador del sistema los admite.

## Carrusel

**Carousel** muestra una imagen principal centrada y hasta cuatro imágenes
vecinas. La ventana se expande a la pantalla para centrar esta vista. La
selección actual puede cambiarse con las flechas, `Alt+H/J/K/L` o la rueda del
ratón.

- Haz clic en la imagen central para aplicar el fondo seleccionado.
- Haz clic en una imagen lateral para seleccionarla y aplicarla.
- `Ctrl+D` vuelve a cargar la selección del carrusel.
- `Ctrl+S` regresa a la cuadrícula.

Al aplicar un fondo desde el carrusel, Aspire BG vuelve a la cuadrícula y
permanece abierta. El modo de carrusel se guarda en la configuración y puede
ser el modo inicial de la siguiente ejecución.

## Wallhaven

Wallhaven requiere compilar con `--features wallhaven`. Al abrirlo por primera
vez, se carga la búsqueda **Hot**. El modo utiliza por defecto las categorías
General y Anime, la clasificación apta para todos los públicos, proporción
16:9 y resolución mínima 1920×1080. No hay interruptores de categorías o
clasificación en la interfaz; usa el campo de búsqueda para consultar otros
tags.

El selector de orden ofrece **Hot**, **Toplist**, **Uploads**, **Random** y
**Views**. **Refresh** inicia una búsqueda nueva con orden aleatorio dentro de
la consulta actual. **More** carga la página siguiente cuando la API informa
que hay más resultados. La búsqueda se envía tras una breve pausa al dejar de
escribir.

Haz clic en un resultado para descargar el archivo completo y aplicarlo. Los
archivos descargados se guardan en
`~/.config/aspire-bg/wallhaven/`; las miniaturas se guardan en
`~/.cache/aspire-bg/`. La caché de miniaturas de Wallhaven se limita a 200
archivos o 50 MB. El acceso a la API pública no requiere una clave; si se
configura una, se envía como `X-API-Key`.

`Esc` o **Exit** cierra Aspire BG desde Wallhaven; no regresa a la cuadrícula
local. Para volver al modo local, vuelve a abrir la aplicación.

La configuración de Wallhaven se guarda en la sección `[wallhaven]` del INI:

```ini
[wallhaven]
api_key =
atleast = 1920x1080
```

La clave API, si se usa, queda guardada como texto en ese archivo; protégelo
con los permisos apropiados.

## Teclado

| Teclas | Acción |
|---|---|
| Flechas | Mover el foco por la selección |
| `Alt+H`, `Alt+J`, `Alt+K`, `Alt+L` | Navegar en cualquier modo: izquierda, abajo, arriba y derecha |
| `Enter` | Aplicar el fondo enfocado en el modo local |
| `Ctrl+R` | Actualizar la selección local |
| `Ctrl+S` | Alternar cuadrícula/carrusel |
| `Ctrl+D` | Actualizar la selección del carrusel |
| `Ctrl+Z` | Ocultar o mostrar la barra del modo local |
| `Esc` | Cerrar Aspire BG |

Las flechas y los atajos `Alt+H/J/K/L` funcionan aunque el puntero no esté
encima de una miniatura. `Ctrl+S`, `Ctrl+D` y `Ctrl+Z` actúan en los modos
locales indicados; Wallhaven tiene sus propios controles en la barra.

## Motor de color: Matugen y Tinty

El motor de color es independiente del backend de fondos y se elige desde
**Aspire Menu → Style → Engine**. El indicador de la barra muestra el motor
activo.

| Motor | Al aplicar un fondo |
|---|---|
| **Matugen** | Deriva una paleta del fondo y la aplica a los componentes configurados. |
| **Tinty** | Conserva la paleta actual; cambiar el fondo no cambia el tema. |

Matugen y Tinty escriben los colores canónicos en
`~/.config/theme-engine/`. `aspire-core` aplica la paleta a los componentes
configurados del escritorio y solicita las recargas necesarias. Aspire BG no
recalcula colores al restaurar el fondo durante el inicio. Tampoco procesa
una imagen de vídeo con Matugen.

Para crear un tema Tinty a partir de una imagen:

1. Abre Aspire Menu y selecciona **Style → Background → Tema base16/base24**.
2. Elige un fondo local y pulsa **Enter**.
3. Aspire BG crea y aplica el tema Tinty; el sistema predeterminado es
   `base24`.

También se puede crear desde la terminal con
`aspire-bg --generate-tinty-theme IMAGEN --theme-system base16` o `base24`.

## Configuración, caché y compatibilidad

- Configuración: `~/.config/aspire-bg/config.ini`.
- Se puede usar otra ubicación del INI definiendo `ASPIRE_BG_CONFIG`.
- Caché local y miniaturas de Wallhaven: `~/.cache/aspire-bg/`.
- Descargas completas de Wallhaven: `~/.config/aspire-bg/wallhaven/`.
- Si no existe el INI de Aspire BG pero sí el de Waypaper, se importa una vez
  la configuración anterior. El antiguo `post_command` que ejecutaba
  `cambio_color.sh` se vacía durante esa migración; la gestión de colores
  corresponde ahora a Aspire BG y `aspire-core`.

Los errores de aplicación se informan con una notificación de escritorio y
por `stderr`. La notificación de éxito depende de `notify-send`; para verla,
el sistema debe tener un servicio de notificaciones, como Mako.

## CLI

```text
aspire-bg
aspire-bg --restore
aspire-bg --random
aspire-bg --next
aspire-bg --wallpaper IMAGEN
aspire-bg --apply-colors
aspire-bg --set-engine matugen|tinty
aspire-bg --set-tinty-theme NOMBRE
aspire-bg --create-tinty-theme [base16|base24]
aspire-bg --generate-tinty-theme IMAGEN [--theme-system base16|base24]
aspire-bg --folder DIRECTORIO
aspire-bg --backend NOMBRE
aspire-bg --monitor NOMBRE|All
aspire-bg --fill fill|stretch|fit|center|tile
aspire-bg --zen-mode
aspire-bg --list
aspire-bg --status
aspire-bg --no-colors
aspire-bg --no-post-command
aspire-bg --help
aspire-bg --version
```

- `--restore` reaplica el fondo guardado sin volver a generar los colores.
- `--random`, `--next` y `--wallpaper` aplican un fondo desde la terminal.
- `--apply-colors` vuelve a aplicar el motor de color activo.
- `--folder`, `--backend`, `--monitor` y `--fill` permiten indicar opciones
  para la operación o ejecución. Los cambios hechos desde la interfaz se
  guardan en el INI.
- `--list` imprime JSON con el backend, monitor, fondo actual y archivos
  encontrados.
- `--no-colors` omite el cambio de paleta al aplicar un fondo.
- `--no-post-command` omite un `post_command` heredado si aún está configurado.
- `--zen-mode` inicia el modo local con la barra oculta.

## Diagnóstico rápido

| Problema | Qué comprobar |
|---|---|
| No aparece Wallhaven | La instalación debe compilarse con `--features wallhaven`. |
| No aparece un backend | Instala su ejecutable y vuelve a abrir Options; la lista se detecta en `PATH`. |
| `mpv` está instalado pero no aparece | Para fondos de vídeo se necesita `mpvpaper` (o `gslapper`), no solo `mpv`. |
| No hay miniatura de vídeo | Comprueba que `ffmpeg` esté instalado y que el formato se pueda decodificar. |
| Folder no abre | Comprueba que `aspire-files` esté instalado y accesible por `PATH`. |
| No se aplican fondos distintos por monitor | Usa un backend con aplicación por salida; `wallutils`, `linux-wallpaperengine`, `feh` y `xwallpaper` no lo permiten aquí. |
| No se ve la notificación de éxito | Comprueba `notify-send` y el servicio de notificaciones, por ejemplo Mako. |
| Matugen no cambia los colores | Comprueba que Matugen esté instalado, que el fondo exista y que Matugen sea el motor activo. |

Para las opciones exactas de desarrollo, las dependencias y el flujo interno,
consulta el [README de Aspire BG](../aspire/aspire-bg/README.md).
