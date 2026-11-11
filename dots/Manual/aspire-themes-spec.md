# Especificación: sistema de temas (`aspire-themes`) y assets de fuentes e iconos

Documento para implementar en el dotfile Aspire (MangoWM, Arch Linux, Wayland). Está escrito para que una IA o una persona pueda implementarlo sin más contexto. Donde algo está marcado como **[a verificar]** o **[sugerencia]**, no es una decisión cerrada.

---

## 1. Contexto del proyecto

- Dotfile con compositor MangoWM. Repo de trabajo: `~/Proyectos/Mangowm_dots/` (remoto `zxvsho/Mangowm_dots`). Sistema vivo: `~/.dots/` + `~/.config/`.
- Herramientas propias en Rust + GTK4 (Relm4 en `aspire-bg`, `aspire-menu` y `aspire-themes`; layer-shell donde aplica), todas con prefijo `aspire-` y agrupadas como la familia **aspire-tools**: `aspire-menu`, `aspire-bg`, `aspire-clip`, `aspire-files` y `aspire-themes`.
- Crate compartido `aspire-core` (`.dots/aspire/core/`): motor de color (`theme_engine.rs`), lectura de paleta (`colors.rs`, `colors::load_theme_colors`).
- Filosofía: nada residente propio; cada herramienta se lanza bajo demanda y termina. Arranque objetivo ~150 ms. Sin scripts shell para generar configuración.
- Motores de color: **matugen** (deriva la paleta del fondo) y **tinty** (esquema base16/base24 fijo; cambiar el fondo no cambia los colores). El motor activo está en `~/.config/color-engine/active`.
- Estado existente: `~/.config/color-engine/tinty-theme` (id del tema tinty activo), `~/.config/color-engine/last-wallpaper`, `~/.config/theme-engine/colors.{rasi,css}` (paleta generada).
- Esquemas tinty: `~/.local/share/tinted-theming/tinty/`. Se parsean como datos, nunca se ejecutan.
- Interfaz de las herramientas en **inglés**; el manual en español.

## 2. Objetivo

1. Definir el **paquete de tema**: esquema de colores + fondos + fuente opcional + iconos opcionales, autocontenido.
2. Definir cómo conviven las fuentes e iconos **globales** con las que traen los temas.
3. Implementar `aspire-themes`: selector con vista previa para aplicar un tema completo.
4. Integrar `aspire-bg` para que muestre los fondos del tema activo.
5. Poner la lógica de aplicar y revertir en `aspire-core`, no en la interfaz.

## 3. Estructura de carpetas

```
.dots/assets/
├── fonts/                      # globales; sirven con matugen y con tinty
│   ├── fonts.md                # para humanos: qué hay, licencias, cómo descargar
│   └── fonts.toml              # para el instalador (ver 5.1)
├── icons/
│   ├── icons.md
│   └── icons.toml
└── themes/
    └── dracula/                # nombre de carpeta = id (slug) del tema
        ├── theme.toml
        ├── scheme.yaml         # esquema YAML de tinty; los presets lo incluyen
        ├── wallpapers/
        ├── fonts/              # opcional: archivos de fuente propios del tema
        └── icons/              # opcional: pack de iconos propio del tema

~/.local/share/aspire/themes/   # temas creados por el usuario, misma estructura
└── mi-tema/...
```

Reglas:

- Los temas del usuario viven **fuera** de `.dots` para que actualizar o reinstalar el dotfile nunca los toque.
- `aspire-core` lee ambas raíces. Si un id está repetido, **gana el del usuario**.
- Los temas se nombran por su nombre de acento (`dracula`, `nord`, `gruvbox`).
- Los binarios de fuentes e iconos **globales** no se versionan en el repo (peso y licencias): se descargan según el manifiesto. Las fuentes o iconos **propios de un tema** sí viven dentro de su carpeta (comprobar licencia antes de redistribuir).

## 4. `theme.toml`

