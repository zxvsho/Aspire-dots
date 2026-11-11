# Prompt: centralizar temas, colores, motores y recargas en `aspire-core`

Documento para una IA (o persona) que implemente el refactor. Complementa `aspire-themes-spec.md` (secciones 4, 7, 9 y 14). Si hay conflicto, **manda este documento**. Lo marcado con **[verificar]** son suposiciones sobre el sistema que hay que comprobar antes de asumirlas.

---

## 1. Objetivo

Hoy algunos programas del dotfile se llaman entre sí lanzando procesos (por ejemplo `aspire-core::themes::apply_theme` ejecuta `aspire-bg`, y `aspire-menu` ejecuta `aspire-bg --set-engine`). Eso crea dependencias entre binarios, añade latencia y esconde errores.

El objetivo es que **un solo lugar, `aspire-core`, contenga toda la lógica de**:

- motores de color activos (matugen y tinty) y su estado;
- obtención y mapeo de la paleta;
- paquetes de tema (colores, fondos, fuente, iconos, archivos por aplicación);
- aplicar y revertir un tema;
- recargar los programas para que vean los cambios de colores, fuentes e iconos, en todo lo que dibuja ventanas (GTK 3, GTK 4, Qt, compositor, barra, notificaciones, terminales y aplicaciones con archivo de tema propio).

Los binarios `aspire-bg`, `aspire-themes`, `aspire-menu`, `aspire-clip` y `aspire-files` pasan a ser **interfaces finas** que llaman a funciones de `aspire-core`. **Ningún binario `aspire-*` lanza a otro.**

## 2. Reglas de arquitectura

1. Ningún código del repositorio puede ejecutar `Command::new("aspire-…")`. Se comprueba con:
   `grep -rn 'Command::new("aspire' --include=*.rs ~/.dots/aspire | grep -v target` (debe devolver 0 líneas).
2. Se permite lanzar procesos **externos de terceros** solo en dos lugares: la extracción de matugen y la capa de recarga (`mmsg`, señales, `makoctl`, `gsettings`, `fc-cache`, etc.).
3. Con el motor **tinty**, aplicar un tema o un fondo **nunca** debe ejecutar matugen ni recalcular colores a partir de una imagen.
4. Los errores se propagan con `Result` y se informan. Prohibido `let _ = …` sobre operaciones que pueden fallar. Un fallo en un paso de recarga no aborta el resto: se registra y se continúa.
5. Todas las operaciones son idempotentes (enlaces, copias, recargas): repetirlas no falla ni duplica nada.
6. Nada residente propio. Arranque objetivo de las herramientas gráficas: ~150 ms. Aplicar un tema con motor tinty: < 200 ms, sin contar las recargas externas lentas.
7. Rust + Relm4 0.11 sobre GTK4, sin libadwaita. Sin scripts shell para generar configuración. Comentarios y commits en español; interfaz de usuario en inglés.

## 3. Módulos de `aspire-core`

| Módulo | Responsabilidad |
|---|---|
| `state` | Leer y escribir **todo** el estado en `~/.config/color-engine/`: `active` (motor), `tinty-theme` (tema activo), `font-global`, `icons-global`, `last-wallpaper`. Un solo sitio, con una API tipada (`State::load()`, `State::save()`). |
| `palette` (hoy `colors.rs`) | Obtener la paleta de la fuente que toque: archivo `colors.toml` del paquete de tema, esquema tinty (YAML leído como datos, sin ejecutar nada) o matugen. Produce los 7 colores canónicos y `color0`–`color15`. |
| `theme_engine` | Renderizar archivos por aplicación y publicarlos de forma atómica en `current/` (ya existe; ver sección 6). |
| `themes` | Descubrir paquetes de tema (dos raíces, el del usuario gana), resolver capas y **aplicar/revertir**. Usa los demás módulos. |
| `wallpaper` | Detección de backends (swaybg, swww, awww, hyprpaper, mpvpaper, gslapper, xwallpaper, feh, wallutils, linux-wallpaperengine), aplicar fondo por monitor y ajuste, elegir uno aleatorio de una carpeta. **Se mueve desde `aspire-bg`**. |
| `fonts` | Enlazar la fuente del tema a `~/.local/share/fonts/aspire/theme/`, `fc-cache`, fijar la familia. Revertir. |
| `icons` | Enlazar el pack de iconos del tema a `~/.local/share/icons/` y fijar el ajuste. Revertir. |
| `reload` | La capa que avisa a los programas del cambio (sección 5). |

