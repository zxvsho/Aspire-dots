#!/bin/bash
# ============================================================================
# SCRIPT DE INSTALACIÓN AUTOMÁTICA — ASPIRE DOTS
# Reinstala todos los paquetes de pacman y AUR necesarios
# Compila los aspire-tools desde el código fuente
# Restaura todas las configuraciones al sistema
# ============================================================================

set -e

echo "=========================================="
echo "  ASPIRE DOTS — INSTALACIÓN AUTOMÁTICA"
echo "=========================================="
echo ""

# Colores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Verificar si se ejecuta como root
if [ "$EUID" -eq 0 ]; then
  echo -e "${RED}No ejecutes este script como root. Usa tu usuario normal.${NC}"
  exit 1
fi

# Verificar si paru está instalado
if ! command -v paru &> /dev/null; then
  echo -e "${YELLOW}paru no está instalado. Instalando paru...${NC}"
  sudo pacman -S --needed --noconfirm git base-devel
  git clone https://aur.archlinux.org/paru.git /tmp/paru
  cd /tmp/paru
  makepkg -si --noconfirm
  cd -
fi

echo -e "${GREEN}=== [1/5] Instalando paquetes de repositorios oficiales (pacman) ===${NC}"
echo ""

# Paquetes explícitos de pacman
PACMAN_PKGS=(
  "android-tools"
  "aria2"
  "base"
  "base-devel"
  "bluetui"
  "bluez"
  "bluez-utils"
  "brightnessctl"
  "btop"
  "btrfs-assistant"
  "btrfs-progs"
  "cachyos-keyring"
  "cachyos-mirrorlist"
  "cachyos-settings"
  "cachyos-v3-mirrorlist"
  "cachyos-v4-mirrorlist"
  "cava"
  "chromium"
  "clipse"
  "fastfetch"
  "fish"
  "foot"
  "fuzzel"
  "ghostty"
  "gtk-3.0"
  "gtk-4.0"
  "htop"
  "kew"
  "kitty"
  "libfm"
  "mako"
  "mango"
  "matugen"
  "micro"
  "mpv"
  "nautilus"
  "nemo"
  "neofetch"
  "nvim"
  "nwg-look"
  "obs-studio"
  "onlyoffice"
  "opencode"
  "procps-ng"
  "pulseaudio"
  "qalculate-gtk"
  "rio"
  "rofi"
  "thunar"
  "tubetui"
  "vesktop"
  "waybar"
  "waypaper"
  "wlogout"
  "xfce4-terminal"
  "yazi"
  "zed"
  "zim"
)

sudo pacman -S --needed --noconfirm "${PACMAN_PKGS[@]}" 2>&1 | tail -5
echo -e "${GREEN}✓ Paquetes de pacman instalados${NC}"
echo ""

echo -e "${GREEN}=== [2/5] Instalando paquetes de AUR ===${NC}"
echo ""

# Paquetes de AUR
AUR_PKGS=(
  "aurelia-bin"
  "cliamp-bin"
  "fetch-git"
  "freellmapi-desktop-git"
  "fsel-bin"
  "google-chrome"
  "hopper"
  "hydra-launcher-bin"
  "jmtpfs"
  "lavat-git"
  "limusic-bin"
  "mangobar-git"
  "neofetch"
  "peaclock"
  "sherlock-launcher-bin"
  "tinty-git"
  "ttf-ms-fonts"
  "tuios-bin"
  "visual-studio-code-bin"
  "xfce-polkit"
)

paru -S --needed --noconfirm "${AUR_PKGS[@]}" 2>&1 | tail -5
echo -e "${GREEN}✓ Paquetes de AUR instalados${NC}"
echo ""

echo -e "${GREEN}=== [3/5] Instalando fuentes ===${NC}"
echo ""
# Fuentes Nerd Fonts (si no están instaladas)
if ! fc-list | grep -qi "VictorMono Nerd"; then
  echo -e "${YELLOW}VictorMono Nerd Font no encontrada. Instálala manualmente desde nerd-fonts.${NC}"
fi

echo -e "${GREEN}=== [4/5] Compilando aspire-tools (Rust) ===${NC}"
echo ""

# Crear directorio bin si no existe
mkdir -p "$HOME/.dots/bin"
mkdir -p "$HOME/.local/bin"

