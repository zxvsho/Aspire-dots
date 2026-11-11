# aspire-core

Núcleo compartido de las apps `aspire-*`. Lógica sin GUI (cero deps de
GTK/gdk). Contiene el motor de color y la lectura de la paleta.

## Módulos

- `colors`: lectura de los colores semánticos y slots ANSI desde
  `~/.config/theme-engine/current/colors.rasi` (con compatibilidad para
  `colors.rasi` en la raíz) → `ThemeColors`.
- `theme_engine`: selección matugen/tinty, paleta canónica, renderizado
  Rust de plantillas y publicación atómica de `current`, aplicación a
  mango/mako/walker y recarga de clientes.
- `mango_config`: edición TOML estructurada de `~/.config/mango/config.toml`,
  conservando comentarios y creando backup antes de escribir. Solo usa
  `config.conf` cuando no existe `config.toml`, como compatibilidad heredada.

Tras publicar los colores, `theme_engine` ejecuta las notificaciones de
`~/.config/theme-engine/targets.toml`. Mango se recarga con
`mmsg dispatch reload_config` y Waybar con `SIGUSR2`; no se repiten esas
acciones cuando están declaradas como destinos. El borde enfocado de Mango
usa el rol `selected` en `focus_color`: Matugen lo obtiene de `primary` y
Tinty del acento `base0D` (no de `base04`, que puede ser verde o gris según el
esquema). El rojo `urgent` solo se escribe en `urgent_color`. Ambos valores se
escriben como hexadecimal en el TOML con comentarios explicativos. Los errores
al actualizar Mango se propagan para no reportar un cambio de paleta exitoso
cuando la config TOML no pudo escribirse.

## Dueño de la paleta

`aspire-bg` es el dueño; este crate solo aloja la implementación para
que menu, clip y bg la compartan sin dependencias cruzadas entre GUIs.

## Plantillas

Se leen archivos `~/.config/theme-engine/templates/<app>.tpl`. Cada
archivo se renderiza en Rust; todos los resultados se validan en staging
y `~/.config/theme-engine/current` se cambia como symlink con `rename`.
Los temporales y el render anterior se limpian; no hay historial ni
rollback. Las plantillas pueden usar `${bg}`, `${fg}`, los roles
semánticos y `${color0}`–`${color15}`. Los slots ANSI proceden de Tinty o
de los roles de Matugen. Si no hay directorio `templates/`, se conserva
el flujo de paleta anterior hasta que se instalen plantillas.

Los esquemas Tinty y personalizados permanecen como datos en
`~/.local/share/tinted-theming/tinty/`; no se guardan bajo `current/`.

## Dependencias

- `anyhow`, `dirs`, `serde_json`.

## Uso

```toml
aspire-core = { path = "../core" }
```

```rust
use aspire_core::colors::load_theme_colors;
use aspire_core::theme_engine;
```