# aspire-clip

Portapapeles con vista previa (texto completo o imagen). Binario en Rust
+ GTK4 layer-shell. Se lanza bajo demanda y muere al copiar.

## Qué hace

- Lee el historial de `cliphist` (texto e imágenes).
- Lista filtrable a la izquierda, vista previa a la derecha.
- Ventana fija 880×480, centrada.
- Colores de `aspire-core`.

## Estructura

```
aspire-clip/
├── Cargo.toml
└── src/
    └── main.rs
```

## Dependencias

- Rust toolchain
- GTK4 dev libs + `gtk4-layer-shell`
- `gdk-pixbuf`
- `aspire-core`
- `cliphist`, `wl-clipboard` en runtime

## Compilar y desplegar

```bash
cd ~/.dots/aspire/aspire-clip
cargo build --release
timeout 3 ~/.local/bin/aspire-clip   # 124 = abre bien
```

Binario vivo: symlink `~/.local/bin/aspire-clip → target/release/aspire-clip`.

## Bind

`SUPER+v` → `pkill -x aspire-clip || ~/.local/bin/aspire-clip`.

## Requisito

Historial de `cliphist`. El vigilante debe estar activo:

```bash
systemctl --user enable --now cliphist.service
```

Si sale vacío, copiar algo de prueba.