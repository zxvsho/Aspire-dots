# aspire-menu

Menú global del dotfile Mangowm_dots. Binario único en Rust con Relm4
gestionando el componente y GTK4 como toolkit, además de layer-shell. Se lanza
bajo demanda (toggle) y muere al usarse.

## Qué hace

- Lee `menu.toml` y muestra niveles navegables con búsqueda difusa.
- Ejecuta el script de cada acción (`bash -c`).
- Capa `Overlay` sin anclas, centrada por el compositor; namespace
  `aspire-menu` para reglas de Mango.
- Animación de altura fotograma a fotograma (160 ms ease-out) + fundido.
- Colores leídos de `~/.config/theme-engine/colors.rasi` (vía
  `aspire-core`).

## Estructura

```
aspire-menu/
├── Cargo.toml
├── menu.toml          # definición del menú (fuente de verdad)
├── build.sh           # (no existe en repo actual; compilar con cargo)
├── actions/           # scripts por categoría
│   ├── Apps/ Style/ Settings/ Utils/ Install/ Remove/ Update/ Extras/ System/
├── lib/               # helpers compartidos (clamp/desaturate)
└── src/
    ├── main.rs        # ventana, eventos, animación
    ├── model.rs       # parseo de menu.toml y estado
    ├── menu_logic.rs  # visibles, rutas, colores
    ├── search.rs      # fuzzy (fuzzy-matcher)
    ├── config.rs
    └── theme_engine.rs  # reexporta aspire-core
```

## Dependencias

- Rust toolchain
- Relm4 0.11
- GTK4 dev libs (`gtk4`, `glib`, `cairo`, `pango`)
- `gtk4-layer-shell`
- `aspire-core` (crate hermano, motor de color)

## Compilar y desplegar

```bash
cd ~/.dots/aspire/aspire-menu
cargo build --release
timeout 3 ~/.local/bin/aspire-menu   # 124 = abre bien
```

Binario vivo: symlink `~/.local/bin/aspire-menu → target/release/aspire-menu`.

## Configuración

- `menu.toml`: fuente en repo; `~/.config/rofi-menu/menu.toml` es
  symlink a ella. Se lee al abrir (no requiere recompilar).
- `ASPIRE_MENU_CONFIG` permite apuntar a otro archivo.
- `SCRIPT_DIR` (env) cambia la raíz de scripts (defecto `~/.dots/scripts`,
  pero las acciones usan `~/.dots/aspire/aspire-menu`).

## Integración

- Bind: `SUPER+espacio` → toggle.
- Regla Mango: `layerrule=noanim:1,layer_name:aspire-menu`.
- Ejecuta scripts de `actions/<Categoria>/` y binarios `aspire-bg`,
  `aspire-bg --set-engine`, etc.