```toml
name   = "Dracula"
system = "base24"                  # base16 | base24
font   = { family = "Fira Code Nerd Font", source = "bundled" }
icons  = { name = "Dracula", source = "bundled" }
tinty_id  = "base24-dracula"       # opcional: usa el esquema que ya incluye tinty
wallpaper = "random"               # random (por defecto) | nombre de un archivo de wallpapers/
```

`source` admite:

| Valor | Significado |
|---|---|
| `bundled` | Los archivos están en la carpeta del tema (`fonts/` o `icons/`). |
| `global` | Ya está en `assets/fonts` o `assets/icons`; no se duplica. |
| `system` | Instalado por el sistema (pacman, etc.); solo se referencia el nombre. |

Si `font` o `icons` faltan, el tema no toca esa capa (se queda la elección global).

Esquema de colores: los temas que vienen con el dotfile **incluyen su `scheme.yaml`** (copia del esquema oficial de tinty), para que el paquete funcione sin depender de que tinty tenga descargados sus repositorios de esquemas (tinty no trae esquemas con el binario; los descarga al sincronizar). `tinty_id` es opcional y solo se usa como respaldo si falta `scheme.yaml` (`~/.local/share/tinted-theming/tinty/scheme-repos/schemes/<sistema>/<nombre>.yaml`). Los esquemas de tinty son YAML; `theme.toml` es solo metadatos propios. `aspire-themes` debe poder listar temas leyendo solo `theme.toml`, sin parsear el YAML.

## 5. Manifiestos de assets globales

### 5.1 `fonts.toml` / `icons.toml`

```toml
[[font]]
family  = "JetBrainsMono Nerd Font"
url     = "https://..."
license = "OFL-1.1"
sha256  = "..."
```

`icons.toml` igual, con `name` en lugar de `family`. Alimentan las acciones del menú `Install → Fonts` e `Install → Themes` (hoy sin script). `fonts.md` e `icons.md` explican lo mismo en lenguaje humano.

## 6. Estado y resolución de capas

Archivos en `~/.config/color-engine/`:

| Archivo | Contenido | Nota |
|---|---|---|
| `active` | `matugen` o `tinty` | existente |
| `tinty-theme` | id del tema activo | existente; debe resolver a un paquete |
| `font-global` | familia elegida por el usuario | **nuevo**; un tema nunca lo sobrescribe |
| `icons-global` | pack de iconos elegido por el usuario | **nuevo**; igual |

Resolución, aplicada tanto a fuente como a iconos:

```
tema activo (solo si motor = tinty)  →  elección global  →  valor por defecto
```

- El valor efectivo se **calcula al aplicar**; nunca se guarda el override en el estado global.
- Con matugen no hay paquete de tema: solo cuentan global y defecto.
- Al pasar de tinty a matugen, `tinty-theme` se conserva, el override se desactiva y se reaplica lo global. Al volver a tinty, el tema recupera su fuente e iconos.

## 7. Aplicar y revertir un tema (lógica en `aspire-core`)

Módulo sugerido: `aspire-core/src/themes.rs`. Responsabilidades: descubrir paquetes en las dos raíces, leer `theme.toml` y `scheme.yaml` como datos, calcular la capa efectiva, aplicar y revertir. Lo usan `aspire-themes`, `aspire-bg` y el menú; nadie reimplementa nada.

**Aplicar**

1. Si el motor es matugen, cambiarlo a tinty (equivalente a `aspire-bg --set-engine tinty`).
2. Esquema: si el tema trae `tinty_id`, usar el esquema que ya incluye tinty y no enlazar nada; si trae `scheme.yaml`, enlazarlo en `~/.local/share/tinted-theming/tinty/custom-schemes/<sistema>/` con prefijo `aspire-` para evitar choques con esquemas del usuario **[a verificar: que esa ruta sea la que lee la versión de tinty instalada]**. Aplicar la paleta con el flujo existente (`theme_engine`).
3. Fuente, si `source = "bundled"`: enlazar la carpeta del tema a `~/.local/share/fonts/aspire/theme/`, ejecutar `fc-cache` y fijar la familia donde ya lo hace `Style → Font`. Si es `global` o `system`, solo fijar la familia.
4. Iconos, si `source = "bundled"`: enlazar a `~/.local/share/icons/<nombre>` y cambiar `gtk-icon-theme-name` en GTK 3 y GTK 4 **[a verificar: ajustes de Qt en qt5ct/qt6ct y gsettings en este entorno]**.
5. Fondos: la carpeta por defecto de `aspire-bg` pasa a ser `<tema>/wallpapers/` mientras haya tema activo (ver sección 9).
6. Guardar `tinty-theme` y lanzar las recargas que ya hace `aspire-core` (Mango, Mako, terminales, Waybar).

