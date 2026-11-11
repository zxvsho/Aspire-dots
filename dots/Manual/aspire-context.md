# Contexto de trabajo: Aspire dotfile (MangoWM)

## Estado actual (Nov 2026)

### Crates
- `~/.dots/aspire/core/` — aspire-core (colors, theme_engine, themes, mango_config).
- `~/.dots/aspire/aspire-themes/` — selector Relm4/GTK4 (symlink a target/release en ~/.local/bin).
- `~/.dots/aspire/aspire-bg/` — selector de fondos (copiado a ~/.local/bin).
- `~/.dots/aspire/aspire-menu/` — menú (symlink).
- `~/.dots/aspire/aspire-clip/`, `aspire-files/`, `aspire-music/`.
- `~/.dots/aspire/aspire-bar/` — scaffold con 26 errores de compilación (modelo reconstruido parcialmente).

### Lo completado
- `aspire-core`: paleta canónica, `colors.toml` prioridad sobre scheme, `apply_package` única, `apply_theme`/`apply_theme_by_id`, normalización de ids (`base24-dracula` ↔ `base24-aspire-dracula`), `restart_waybar` con validación de imports (kill+respawn), `mango_config` escribe top-level, `targets.toml` para recarga extensible.
- **Módulos nuevos de core (refactor arquitectura):** `state` (State::load/save + accessors granulares), `fonts`, `icons`, `reload` (trait Reload + destinos Mango/Waybar/Terminales/Mako/GTK/Qt/targets.toml), `wallpaper` (backends, random, restore, swaybg). `themes` y `theme_engine` delegan en ellos.
- `aspire-themes`: UI 520×350, 8 barras de acento escalonadas, info alineada a la izquierda (`Font:`, `Icons:`, `Wallpapers:`, `Includes:`), atajos Alt+j/k/l/h, bind SUPER+t, no silencia errores.
- Temas: `dracula` (base24) y `everforest-dark` (base16 soft) con `colors.toml`, `scheme.yaml`, wallpapers.
- Waybar themes: `capsule-bar`, `simple-bar` importan `colors.css` compartido por symlink.
- `~/.config/mango/config.toml`: bloque de colores duplicado eliminado.
- `cargo test` (core) → 16 tests OK.
- `grep Command::new("aspire` → 0 líneas.

### Pendiente
- Migrar `aspire-bg` para que use `aspire-core::wallpaper` (multi-backend) en lugar de su código propio; hoy sigue teniendo su propia implementación.
- Verificar en sistema real los destinos GTK 3/4, Qt y apps con archivo propio (marcados [verificar]).
- `targets.toml` ya creado en `~/.config/theme-engine/targets.toml`.

### Completado adicional
- `aspire-bar`: modelo adaptado a `Option<...>`, 26 errores corregidos, UI conectada en `main.rs` (llama a `ui::run()`); compila y desplegado en `~/.local/bin`.
- `fonts`: ahora fija la familia en GTK (settings.ini + gsettings) al aplicar/revertir.
- `reload`: destinos GTK (gsettings) y Qt (qt5ct/qt6ct) implementados (best-effort).
- `cargo test` (core) → 16 tests OK; grep aspire → 0.

### Comandos clave
- Compilar core: `cargo build --release --manifest-path ~/.dots/aspire/core/Cargo.toml`
- Tests: `cargo test --manifest-path ~/.dots/aspire/core/Cargo.toml`
- Desplegar: `ln -sf .../target/release/aspire-themes ~/.local/bin/aspire-themes`
- Verificar: `grep -rn 'Command::new("aspire' ~/.dots/aspire --include=*.rs` → 0 líneas
- Arranque: `timeout 3 ~/.local/bin/aspire-themes` → 124

### Archivos de referencia
- `~/.dots/Manual/aspire-themes-spec.md`
- `~/.dots/Manual/aspire-core-arquitectura-prompt.md`
- `~/.dots/Manual/02-estructura-general.md`, `07-aspire-themes.md`