# Compilar aspire-bg (background changer)
if [ -d "$HOME/.dots/aspire/aspire-bg" ]; then
  cd "$HOME/.dots/aspire/aspire-bg"
  cargo build --release 2>&1 | tail -3
  if [ -f "target/release/aspire-bg" ]; then
    cp target/release/aspire-bg "$HOME/.dots/bin/" 2>/dev/null
    ln -sf "$HOME/.dots/bin/aspire-bg" "$HOME/.local/bin/aspire-bg" 2>/dev/null
    echo -e "${GREEN}✓ aspire-bg compilado y enlazado en ~/.local/bin${NC}"
  fi
  cd - > /dev/null
fi

# Compilar aspire-bar (barra de estado)
if [ -d "$HOME/.dots/aspire/aspire-bar" ]; then
  cd "$HOME/.dots/aspire/aspire-bar"
  cargo build --release 2>&1 | tail -3
  if [ -f "target/release/aspire-bar" ]; then
    cp target/release/aspire-bar "$HOME/.dots/bin/" 2>/dev/null
    ln -sf "$HOME/.dots/bin/aspire-bar" "$HOME/.local/bin/aspire-bar" 2>/dev/null
    echo -e "${GREEN}✓ aspire-bar compilado y enlazado en ~/.local/bin${NC}"
  fi
  cd - > /dev/null
fi

# Compilar aspire-menu (launcher)
if [ -d "$HOME/.dots/aspire/aspire-menu" ]; then
  cd "$HOME/.dots/aspire/aspire-menu"
  cargo build --release 2>&1 | tail -3
  if [ -f "target/release/aspire-menu" ]; then
    cp target/release/aspire-menu "$HOME/.dots/bin/" 2>/dev/null
    ln -sf "$HOME/.dots/bin/aspire-menu" "$HOME/.local/bin/aspire-menu" 2>/dev/null
    echo -e "${GREEN}✓ aspire-menu compilado y enlazado en ~/.local/bin${NC}"
  fi
  cd - > /dev/null
fi

# Compilar aspire-clip (clipboard manager)
if [ -d "$HOME/.dots/aspire/aspire-clip" ]; then
  cd "$HOME/.dots/aspire/aspire-clip"
  cargo build --release 2>&1 | tail -3
  if [ -f "target/release/aspire-clip" ]; then
    cp target/release/aspire-clip "$HOME/.dots/bin/" 2>/dev/null
    ln -sf "$HOME/.dots/bin/aspire-clip" "$HOME/.local/bin/aspire-clip" 2>/dev/null
    echo -e "${GREEN}✓ aspire-clip compilado y enlazado en ~/.local/bin${NC}"
  fi
  cd - > /dev/null
fi

echo ""

echo -e "${GREEN}=== [5/5] Restaurando configuraciones ===${NC}"
echo ""

# Restaurar archivos de shell
cp .zshrc ~/.zshrc 2>/dev/null && echo "✓ .zshrc restaurado"
cp .bashrc ~/.bashrc 2>/dev/null && echo "✓ .bashrc restaurado"
cp .bash_profile ~/.bash_profile 2>/dev/null && echo "✓ .bash_profile restaurado"
cp .gitconfig ~/.gitconfig 2>/dev/null && echo "✓ .gitconfig restaurado"

# Restaurar .config
if [ -d "config" ]; then
  cp -r config/* ~/.config/ 2>/dev/null
  echo "✓ Configuraciones de ~/.config restauradas"
fi

# Restaurar .dots
if [ -d "dots" ]; then
  cp -r dots/* ~/.dots/ 2>/dev/null
  echo "✓ Scripts y binarios de ~/.dots restaurados"
fi

# Restaurar configuraciones de terminal (Rio, Foot, etc.)
if [ -d "config" ]; then
  cp -r config/* ~/.config/ 2>/dev/null
  echo "✓ Configuraciones de ~/.config restauradas (incluye Rio y Foot)"
fi

# Asegurar que ~/.dots/bin esté en PATH
if ! grep -q '\.dots/bin' "$HOME/.zshrc" 2>/dev/null; then
  echo 'export PATH="$HOME/.dots/bin:$PATH"' >> "$HOME/.zshrc"
  echo "✓ PATH actualizado en .zshrc"
fi

echo ""
echo -e "${GREEN}=========================================="
echo "  INSTALACIÓN COMPLETADA"
echo "==========================================${NC}"
echo ""
echo "Terminal principal: Rio (xdg-terminals.list -> rio.desktop)"
echo "Shell: Zsh (/usr/bin/zsh)"
echo "Window Manager: MangoWM / Hyprland"
echo "Aspire Tools: aspire-bg, aspire-bar, aspire-menu, aspire-clip (en ~/.local/bin)"
echo ""
echo "Reinicia tu sesión para aplicar todos los cambios."