**Revertir / cambiar de tema**

Antes de aplicar otro tema (o al pasar a matugen): vaciar `~/.local/share/fonts/aspire/theme/`, quitar el enlace de iconos del tema anterior, `fc-cache`, y reaplicar `font-global` e `icons-global`.

Todas las operaciones sobre enlaces deben ser idempotentes y no fallar si el destino ya existe.

## 8. `aspire-themes` (herramienta nueva)

Binario aparte en Rust con Relm4 0.11 sobre GTK4, mismo patrón que `aspire-clip`: se lanza, se usa y termina. Comparte `aspire-core`. Interfaz en inglés. Todos los textos en un solo módulo de constantes (sin sistema de traducción por ahora).

### 8.1 Layout

Ventana de tamaño fijo, centrada **[sugerencia: ~760×460, ajustable]**. Dos paneles:

- **Izquierda (estrecha, ~150–180 px)**: lista de nombres de tema, tamaño de texto pequeño. **Sin buscador**: se navega solo con teclado o ratón.
- **Derecha (ancha)**: tarjeta de vista previa del tema seleccionado.

```
┌──────────────┬────────────────────────────────────────────┐
│ Dracula      │  Dracula                           base24  │
│ Nord         │                                            │
│ Gruvbox      │  ░░░ ▓▓▓  ███ ███ ███ ███ ███ ███ ███ ███  │
│ Everforest   │  ░░░ ▓▓▓  ███ ███ ███ ███ ███ ███ ███ ███  │
│ Catppuccin   │  ░░░ ▓▓▓  ███ ███ ███ ███ ███ ███ ███ ███  │
│ Tokyo Night  │  ░░░ ▓▓▓  ███ ███ ███ ███ ███ ███ ███ ███  │
│ ...          │  ░░░ ▓▓▓  ███ ███ ███ ███ ███ ███ ███ ███  │
│              │  bg  fg   8 accents                        │
│              │                                            │
│              │  Font    Fira Code Nerd Font               │
│              │  Icons   Dracula                           │
│              │                                            │
│              │  Enter apply · Esc close                   │
└──────────────┴────────────────────────────────────────────┘
```

El dibujo es solo orientativo: la caja izquierda es la lista y las columnas `███` son las barras verticales (cada una unas 3 veces más alta que ancha). Las medidas reales están en 8.2.

### 8.2 Tarjeta de vista previa

- Cabecera: nombre del tema y la insignia `base16` / `base24`.
- **Barras de color** (elemento principal):
  - Columnas verticales, **más altas que anchas**, esquinas **cuadradas** (como el resto de tarjetas del dotfile), pegadas con 4 px de separación, de ~130 px de alto.
  - Delante, dos barras estrechas (~22 px) con el **fondo** y el **texto** del tema, separadas de los acentos por un hueco de ~14 px. La barra de fondo lleva un borde de 1 px con el color de texto del tema al ~25 % de opacidad para que no desaparezca en temas oscuros.
  - Después, los **8 acentos** repartiéndose el ancho restante: en base16 son `base08`–`base0F`.
  - En base24 **[opcional]**, una segunda fila baja (~16 px) con las variantes brillantes (`base12`–`base17`), alineada con la de arriba.
  - Sin etiquetas dentro de las barras. El hex en un tooltip al pasar el ratón. Rótulos discretos opcionales debajo.
  - Un fundido de color de ~100 ms al cambiar de tema es un extra opcional; implementar primero sin él.
