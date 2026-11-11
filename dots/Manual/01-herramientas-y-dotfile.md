# 01 — Herramientas del dotfile y descripción general

Cómo es este dotfile, qué programas lo forman, para qué sirve cada uno,
dónde vive su configuración y dónde aprender a usarlos.

---

## Qué es este dotfile

Dotfiles para **Arch Linux (y derivados)** con el compositor **MangoWM**
en Wayland. Filosofía:

- **Modular y script-based**: la lógica vive en `.dots/scripts/` (legacy) y
  en los crates `aspire-*` (Rust). La configuración pura en `~/.config/<app>`.
  Cada programa hace una cosa, la hace y termina.
- **Nada residente de cosecha propia**: lo único que corre toda la sesión
  son componentes del sistema (compositor, barra, notificaciones). El menú
  (`aspire-menu`) se lanza bajo demanda y muere al usarse. Lo mismo
  `aspire-clip` y `aspire-bg` y los demás aspire-tools.
- **Color centralizado**: el wallpaper manda. `matugen` o `tinty`
  generan la paleta y el resto de apps la leen (ver parte 02).
- **Un solo menú global**: `aspire-menu` (parte 03). Puedes elegir entre "rofi" o "fuzzel"
para el lanzador rápido de aplicaciones (`Alt+espacio`); `rofi` para
  powermenu y selectores de estilo.
- **Gestor de archivos propio**: `aspire-files` (parte 06), inspirado en
  Nautilus, sin dependencias de GNOME, tematizable con Matugen/Tinty.

## Los archivos de los Dotfiles:

| Carpeta | Contenido |
|---|---|
| `~/.config/<app>` | Config de cada programa que usa el dotfile |
| `~/.dots/` | Lógica y recursos propios: `Manual/`, `aspire-tools/` (crates Rust), `scripts/` (legacy), `assets/` |


### Compositor y sesión

| Compositor: Mangowm (cambiar la configuración tipo config a toml)
| 
### Lanzadores y menú

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **aspire-menu** | Menú global del dotfile (parte 03) | `menu.toml` (ver parte 03) | Parte 03 de este Manual |
| **rofi-wayland** | Powermenus, launchers, applets y selectores de estilo (`rofi -dmenu`) | `~/.config/rofi/` | `man rofi` |

### Color y fondo

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **aspire-bg** | Selector de fondos y motor de color integrado (matugen/tinty) | `~/.config/aspire-bg/` | `aspire-bg --help`, parte 05 |
| **aspire-core** | Crate compartido: motor de color y lectura de la paleta | `~/.dots/aspire/core/` | `cargo doc` en el crate |
| **matugen** | Genera la paleta desde el wallpaper | `~/.config/matugen/` (+ plantillas) | `matugen --help` |
| **tinty** | Gestor y aplicador de temas | `~/.config/tinty/`, temas en `~/.local/share/tinted-theming/` | `tinty --help`, `tinty list` |

### Gestor de archivos (D)

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **aspire-files** | Gestor de archivos en Rust, tematizable | `~/.config/aspire-files/config.toml` | Parte 06, README del crate |

### Música en terminal (D)

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **aspire-music** | Reproductor de música TUI (Ratatui+Tokio; motor MPV + yt-dlp) | `aspire-music/config.toml` (en el crate) | Parte 08 |
# Tambien puedes instalar algún otro reproductor diferente desde: aspire-menu/Install/



### Terminales y shell (D)

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **Rio Terminal** | Terminal nativa Wayland (principal) | `~/.config/foot/` | `man foot`, `man foot.ini` |
| **kitty** | Terminal alternativa | `~/.config/kitty/` | `kitty --help`, `man kitty` |
| **alacritty** | Terminal alternativa | `~/.config/alacritty/` | `man alacritty` |
| **ghostty** | Terminal alternativa | `~/.config/ghostty/` | `ghostty --help` |
| foot | Terminal alternativa | ~/.config/foot/foot.ini


