# 07 — theme-engine (plantillas de tema)

Motor de renderizado de configuración compartido por `aspire-bg` y los
consumidores de `aspire-core`. La sustitución de tokens y publicación de
archivos se hace en Rust; no ejecuta scripts shell para generar
configuración.

## Flujo

1. `aspire-bg` obtiene la paleta desde Matugen o desde el esquema Tinty
   activo.
2. `aspire-core` lee `~/.config/theme-engine/templates/<app>.tpl`.
3. Sustituye tokens y rechaza los desconocidos, mal formados o ausentes.
4. Escribe todos los resultados en un staging temporal, valida nombres,
   archivos y ausencia de tokens pendientes.
5. Renombra staging a un directorio publicado y crea un symlink temporal.
   `std::fs::rename` reemplaza `~/.config/theme-engine/current`
   atómicamente.
6. El directorio renderizado anterior se elimina después del cambio; no
   se conserva historial ni existe rollback.

El staging, el directorio publicado y el symlink están bajo
`~/.config/theme-engine`, en el mismo filesystem. Si `current` existe
como directorio real (no symlink), la operación falla sin reemplazarlo.
Si aún no existe `templates/`, el motor conserva el flujo de paleta
canónica previo y no cambia `current`.

## Tokens

Los colores se insertan como `#RRGGBB`:

| Token | Valor |
|---|---|
| `${bg}` | Alias de `${background}` |
| `${fg}` | Alias de `${foreground}` |
| `${background}` | Fondo |
| `${background_alt}` | Fondo alternativo |
| `${foreground}` | Texto principal |
| `${selected}` | Color de selección |
| `${selectedtext}` | Texto de selección |
| `${active}` | Color activo |
| `${urgent}` | Color urgente |
| `${color0}`…`${color15}` | Slots ANSI |

Tinty proporciona los 16 slots leyendo los datos del esquema sin
ejecutarlo. Matugen los deriva de roles de su paleta Material y usa roles
canónicos como fallback.

Ejemplo `~/.config/theme-engine/templates/foot.ini.tpl`:

```ini
background=${color0}
foreground=${color15}
selection-background=${selected}
selection-foreground=${selectedtext}
```

El nombre de salida será `foot.ini` dentro de
`~/.config/theme-engine/current/`. Las plantillas son archivos planos,
sin subdirectorios ni ejecución de comandos.

## Recarga de aplicaciones y datos de esquemas

Después de aplicar la paleta, `aspire-core` conserva las señales y
comandos de recarga existentes para Mango, Mako, terminales y Waybar.
Los consumidores que leen `colors.rasi` buscan primero
`current/colors.rasi` y luego el archivo canónico anterior en la raíz.

Los esquemas Tinty y personalizados siguen siendo entradas en
`~/.local/share/tinted-theming/tinty/`; no se copian al directorio
`current/`. Los scripts estándar y los YAML custom se parsean como datos
y no se ejecutan para renderizar configuración. El tema activo se
reemplaza, no se versiona.

## Código y pruebas

- `../aspire/core/src/theme_engine.rs`: tokenización, renderizado,
  staging, validación y cambio atómico.
- `../aspire/core/src/colors.rs`: lector semántico y slots ANSI.
- Validación: `cargo test --manifest-path .dots/aspire/core/Cargo.toml`.