- Filas de texto: `Font` y `Icons`, **solo el nombre** (no se renderiza la fuente ni se muestran iconos de muestra). Si el tema no trae la suya: `Global` en color apagado.
- Pie discreto: `Enter apply · Esc close`.
- **No** se muestran fondos en esta herramienta (ver sección 9).

### 8.3 Colores de la interfaz

- **Ventana y lista**: colores del **tema activo**, leídos con `aspire-core` como las demás herramientas.
- **Solo la tarjeta de vista previa**: colores del **tema seleccionado** (su fondo y su texto). En GTK4, con un `CssProvider` aplicado a ese widget mediante clases propias (`themes-preview`), nunca al display.
- Razón: si toda la ventana cambiara al navegar, se perdería la referencia del tema actual y habría combinaciones ilegibles.
- Al aplicar un tema, recargar los colores de la ventana (como `aspire-files` con `notify`) o simplemente esperar a la próxima apertura.

### 8.4 Atajos (estilo vim, coherentes con `aspire-menu` y `aspire-clip`)

| Tecla | Efecto |
|---|---|
| `Alt+j` / `Alt+k` | Mueve la selección abajo / arriba y actualiza la vista previa |
| `Alt+l` | Aplica el tema seleccionado y cierra, como `Enter` |
| `Alt+h` | Cierra sin aplicar, como `Esc` **[sugerencia, ver nota]** |
| `↑` / `↓` | Mueve la selección |
| `Enter` | Aplica el tema seleccionado y cierra |
| `Esc` | Cierra sin aplicar |
| `Ctrl+j` / `Ctrl+k` / `Ctrl+l` | Equivalentes a `Alt+j` / `Alt+k` / `Alt+l`, como en `aspire-menu` y `aspire-clip` **[opcional]** |
| Clic en una fila | Selecciona; doble clic aplica **[sugerencia]** |

Nota sobre `Alt+h`: en `aspire-menu`, `h` significa "atrás" (subir de nivel). Aquí no hay niveles ni búsqueda que borrar, así que no tiene un destino natural. Se propone que cierre sin aplicar, que es lo más cercano a "volver"; si no se quiere, dejarlo sin asignar. Las teclas `Alt+j/k/l` deben funcionar aunque el puntero no esté sobre la lista.

Si el motor activo es matugen, mostrar en la tarjeta un aviso discreto: `Switches engine to Tinty`.

### 8.5 Integración

- Lanzamiento desde `aspire-menu`: `Style → Theme → Selector`.
- Bind propio tipo interruptor, como los demás **[sugerencia]**: `pkill -x aspire-themes || ~/.local/bin/aspire-themes`.
- Binario vivo como symlink a `target/release`, como el resto (sin `build.sh`).
- Sin ID de aplicación de D-Bus, para mantener el arranque rápido.
- Los datos para la lista y la vista previa salen de `theme.toml` y del esquema (el `scheme.yaml` del paquete, o el de tinty según `tinty_id`), leídos como datos. Si no se puede leer el esquema, la tarjeta debe mostrar un aviso claro (`Scheme not found`) en lugar de quedarse vacía. Cachear la lista si el arranque se acerca a ~150 ms.

## 9. Fondos del tema (`aspire-bg`)

`aspire-themes` no muestra fondos. De ellos se encarga `aspire-bg`, que ya tiene cuadrícula, carrusel y caché de miniaturas:

- Con motor tinty y tema activo, la carpeta por defecto de `aspire-bg` es `<tema>/wallpapers/`, con una opción para cambiar a la carpeta global de siempre.
- Con tinty, cambiar el fondo no recolorea: no hay conflicto con la paleta del tema.
- Al activar el tema se aplica un fondo de `wallpapers/`: aleatorio por defecto (`wallpaper = "random"`) o el archivo indicado en `wallpaper`. Con tinty no recolorea, así que equivale a `--random` sobre la carpeta del tema con `--no-colors`.
- Guardar la carpeta anterior para poder volver.
- `aspire-bg --generate-tinty-theme IMAGEN` debe generar un **paquete completo** en `~/.local/share/aspire/themes/<id>/` (con `theme.toml`, `scheme.yaml` y `wallpapers/` con la imagen de origen copiada), no solo un YAML suelto. Rechazar o renombrar ids que choquen con presets del dotfile.
- `aspire-bg --set-tinty-theme ID` debe aplicar el paquete completo (sección 7), no solo la paleta.