### Audio, red, hardware (D)

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **pipewire** (+ wireplumber) | Audio | — | `wpctl status`, `man wpctl` |
| **pamixer / wpctl** | Volumen desde scripts/atajos | — | `pamixer --help` |
| **brightnessctl** | Brillo desde scripts/atajos | — | `brightnessctl --help` |
| **iwd / impala** | WiFi (CLI/TUI) | — | `man iwd`, `impala --help` |
| **bluetui / bluez** | Bluetooth | — | `bluetui --help` |
| **wiremix** | Mezclador TUI de PipeWire | — | `wiremix --help` |

### Captura y utilidades (D)

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **grim + slurp** | Capturas (pantalla / región) | — | `man grim`, `man slurp` |
| **wl-clipboard** (`wl-copy`) | Portapapeles en Wayland | — | `man wl-copy` |
| **cliphist** | Historial del portapapeles | — | `cliphist --help` |
| **clipse** | Daemon de portapapeles (usado por aspire-clip) | `~/.config/clipse/` | `clipse --help` |
| **swaylock** | Bloqueo de pantalla | `~/.config/swaylock/` (si existe) | `man swaylock` |
| **yazi** | Gestor de archivos en terminal | `~/.config/yazi/` (si existe) | `yazi --help`, `?` dentro |
| **thunar / pcmanfm** | Gestores gráficos | `~/.config/Thunar/`, `~/.config/pcmanfm/` | — |
| **btop / htop** | Monitor del sistema | `~/.config/btop/` | `man btop` |
| **fastfetch / neofetch** | Info del sistema | `~/.config/fastfetch/`, `~/.config/neofetch/` | `man fastfetch` |
| **peaclock** | Reloj en terminal | — | `peaclock --help` |
| **cava** | Visualizador de audio | `~/.config/cava/` | `man cava` |
| **mpv** | Reproductor | `~/.config/mpv/` | `man mpv` |
| **rofi-calc / rofi-emoji** | Calculadora y emojis vía rofi | — | Dentro del propio rofi |

### Editores y apps (D)

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **neovim** | Editor principal | `~/.config/nvim/` | `:Tutor` |
| **micro** | Editor simple | `~/.config/micro/` | `Ctrl+G` dentro |
| **zed** | Editor gráfico | `~/.config/zed/` (si existe) | — |
| **firefox / chromium** (+ forks: librewolf, helium) | Navegadores | Perfiles propios | — |

### Fuentes y theming (base)

| Programa | Para qué | Config | Aprender |
|---|---|---|---|
| **nerd-fonts** (JetBrainsMono, etc.) | Iconos y glifos del menú y la barra | `~/.config/fontconfig/` | `fc-list \| grep -i nerd` |
| **gtk-3.0 / gtk-4.0** | Tema GTK | `~/.config/gtk-3.0/`, `~/.config/gtk-4.0/` | `nwg-look` para previsualizar |
| **qt5ct / qt6ct / kvantum** | Tema Qt | `~/.config/qt*/`, `~/.config/Kvantum/` | — |
| **nwg-look** | Ajustes GTK/Qt con GUI | — | — |

### Gestión del sistema

| Programa | Para qué | Aprender |
|---|---|---|
| **pacman + yay/paru** | Paquetes oficiales y AUR | `man pacman`, `yay --help` |
| **git** | Versionado del dotfile y `Update → Dotfiles` | `man git` |
| **flatpak / appimage** | Apps extra (`Install → Store`) | `flatpak --help` |

---

## Cómo aprender cualquier herramienta (método)

1. `programa --help` (o `-h`) para uso rápido.
2. `man programa` para el detalle.
3. Mirar su carpeta en `~/.config/` — los ficheros comentados enseñan más que los wikis.
4. Probarla desde el menú (`aspire-menu`) o desde terminal antes de atarla a un bind.