### API pública mínima

```rust
pub fn state() -> Result<State>;
pub fn set_engine(engine: Engine) -> Result<Report>;
pub fn apply_colors() -> Result<Report>;                 // reaplica el motor activo
pub fn list_themes() -> Result<Vec<ThemePackage>>;
pub fn apply_theme(id: &str, opts: ApplyOpts) -> Result<Report>;
pub fn revert_theme() -> Result<Report>;
pub fn set_wallpaper(path: &Path, target: Monitor, fit: Fit, opts: WallOpts) -> Result<Report>;
pub fn random_wallpaper(folder: &Path) -> Result<PathBuf>;
pub fn restore() -> Result<Report>;                      // reaplica el fondo guardado sin recolorear
```

`Report` devuelve, por cada paso (paleta, archivos, fuente, iconos, fondo, cada recarga), si salió bien, el error si lo hubo y el tiempo que tardó. Con `ASPIRE_TIMING=1` se imprime en stderr.

## 4. Orden de `apply_theme`

1. Cargar el paquete y resolver las capas de fuente e iconos (tema → `font-global` / `icons-global` → defecto).
2. Si el motor era matugen, cambiarlo a tinty (con `State`).
3. Paleta: `colors.toml` del paquete si existe; si no, el esquema (`scheme.yaml` del paquete, o `tinty_id` como respaldo); aplicar el mapeo de la sección 14 de `aspire-themes-spec.md`.
4. Renderizar y publicar `current/` (sección 6).
5. Fuente e iconos (módulos `fonts` e `icons`); antes, revertir los del tema anterior.
6. Fondo: aleatorio de `<tema>/wallpapers/` si `wallpaper = "random"` o falta; archivo concreto si se indica. **Con módulo `wallpaper` de core, sin proceso externo y sin tocar colores.**
7. Guardar el estado (`tinty-theme`).
8. Llamar a `reload` una sola vez al final, en paralelo y sin bloquear.

Revertir o pasar a matugen: vaciar `fonts/aspire/theme/`, quitar el enlace de iconos del tema y reaplicar `font-global` e `icons-global`.

## 5. Capa de recarga (`reload`)

Un registro de **destinos**; cada uno sabe cómo avisar a su programa. Se ejecutan en paralelo, con un tiempo máximo por destino, y su fallo no detiene a los demás. Ya existen recargas para Mango, Mako, terminales y Waybar: **se conservan tal cual** y se mueven aquí.

| Destino | Qué hace | Nota |
|---|---|---|
| Compositor (Mango) | `mmsg dispatch reload_config` y los archivos de color que ya genera | existente |
| Barra (Waybar) | señal de recarga | existente |
| Notificaciones (Mako) | `makoctl reload` | existente |
| Terminales | foot, kitty, alacritty, ghostty: secuencias o señales actuales | existente |
| GTK 4 y GTK 3 | escribir `~/.config/gtk-4.0/settings.ini` y `~/.config/gtk-3.0/settings.ini` (icon theme, font, preferencia de esquema claro/oscuro) y reflejar en `gsettings`/dconf lo mismo **[verificar qué claves lee cada versión]**; archivo `gtk-colors.css` importado desde `gtk.css` **[verificar si hace falta y cómo recargan las aplicaciones abiertas]** | las aplicaciones `aspire-*` vigilan `current/` con `notify` y reaplican su CSS al instante |
| Qt 5 y Qt 6 | `qt5ct`/`qt6ct` (iconos, fuente, paleta) y Kvantum si se usa **[verificar]** | muchas apps Qt solo lo ven al reiniciarse |
| Apps con archivo de tema propio | lo declarado en `~/.config/theme-engine/targets.toml` (sección 7) | btop, yazi, cava, nvim... |
| Fuentes | `fc-cache` solo si cambió la fuente | |

