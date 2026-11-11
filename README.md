# Aspire Dots — Mis dotfiles para Mangowm y Arch Linux

Backup de configuraciones antes de reinstalar Arch Linux.

## 📦 Paquetes instalados (pacman -Qe)

```
android-tools 37.0.0.p1-1.1
aria2 1.37.0-3.1
aurelia-bin 0.2.3-1
base 3-3
base-devel 1-2
bluetui 0.8.2-1.1
bluez 5.87-2.1
bluez-utils 5.87-2.1
brightnessctl 0.5.1-3
btop 1.4.7-1.1
btrfs-assistant 3.0.0-1.1
btrfs-progs 7.3-1
cachyos-keyring 20240331-1
cachyos-mirrorlist 27-1
cachyos-settings 1:1.4.4-1
cachyos-v3-mirrorlist 27-1
cachyos-v4-mirrorlist 27-1
cava 1.0.0-1.1
chromium 156.0.8078.79-1.1
cliamp-bin 2.4.0-1
clipse 1.3.0-1.1
color-engine 1.0.0-1
com.limusic.desktop 1.0.0-1
datarig 0.1.0-1
dunst 1.13.1-1.1
fastfetch 2.62.1-1.1
fish 4.1.3-1.1
foot 1.27.0-1.1
fuzzel 1.14.1-1.1
ghostty 1.2.3-1.1
gtk-3.0 3.24.52-1.1
gtk-4.0 4.20.5-1.1
herdr 0.1.0-1
hopper 5.19.4-1.1
htop 3.4.1-1.1
hydralauncher 3.0.0-1.1
kew 4.1.0-1.1
kitty 0.44.0-1.1
libfm 1.4.0-1.1
mako 1.10.0-1.1
mango 0.12.5-1.1
mangobar 0.1.0-1
matugen 4.1.0-1.1
micro 2.0.15-1.1
mpv 0.40.0-1.1
nautilus 49.2-1.1
nemo 6.6.4-1.1
neofetch 7.1.0-1.1
net.imput.helium 0.9.2-1.1
nvim 0.11.5-1.1
nwg-look 1.0.6-1.1
obs-studio 32.0.4-1.1
onlyoffice 9.2.1-1.1
opencode 1.0.0-1.1
procps-ng 4.0.5-1.1
pulseaudio 17.0-1.1
qalculate-gtk 5.8.1-1.1
rio 0.2.37-1.1
rofi 2.0.0-1.1
thunar 4.20.7-1.1
tubetui 0.12.0-1.1
vesktop 1.6.5-1.1
waybar 0.15.0-1.1
waypaper 2.8.0-1.1
wlogout 1.2.2-1.1
xfce4-terminal 1.1.5-1.1
yazi 26.1.22-1.1
zed 0.216.0-1.1
zim 0.76.1-1.1
```

## 📦 Paquetes de AUR (20)

```
aurelia-bin 0.2.3-1
cliamp-bin 2.4.0-1
fetch-git 1.0.0-1
freellmapi-desktop-git 0.1.0-1
fsel-bin 0.1.0-1
google-chrome 142.0.7444.175-1
hopper 5.19.4-1.1
hydra-launcher-bin 3.0.0-1.1
jmtpfs 0.5-3
lavat-git 2.5.0-1
limusic-bin 1.0.0-1
mangobar-git 0.1.0-1
neofetch 7.1.0-1.1
peaclock 0.8.2-1
sherlock-launcher-bin 0.1.0-1
tinty-git 0.2.0-1
ttf-ms-fonts 2.0-12
tuios-bin 0.4.0-1
visual-studio-code-bin 1.105.1-1
xfce-polkit 0.3-1
```

## 🖥️ Terminal principal

**Rio Terminal** (`rio.desktop` en `xdg-terminals.list`)

Configurado con:
- Fuente: VictorMono Nerd Font (size 14)
- Opacidad: 0.98 con blur
- Shell: Zsh
- Multiplexación con Alt+n (split right) / Alt+m (split down)
- Navegación entre splits con Alt+hjkl

## 🛠️ Aspire Tools (compilados desde Rust)

Los binarios se compilan desde `~/.dots/aspire/` usando `cargo build --release`:

| Herramienta | Directorio fuente | Binario |
|-------------|-------------------|---------|
| aspire-bg | `~/.dots/aspire/aspire-bg/` | `~/.dots/bin/aspire-bg` |
| aspire-bar | `~/.dots/aspire/aspire-bar/` | `~/.dots/bin/aspire-bar` |
| aspire-menu | `~/.dots/aspire/aspire-menu/` | `~/.dots/bin/aspire-menu` |
| aspire-clip | `~/.dots/aspire/aspire-clip/` | `~/.dots/bin/aspire-clip` |