## 10. Convenciones a respetar

- Rust + Relm4 0.11 + GTK4; sin libadwaita. Sin procesos residentes propios.
- Interfaz en inglés, sentido de capitalización "sentence case", sin puntuación final en etiquetas.
- Si hace falta algún script, `set -euo pipefail` y `exit 0`; preferir Rust.
- Pruebas del módulo de temas: `cargo test --manifest-path .dots/aspire/core/Cargo.toml`.
- Documentar en el manual: nueva parte para `aspire-themes` y actualización de `02-estructura-general.md` (árbol de `assets/`, archivos de estado nuevos).

## 11. Criterios de aceptación

1. `aspire-themes` abre en ~150 ms, lista temas de ambas raíces y muestra la vista previa con barras, fuente e iconos.
2. Mover la selección actualiza solo la tarjeta; la ventana mantiene los colores del tema activo.
3. `Enter` aplica paleta, fuente, iconos y carpeta de fondos; cambia el motor a tinty si hacía falta.
4. Cambiar de un tema con fuente propia a otro sin fuente restaura la fuente global del usuario.
5. Cambiar a matugen reaplica las elecciones globales; volver a tinty reactiva el override del tema.
6. Un tema del usuario con el mismo id que un preset lo reemplaza; actualizar `.dots` no modifica `~/.local/share/aspire/themes/`.
7. `font-global` e `icons-global` nunca contienen valores impuestos por un tema.
8. Los presets y los temas generados con `--generate-tinty-theme` son paquetes válidos y aparecen en la lista.

## 12. Fuera de alcance

- Mostrar o elegir fondos dentro de `aspire-themes`.
- Buscador o filtro de temas dentro de `aspire-themes`.
- Editor o creador de temas con interfaz (la creación sigue por `aspire-bg` y el menú).
- Renderizar la fuente o mostrar iconos de muestra en la vista previa.
- Soporte específico de Qt más allá de lo que se verifique con qt5ct/qt6ct.

## 13. Puntos a verificar antes de implementar

- Ruta de `custom-schemes` que lee la versión instalada de tinty.
- Qué ajustes cambian el tema de iconos en GTK 3/4 y Qt en este entorno Wayland.
- Licencias de las fuentes e iconos que se quieran incluir dentro de un tema.
- Que `font-global` e `icons-global` no choquen con archivos ya existentes en `color-engine/`.
- Formato de los YAML de tinty: los actuales usan la clave `palette` con `base00`…`base17`. En el de Dracula base24 los valores vienen entre comillas y **con** `#` (`"#ff5555"`), pero otros esquemas pueden venir sin `#`: el lector debe aceptar ambos (añadiendo el `#` si falta) y también el formato antiguo con las claves en la raíz.

## 14. Mapeo de slots base16/base24 a `color0`–`color15`

Las plantillas (cava, btop, yazi, terminales...) usan `${color0}`–`${color15}` con la semántica ANSI habitual (color1 = rojo, color2 = verde...). El motor debe construir esos slots **con este mapeo** y no con la identidad (`color0 = base00`, `color1 = base01`...), que daría colores desordenados:

```
color0  = base00      color8  = base03
color1  = base08      color9  = base08  (base24: base12)
color2  = base0B      color10 = base0B  (base24: base14)
color3  = base0A      color11 = base0A  (base24: base13)
color4  = base0D      color12 = base0D  (base24: base16)
color5  = base0E      color13 = base0E  (base24: base17)
color6  = base0C      color14 = base0C  (base24: base15)
color7  = base05      color15 = base07
```

Referencia para verificarlo: los scripts de `tinted-shell` que ya están en la máquina, por ejemplo `~/.local/share/tinted-theming/tinty/repos/tinted-shell/scripts/base24-dracula.sh`, que asignan cada `colorN` de la terminal a su slot. Si difieren de esta tabla, mandan los scripts.
