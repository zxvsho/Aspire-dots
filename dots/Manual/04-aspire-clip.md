# 04 — aspire-clip (portapapeles con vista previa)

Menú del portapapeles estilo tu captura: lista a la izquierda,
vista previa a la derecha (texto completo o imagen), tamaño medio,
apertura rápida.

---

## Qué es y estado actual

- Binario aparte en Rust + GTK4 layer-shell:
  `.dots/aspire/aspire-clip/` (`Cargo.toml`, `src/main.rs`).
- Lee el historial de **cliphist**
  (`cliphist list` → `id\tvista`, `cliphist decode <id>` por selección),
  que guarda texto e imágenes.
- Nada residente: se lanza, copias con `Enter` y muere. Sin app-id de
  D-Bus para abrir en ~150 ms como aspire-menu.
- Reutiliza `aspire-core` (`colors::load_theme_colors`) para los colores;
  clases CSS propias (`clip-*`).
- Comparte `aspire-core` con `aspire-bg` y `aspire-menu`.

## Requisito: historial de cliphist

Fuente: **cliphist**. En `autostart.sh` del compositor puede lanzarse el
vigilante:

```
wl-paste --type text --watch cliphist store &
wl-paste --type image --watch cliphist store &
```

El sistema actual no lo tiene en `autostart.sh` (clipse se gestiona solo);
si el menú sale vacío, lanzar los vigilantes manualmente o copiar algo
de prueba. Límite en código: últimas 40.
Gestión: `cliphist delete-query <texto>` / `cliphist wipe`.

## Compilar, probar y desplegar

```bash
cd ~/.dots/aspire/aspire-clip
cargo build --release
cp target/release/aspire-clip ~/.local/bin/aspire-clip
timeout 3 ~/.local/bin/aspire-clip   # probar: 124 = abre bien
```

Binario vivo: copia en `~/.local/bin/aspire-clip` (se despliega copiando
`~/.dots/aspire/aspire-clip/target/release/aspire-clip`).

Bind (interruptor, como el menú):

```
bind=SUPER,v,spawn_shell,pkill -x aspire-clip || ~/.local/bin/aspire-clip
```

## Diseño (fijo 880×480, centrado)

```
┌──────────────────────────────┬───────────────────────────┐
│ Search clipboard...          │                           │
│ ▸ primera coincidencia       │   vista previa del        │
│   segunda línea              │   seleccionado:           │
│   📷 captura.png             │   texto completo (scroll) │
│   ...                        │   o imagen escalada       │
│                              │                           │
│                              │   Enter copiar · Esc cerrar│
└──────────────────────────────┴───────────────────────────┘
```

- Izquierda (360 px): buscador + lista filtrable (subcadena,
  insensible a mayúsculas), scroll si hay muchas.
- Derecha: `Stack` con página de texto (`TextView` solo lectura,
  monoespaciada) o de imagen (`Picture` con `ScaleDown`, nunca
  desborda). Separador vertical entre paneles.
- Sin animación de tamaño (ventana fija; el contenido hace scroll).

## Atajos

| Tecla | Efecto |
|---|---|
| `Enter` | Copia lo seleccionado (`wl-copy`, imagen con `--type image/png`) y cierra |
| `Esc` | Cierra sin copiar |
| `↑` / `↓`, `Ctrl+j` / `Ctrl+k` | Mueve la selección (actualiza la vista previa) |
| `Ctrl+h` | Limpia la búsqueda |
| `Ctrl+l` | Copia como `Enter` |
| Clic | Copia la fila |

## Problemas comunes

| Síntoma | Causa probable | Solución |
|---|---|---|
| Sale vacío | No corre el vigilante / no hay historial | `cliphist list`; lanzar `wl-paste --watch cliphist store` |
| No aparece imagen | `filePath` apunta a fichero borrado (`tmp_files` limpiado) | Re-copiar la imagen |
| No copia | Falta `wl-clipboard` | Instalar `wl-clipboard` (`wl-copy`/`wl-paste`) |
| No abre | Ver parte 03 (mismo arranque que aspire-menu) | `timeout 3 ~/.local/bin/aspire-clip` debe dar 124 |