### Requisitos para compilar:
- Rust toolchain (rustc, cargo)
- `cargo build --release` en cada directorio

### Symlinks en PATH:
El script crea enlaces simbólicos en `~/.local/bin/` para que los aspire-tools estén disponibles en el PATH del usuario.

## 📖 Manual completo

El directorio `Manual/` contiene documentación detallada de cada componente:
- `01-herramientas-y-dotfile.md` — Herramientas y dotfile
- `02-estructura-general.md` — Estructura general
- `03-aspire-menu.md` — Aspire Menu
- `04-aspire-clip.md` — Aspire Clip
- `05-aspire-bg.md` — Aspire BG
- `06-aspire-files.md` — Aspire Files
- `07-theme-engine.md` — Theme Engine
- `08-aspire-music.md` — Aspire Music
- `09-aspire-bar.md` — Aspire Bar
- `aspire-themes-spec.md` — Especificación de temas
- `aspire-context.md` — Contexto de Aspire
- `aspire-core-arquitectura-prompt.md` — Arquitectura core

## 📦 Scripts de instalación automática

### Para Arch Linux (CachyOS)
El archivo `install.sh` instala automáticamente:
- Todos los paquetes de pacman (161 paquetes)
- Todos los paquetes de AUR (20 paquetes) usando paru
- Fuentes Nerd Fonts (verificación)
- Compila los aspire-tools desde Rust
- Crea symlinks en `~/.local/bin` para que estén en el PATH
- Actualiza `~/.zshrc` con `~/.dots/bin` en el PATH
- Restaura los dotfiles

### Uso:
```bash
chmod +x install.sh
./install.sh
```

### Para Void Linux
El archivo `install-void.sh` instala automáticamente:
- Activa repositorios non-free (`void-repo-nonfree`)
- Paquetes equivalentes de Void Linux (xbps)
- Paquetes de AUR equivalentes disponibles en Void
- Fuentes Nerd Fonts (verificación)
- Compila los aspire-tools desde Rust
- Crea symlinks en `~/.local/bin`
- Restaura los dotfiles

### Uso:
```bash
chmod +x install-void.sh
./install-void.sh
```

### Diferencias entre Arch y Void:
| Aspecto | Arch (CachyOS) | Void Linux |
|---------|---------------|------------|
| Gestor de paquetes | pacman / paru | xbps / xbps-install |
| Repositorios non-free | Activados por defecto | Requiere `void-repo-nonfree` |
| AUR | Disponible con paru/yay | No disponible, paquetes en repos oficiales |
| Init system | systemd | runit |
| Kernel | CachyOS kernels | Void kernels (linux, linux-lts) |

## 🛠️ Servicios de sistema (MariaDB estaba activo)

```bash
sudo systemctl stop mariadb
sudo systemctl disable mariadb
```

## 📁 Estructura del backup

```
dots-backup/
├── .zshrc
├── .bashrc
├── .bash_profile
├── .gitconfig
├── config/          # ~/.config (excluyendo navegadores, caches pesados)
└── dots/            # ~/.dots (scripts, bin, Manual, build.sh, aspire-tools source)
```

### Configuraciones incluidas en config/ (excluyendo navegadores y editores pesados):
alacritty, aspire-bar, aspire-bg, aspire-files, btop, cava, color-engine, dunst, fastfetch, fish, fontconfig, foot, fuzzel, gh, ghostty, gtk-3.0, gtk-4.0, herdr, hopper, htop, kitty, libfm, mango, mangobar, matugen, menus, micro, mako, matugen, neofetch, nvim, nwg-look, obs-studio, opencode, rofi, rofi-menu, sherlock, steam-tui, theme, theme-engine, Thunar, tinted-theming, tinty, tubetui, tuios, waybar, waypaper, wlogout, xfce4, xsettingsd, yazi, zed, zim, y más.

## 🔄 Restauración

### Opción 1: Script automático (recomendado)
```bash
git clone https://github.com/zxvsho/Aspire-dots.git ~/dots-restore
cd ~/dots-restore
chmod +x install.sh
./install.sh
```

### Opción 2: Manual
```bash
git clone https://github.com/zxvsho/Aspire-dots.git ~/dots-restore
cp -r ~/dots-restore/* ~/
cp -r ~/dots-restore/config/* ~/.config/
```

---

## ⚠️ Nota importante

El directorio `aspire/aspire-bg/target/` (builds de Rust, ~15GB) se excluyó del backup para ahorrar espacio. El script `install.sh` lo recompila automáticamente.

---

**Backup creado antes de reinstalar Arch Linux.**