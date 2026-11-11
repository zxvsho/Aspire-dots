# 07 — aspire-themes (selector de temas)

Selector de temas completo (paleta + fondo + fuente + iconos) para MangoWM.
Implementado en Rust con Relm4 0.11 sobre GTK4 y layer-shell.

## Atajos

| Tecla | Efecto |
|---|---|
| `Alt+j` | Baja la selección y actualiza la vista previa |
| `Alt+k` | Sube la selección y actualiza la vista previa |
| `Alt+l` | Aplica el tema seleccionado y cierra (igual que `Enter`) |
| `Alt+h` | Cierra sin aplicar (igual que `Esc`) |
| `↑` / `↓` | Mueven la selección |
| `Enter` | Aplica el tema seleccionado y cierra |
| `Esc` | Cierra sin aplicar |
| `Ctrl+j` / `Ctrl+k` / `Ctrl+l` | Opcionales: equivalentes a `Alt+j` / `Alt+k` / `Alt+l` |

Los atajos funcionan aunque el puntero no esté sobre la ventana. Al llegar al final, `Alt+j` no da la vuelta (igual con `Alt+k` en el primero). Mover la selección actualiza solo la tarjeta de vista previa; la ventana mantiene los colores del tema activo.

## Bind

`SUPER+t` → `pkill -x aspire-themes || ~/.local/bin/aspire-themes` (toggle).

## Arranque

`timeout 3 ~/.local/bin/aspire-themes` debe terminar con código 124.