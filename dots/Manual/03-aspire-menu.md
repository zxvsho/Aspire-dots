# 03 — aspire-menu

Todo lo del menú global vive aquí: qué es, cómo se compila e instala,
cómo se configura (`menu.toml`), cómo funciona por dentro, **niveles**,
qué lanza cada nivel, reglas de Mango y problemas comunes.

---

## Qué es y estado actual

- Binario único en Rust con **Relm4 0.11 + GTK4** como capa **layer-shell**
  (`gtk4-layer-shell`).
- Se lanza **bajo demanda** y muere al usarse. Nada residente.
- Lee `menu.toml`, muestra niveles navegables con búsqueda difusa y ejecuta el script/acción de cada elemento.
- Fuente del código: `~/.dots/aspire/aspire-menu/` (sistema vivo).
- Comparte `aspire-core` (motor de color + lectura de paleta) con `aspire-bg` y `aspire-clip`.

---

## Compilar, probar y desplegar

El `build.sh` de la raíz está reservado a aspire-bar. Despliegue de aspire-menu en `~/.local/bin/aspire-menu`:

```bash
cd ~/.dots/aspire/aspire-menu
cargo build --release
cp target/release/aspire-menu ~/.local/bin/aspire-menu
timeout 3 ~/.local/bin/aspire-menu   # probar: 124 = abre bien
```

Dependencias: toolchain Rust, Relm4 0.11, GTK4 dev (`gtk4`, `glib`, `cairo`,
`pango`) y `gtk4-layer-shell`.

---

## Configuración: `menu.toml`

Fuente: `~/.config/aspire-menu/menu.toml` (fichero vivo, no symlink del repo lab obsoleto). Se lee al abrir; no hace falta recompilar.

Estructura con **niveles**:

```toml
[[main]]
name = "Waybar Studio"
icon = "applications-system-symbolic"
action = "exec"
target = "aspire-bar"

[[main]]
name = "Menú de aplicaciones"
icon = "applications-utilities-symbolic"
action = "exec"
target = "fuzzel --show-files"

[[main]]
name = "Color y fondos"
icon = "preferences-color-symbolic"
[[main.sub]]
name = "Cambiar wallpaper"
action = "exec"
target = "aspire-bg"
[[main.sub]]
name = "Selector de color"
action = "exec"
target = "hyprpicker"

[[main]]
name = "Gestión de archivos"
icon = "folder-symbolic"
[[main.sub]]
name = "Abrir ~/.config"
action = "exec"
target = "nautilus ~/.config"
[[main.sub]]
name = "Abrir ~/.dots"
action = "exec"
target = "nautilus ~/.dots"

[[main]]
name = "Sistema"
icon = "system-shutdown-symbolic"
[[main.sub]]
name = "Apagar"
action = "script"
target = "~/.dots/scripts/poweroff.sh"
[[main.sub]]
name = "Reiniciar"
action = "script"
target = "~/.dots/scripts/reboot.sh"
[[main.sub]]
name = "Suspender"
action = "exec"
target = "systemctl suspend"
```

---

## Niveles del menú (definición)

| Nivel | Qué es | Qué hace | Qué lanza |
|-------|--------|----------|-----------|
| **Principal** | Elementos de raíz (`[[main]]`) | Muestra categorías principales. Cada una puede abrir un submenú o lanzar una acción directa. | `exec` → lanza binario/comando; `script` → ejecuta bash; `menu` → abre subnivel. |
| **Submenú** | Elementos dentro de `[[main.sub]]` | Segundo nivel de navegación. Agrupa acciones relacionadas bajo una categoría. | `exec` o `script`. |
| **Acción** | Hoja de acción (sin hijos) | Ejecuta el comando o script asociado y cierra el menú. | El target definido en `target`. |

**Reglas de acción:**
- `action = "exec"` → `target` es un comando/binario (ej. `aspire-bar`, `fuzzel`).
- `action = "script"` → `target` es una ruta a script bash (ej. `~/.dots/scripts/poweroff.sh`).
- `action = "menu"` → `target` es el nombre de un submenú definido en el mismo archivo.

---

## Scripts Bash integrados

Los scripts de Bash que ya usas y se pueden integrar en la lógica del menú. Guárdalos en `~/.dots/scripts/` y hazlos ejecutables:

### `~/.dots/scripts/poweroff.sh`
```bash
#!/bin/bash
pkexec poweroff
```

### `~/.dots/scripts/reboot.sh`
```bash
#!/bin/bash
pkexec reboot
```

### `~/.dots/scripts/suspend.sh`
```bash
#!/bin/bash
systemctl suspend
```

### `~/.dots/scripts/lock.sh`
```bash
#!/bin/bash
swaylock -f -c 000000
```

### `~/.dots/scripts/restart-waybar.sh`
```bash
#!/bin/bash
pkill waybar
waybar &
```

Dar permisos:
```bash
chmod +x ~/.dots/scripts/*.sh
```

---

## Reglas de MangoWM (binds)

El menú se lanza bajo demanda desde un bind de MangoWM, no es residente.

En `~/.config/mango/bind.toml` (o el archivo de binds activo):

```toml
[[bind]]
mod = ["SUPER"]
key = "space"
action = "exec"
target = "aspire-menu"
```

Cierre automático al usar: el menú se destruye tras ejecutar una acción (`timeout 3` en pruebas).

---

## Cómo aprender cualquier herramienta (método)

1. `programa --help` (o `-h`) para uso rápido.
2. `man programa` para el detalle.
3. Mirar su carpeta en `~/.config/` — los ficheros comentados enseñan más que los wikis.
4. Probarla desde `aspire-menu` o desde terminal antes de atarla a un bind.