Cada destino implementa una interfaz común (`trait Reload { fn name(&self) -> &str; fn reload(&self, ctx: &Ctx) -> Result<()>; }`) para añadir otro sin tocar el resto.

## 6. Archivos por aplicación: `include/` y `templates/`

`theme_engine` ya renderiza `~/.config/theme-engine/templates/<app>.tpl` con tokens. Se añaden dos cosas **dentro del paquete de tema**:

```
assets/themes/<tema>/
├── theme.toml
├── colors.toml          # paleta ya resuelta (ver abajo); manda sobre derivar del esquema
├── scheme.yaml          # esquema tinty (los presets lo incluyen)
├── wallpapers/
├── include/             # archivos oficiales de cada app, copiados TAL CUAL (sin tokens)
├── templates/           # opcional: plantillas con tokens específicas del tema
├── fonts/               # opcional
└── icons/               # opcional
```

Resolución, por cada archivo de salida de `current/<nombre>`:

1. `include/<nombre>` del tema activo (se copia sin modificar; no pasa por el motor de tokens).
2. `templates/<nombre>.tpl` del tema activo (con tokens).
3. `~/.config/theme-engine/templates/<nombre>.tpl` global (con tokens).

Con matugen no hay paquete, así que solo se usa el paso 3. El de `include/` evita que las expansiones propias de algunos formatos (por ejemplo `${…}` de shell) se confundan con tokens y hagan fallar la publicación completa.

### `colors.toml` (paleta explícita del tema)

```toml
background     = "#282a36"
background_alt = "#21222c"
foreground     = "#f8f8f2"
selected       = "#bd93f9"
selectedtext   = "#282a36"
active         = "#bd93f9"
urgent         = "#ff5555"

[ansi]
color0  = "#21222c"
color1  = "#ff5555"
color2  = "#50fa7b"
color3  = "#f1fa8c"
color4  = "#bd93f9"
color5  = "#ff79c6"
color6  = "#8be9fd"
color7  = "#f8f8f2"
color8  = "#6272a4"
color9  = "#ff6e6e"
color10 = "#69ff94"
color11 = "#ffffa5"
color12 = "#d6acff"
color13 = "#ff92df"
color14 = "#a4ffff"
color15 = "#ffffff"
```
(TOML exige una clave por línea; el archivo completo de Dracula está en `colors.toml`.)

