# 02 — Estructura general actual

Cómo está organizado el repositorio y la configuración en uso.
Todo lo de esta página está verificado contra el sistema real (octubre 2026).

---

## Árbol real de `.dots/` (sistema vivo)

```
.dots/
├── Manual/               # Este manual
├── aspire/
│   ├── core/             # Crate compartido: motor de color + lectura de paleta
│   │   ├── Cargo.toml
│   │   └── src/ (lib.rs, colors.rs, theme_engine.rs)
│   ├── aspire-bg/        # Selector de fondos + motor de color (Rust+Relm4/GTK4)
│   ├── aspire-clip/      # Portapapeles con preview (Rust+GTK4)
│   ├── aspire-menu/      # Menú global (Rust+Relm4/GTK4)
│   │   ├── actions/      # Acciones: Apps/ Settings/ Style/ Utils/ Install/ Remove/ Update/ Extras/ System/
│   │   ├── lib/          # Helpers compartidos (clamp/desaturate)
│   │   ├── menu.toml     # Definición del menú (fuente de verdad del menú)
│   │   └── src/          # Código Rust
│   ├── aspire-tui/       # Experimentos TUI (Ratatui)
│   ├── aspire-music/     # Reproductor de música TUI (Ratatui+Tokio; parte 08)
│   ├── aspire-themes/    # Selector de temas Aspire
│   └── aspire-bar/       # Editor de Waybar para MangoWM (Rust+GTK4)
├── assets/               # Wallpapers, iconos, fuentes y waybar-themes/
├── bin/                  # Staging generado por build.sh (aspire-bar)
├── build.sh              # Compila aspire-bar a staging; deploy explícito
├── Manual/               # Manual del dotfile
└── scripts/              # Scripts legacy y utilidades
```

`aspire-files` vive en un repo aparte: `~/Proyectos/aspire-files/`
(crates: `aspire-files`, `aspire-fs`, `aspire-config`, `aspire-thumb`).

## Configuración en `~/.config/`

Cada programa lee su carpeta. Las que manda este dotfile:

```
~/.config/
├── mango/          # config.conf, rule.conf, bind.conf, autostart.sh,
│                   # tag.conf, monitor.conf, env.conf, scripts/
├── rofi/           # launchers/, powermenu/, launcher_activo, powermenu_activo
├── rofi-menu/      # menu.toml ← symlink al del repo (fuente de verdad)
├── aspire-menu/    # (reservado; hoy vacío)
├── waybar/ mako/ foot/ kitty/ alacritty/ ghostty/
├── aspire-bg/ matugen/ tinty/
├── color-engine/   # active, last-wallpaper, tinty-theme, mango-focuscolor.conf, ...
├── theme-engine/   # colors.rasi, colors.css (generados por aspire-bg)
├── aspire-files/   # config.toml (tema, preferencias)
├── aspire-music/   # config.toml opcional (el crate trae el suyo en dev)
├── fish/ nvim/ micro/ btop/ fastfetch/ cava/ mpv/
├── gtk-3.0/ gtk-4.0/ fontconfig/
└── ...             # resto de apps (thunar, pcmanfm, chromium, ...)
```

## Archivos de estado (los lee el menú y los programas)

| Archivo | Ejemplo real | Quién lo escribe |
|---|---|---|
| `~/.config/color-engine/active` | `matugen` | `aspire-bg --set-engine` |
| `~/.config/color-engine/tinty-theme` | `base16-everforest-dark-hard` | `aspire-bg --set-tinty-theme` |
| `~/.config/color-engine/last-wallpaper` | ruta del fondo | `aspire-bg` |
| `~/.config/rofi/launcher_activo` | `type1/style1` | Style → Launcher → Apply |
| `~/.config/rofi/powermenu_activo` | `style3` | Style → Powermenu → Apply |
| `~/.config/theme-engine/colors.{rasi,css}` | paleta generada | `aspire-bg` (`aspire-core::theme_engine`) |
| `~/.config/aspire-bg/config.ini` | backend, carpeta, fondo, fill, sort, color | `aspire-bg` |
| `~/.config/aspire-files/config.toml` | tema, preferencias | `aspire-files` |

Nota: `~/.config/bg-engine/` es un vestigio legacy; el código lee ambos pero
`color-engine/` es la fuente real. No crear archivos en `bg-engine`.

## Sistema vivo único (importante)

Hay **un solo árbol de trabajo**: `~/.dots/` + `~/.config/`. Ahí se edita,
se compila y se despliega. El antiguo laboratorio `~/Proyectos/Mangowm_dots/`
(remoto `zxvsho/Mangowm_dots`) quedó **obsoleto**: no se usa ni se
sincroniza; su valor es solo el historial git de referencia.

Binarios vivos en `~/.local/bin/` (verificado octubre 2026):

- `aspire-menu`, `aspire-clip`, `aspire-bg`, `aspire-files`: **copias** de
  `target/release`. Tras compilar hay que copiar el binario nuevo.
- `aspire-music`: **symlink** a snapshot estable,
  `~/.dots/aspire/aspire-music/releases/aspire-music-<fecha>` (nunca a
  `target/release`; flujo dev/deploy en parte 08).
- `aspire-bar`: copia de release desplegada con `~/.dots/build.sh deploy`;
  el staging previo queda en `~/.dots/bin/aspire-bar`.

`build.sh` compila y deja aspire-bar en `~/.dots/bin/`; el deploy es
explícito. Los demás crates se compilan desde su directorio con Cargo.

`~/.config/rofi-menu/menu.toml` ya **no es symlink**: es un fichero vivo
en `~/.config/` (edítalo ahí; se lee al abrir el menú, sin recompilar).

## Binds del menú (en `~/.config/mango/bind.conf`)

```
SUPER+espacio    →  pkill -x aspire-menu || ~/.local/bin/aspire-menu   (toggle)
SUPER+v          →  pkill -x aspire-clip || ~/.local/bin/aspire-clip   (toggle)
SUPER+t          →  pkill -x aspire-themes || ~/.local/bin/aspire-themes (toggle)
ALT+espacio      →  fuzzel (lanzador de apps)
SUPER+w          →  pkill -x aspire-bg || ~/.local/bin/aspire-bg       (selector de fondos)
SUPER+s          →  ~/.dots/scripts/rofi_menu_selector.sh              (selector estilo legacy)
SUPER+x          →  ~/.dots/scripts/rofi_powermenu.sh                  (powermenu legacy)
SUPER+l          →  ~/.dots/aspire/aspire-menu/actions/System/lock.sh
SUPER+SHIFT+s    →  screenshot región
Print            →  screenshot pantalla completa
SUPER+ALT+b      →  status-bar-selector (waybar/mangobar)
SUPER+ALT+SHIFT+b→  aspire-bar (editor Waybar; bind sugerido)
SUPER+CTRL+Return→  mmsg dispatch reload_config
```

El antiguo `rofi-menu.sh` y su bind se eliminaron; el crate viejo
`scripts/rofi-menu/` también. Los scripts de `Apps/`, `Style/`, etc.
**se quedan**: aspire-menu los ejecuta vía `[actions]` en `menu.toml`.

## Flujo de color

```
aspire-bg --restore (autostart)  o  elegir fondo en aspire-bg
        │
        ▼
motor activo en ~/.config/color-engine/active
        │
   ┌────┴─────┐
   ▼          ▼
matugen     tinty
(deriva del (tema fijo; el fondo
 wallpaper)  no cambia colores)
   │          │
   └────┬─────┘
        ▼
aspire-core::theme_engine: 7 colores canónicos
        │
        ├── ~/.config/theme-engine/colors.{rasi,css} → rofi, waybar, aspire-*
        ├── mango (root/border/focus/urgent…), mako, walker
        └── plantillas de matugen/tinty → foot, kitty, alacritty, ghostty

Motor matugen: cada cambio de fondo reextrae y recolorea todo.
Motor tinty: cambiar fondo solo cambia el fondo. Los temas base16/24
se crean a propósito desde aspire-menu → Background → Tema base16/24…
(ahí aspire-bg abre el selector y genera/aplica el esquema al elegir).

El borde enfocado de Mango toma el acento de la paleta: `primary` en Matugen
y `base0D` en Tinty. `base04` no es un color de acento y puede verse verde o
gris según el esquema. `aspire-core` actualiza `focus_color` y `urgent_color`
en `~/.config/mango/config.toml` conservando comentarios, y luego recarga Mango.
```