(Los valores de ejemplo son los de Dracula, a verificar contra https://github.com/dracula/dracula-theme.) Si el paquete trae `colors.toml`, `palette` lo lee directamente y **no deriva nada** del esquema: así un tema nunca depende del mapeo base16/base24. Para temas que no lo traen, se mantiene la derivación desde el esquema.

### `targets.toml` (enlaces y recargas de apps con archivo propio)

```toml
[btop]
from   = "btop.theme"
to     = "~/.config/btop/themes/aspire.theme"
reload = "pkill -USR2 btop"

[yazi]
from = "yazi-flavor.toml"
to   = "~/.config/yazi/flavors/aspire.yazi/flavor.toml"

[cava]
from   = "cava-theme"
to     = "~/.config/cava/themes/aspire"
reload = "pkill -USR2 cava"
```

Si el archivo no existe, no se hace nada. Los enlaces se crean de forma idempotente.

## 7. Bug de colores con esquemas base24 (corregir dentro de este refactor)

Con motor tinty y `base24-dracula`, `colors.css` sale con `background-alt = #ff6e6e` (es `base12`; debe ser `base01`) y `urgent = #21222c` (es `base01`; debe ser el rojo, `base08`). Con `base16-dracula`, `background-alt` sale bien.

- Leer los slots **siempre por nombre de clave**, nunca por posición ni ordenando como texto ni interpretando `base10` como hexadecimal. Los esquemas base24 añaden `base10`–`base17`.
- Mismo mapeo para base16 y base24 en los 7 colores canónicos. Documentarlo en el código.
- Valores del YAML con o sin `#` y entre comillas.
- Prueba unitaria con los YAML de `base16-dracula` y `base24-dracula` que compare los 7 canónicos y falle si difieren.
- `color0`–`color15` según la sección 14 de `aspire-themes-spec.md`, verificado contra `tinted-shell/scripts/base24-dracula.sh`.

## 8. Cambios por binario

- **`aspire-bg`**: conserva todas sus opciones de línea de comandos (`--restore`, `--random`, `--set-engine`, `--set-tinty-theme`, `--apply-colors`, etc.), pero cada una pasa a ser una llamada a `aspire-core`. Su interfaz gráfica llama a `wallpaper`, `palette` y `reload` de core. Con motor tinty y un tema activo, la carpeta por defecto es `<tema>/wallpapers/`, con opción de volver a la carpeta global.
- **`aspire-themes`**: llama a `apply_theme` de core; ya no ejecuta `aspire-bg`. Muestra por stderr cualquier error (prohibido `let _ =` en `apply_current`).
- **`aspire-menu`**: los ítems `matugen` / `tinty` llaman a `set_engine` de core en lugar de lanzar `aspire-bg --set-engine`. Si hoy esa lógica está en `main.rs`, queda como una llamada a core.
- **`aspire-clip` y `aspire-files`**: solo leen la paleta con `aspire-core` y vigilan `current/` con `notify` para reaplicar su CSS.

## 9. Tarjeta de vista previa de `aspire-themes` (información)

La información no va centrada: es una lista alineada a la izquierda con el formato **`Título: valor`**, etiqueta en color apagado:

```
Font: Global
Icons: Global
Wallpapers: 12
Includes: cava · nvim
```

- `Font` e `Icons`: nombre de la familia o del pack si el tema trae el suyo; si no, solo `Global`.
- `Wallpapers`: únicamente el número de archivos soportados en `wallpapers/`.
- `Includes`: nombres de las apps con archivo en `include/` o `templates/` del tema (texto del nombre de archivo anterior al primer `-` o `.`), separados por ` · `; si no hay, `none` apagado.

(El resto del diseño de la tarjeta sigue lo ya acordado: 8 barras de acento con degradado escalonado retro, sin barras de fondo y texto, ventana ~520×350, lista estrecha de ~140 px sin buscador.)

## 10. Orden de trabajo sugerido

Cada paso debe compilar y pasar las pruebas antes del siguiente:

1. Mover `state` y `reload` a core sin cambiar el comportamiento visible.
2. Mover `wallpaper` de `aspire-bg` a core; `aspire-bg` lo usa desde ahí.
3. `themes::apply_theme` usa solo módulos de core (sin `Command::new("aspire…")`).
4. `aspire-themes` y `aspire-menu` llaman a core.
5. Corregir el bug base24 (sección 7) y añadir la prueba.
6. Añadir `colors.toml` e `include/` al paquete de tema.
7. Añadir `targets.toml` y los destinos GTK/Qt de recarga, **verificando cada uno** en el sistema.

## 11. Criterios de aceptación

1. El `grep` de la regla 1 no devuelve ninguna línea.
2. Aplicar Dracula desde `aspire-themes`, desde `aspire-bg --set-tinty-theme` y desde el menú produce **exactamente los mismos** `colors.rasi` y `colors.css`.
3. Con motor tinty, ningún camino ejecuta matugen.
4. Aplicar un tema con motor tinty tarda < 200 ms (sin contar recargas externas lentas); con `ASPIRE_TIMING=1` se ve el reparto por paso.
5. Mango, Waybar, Mako, terminales, GTK 3, GTK 4 y las apps `aspire-*` muestran la paleta del tema. Qt se documenta aparte según lo verificado.
6. Cambiar de tema o a matugen restaura la fuente y los iconos globales del usuario.
7. Un fallo en un destino de recarga no impide los demás, y queda registrado.
8. Con el motor matugen todo sigue funcionando como antes (sin regresiones).
9. `cargo test --manifest-path ~/.dots/aspire/core/Cargo.toml` pasa, incluida la prueba de paridad base16/base24.

## 12. Entrega esperada

1. El diff completo por crate, agrupado por paso del apartado 10.
2. Comandos para compilar y desplegar (`cargo build --release` en cada crate y copiar/enlazar a `~/.local/bin/`).
3. Una lista de lo que se verificó en el sistema real y de lo que quedó como **[verificar]**.