`theme_engine.rs` vive en `aspire-core` y lo usa `aspire-bg`; ya no hay
`color-engine.sh`, `cambio_color.sh` ni cadenas de scripts de color.

## Convenciones

| Regla | Descripción |
|---|---|
| `aspire-menu/actions/<Categoria>/` | Acciones del menú, PascalCase (`Style`, `Utils`, `System`) |
| `aspire-menu/lib/` | Código compartido; nunca se invoca directo desde el menú |
| `~/.config/` | Solo archivos estáticos de config, sin lógica |
| `themes/` | Paquetes de tema autocontenidos |
| `local/` | Gitignored: por máquina y secretos |
| Scripts de acción | `set -euo pipefail`, `SCRIPT_DIR` con defecto a `~/.dots/aspire/aspire-menu`, `exit 0` |

## Mango 0.18.0: claves con guion bajo

Desde la 0.18.0 las claves usan `snake_case` (`border_color`,
`focus_color`, `window_rule`, `layer_rule`, `tag_rule`, `exec_once`,
`key_mode`, …). Las viejas sin guion se ignoran en silencio. Tras
editar `~/.config/mango/`, validar con `mango -p` (debe dar 0
errores) y recargar con `mmsg dispatch reload_config` o
`SUPER+CTRL+Return`. Respaldo previo a la migración en
`~/.config/mango/bak-018/`. Opciones eliminadas en la 0.18.0
(comentadas como `REMOVED-018`): `ov_tab_mode`, `ov_no_resize`,
`syncobj_enable`. En reglas, además: `appid`→`app_id`,
`isfloating`→`is_floating`, `isnoborder`→`no_border`,
`isnosizehint`→`no_size_hint`, `isterm`→`is_term`,
`istagsilent`→`is_tag_silent`, `nofadein/out`→`no_fade_in/out`,
`globalkeybinding`→`global_key_binding`, y `is_unglobal` no existe
más (se quitó dejando el `is_overlay` que ya estaba). Referencia:
wiki `mangowm/mango` → página `rules`.

## Legacy (`~/.dots/scripts/`)

El directorio `~/.dots/scripts/` conserva los scripts bash antiguos.
Muchos están huérfanos. Los que siguen vivos:

- `rofi_menu_selector.sh` (bind `SUPER+s`) y su cadena interna:
  `rofi_style_selector.sh` → `color-engine-selector.sh`,
  `tinty-theme-selector.sh`, `rofi_selector.sh`,
  `rofi_powermenu_selector.sh`, `Style/Fonts/set-font.sh`.
- `rofi_powermenu.sh` (bind `SUPER+x`).
- `Utils/status-bar-selector.sh` (autostart, modo `auto`).
- `foot-osc-reload.sh` (uso manual).

Todo lo demás en `~/.dots/scripts/` (duplicados top-level, `lib/`
helpers, `Style/*` selectores, `Utils/clipboard.sh`, etc.) está
superseded por `aspire-menu` + `aspire-bg` y es candidato a borrar.
La copia `~/.dots/scripts/aspire-menu/` es stale (5 archivos con rutas
legacy); la canónica es `~/.dots/aspire/aspire-menu/`.

## Notas

- `ALT+espacio` apunta a `fuzzel`, no a rofi. Si se quiere un lanzador
  rofi directo en esa tecla, atar a `rofi -show drun`.
- `SUPER+x` apunta a `rofi_powermenu.sh`, que **sí existe** y funciona.
- `aspire-music` aún **no tiene bind**. Sugerido:
  `bind=SUPER,m,spawn_shell,pkill -x aspire-music || ~/.local/bin/aspire-music` (toggle, parte 08).
- `SUPER+l` y `SUPER+SHIFT+s` apuntan a
  `~/.dots/aspire/aspire-menu/actions/...`, que sí existen.
- `rofi-menu.rasi` sigue declarando `modi: "menu:~/.dots/scripts/rofi-menu.sh"`,
  pero ese modo ya no se usa (el menú global es `aspire-menu`). Línea stale.