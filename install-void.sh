#!/bin/bash
# ============================================================================
# SCRIPT DE INSTALACIÓN AUTOMÁTICA — ASPIRE DOTS (VOID LINUX)
# Reinstala todos los paquetes disponibles en Void Linux
# Activa repositorios non-free y compila aspire-tools desde Rust
# ============================================================================

set -e

echo "=========================================="
echo "  ASPIRE DOTS — INSTALACIÓN VOID LINUX"
echo "=========================================="
echo ""

# Colores
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Verificar si se ejecuta como root
if [ "$EUID" -eq 0 ]; then
  echo -e "${RED}No ejecutes este script como root. Usa tu usuario normal.${NC}"
  exit 1
fi

# Verificar que estamos en Void Linux
if [ ! -f /etc/void-release ]; then
  echo -e "${RED}Este script es para Void Linux. No se detectó /etc/void-release.${NC}"
  exit 1
fi

echo -e "${GREEN}=== [1/5] Actualizando repositorios y activando non-free ===${NC}"
echo ""

# Sincronizar repositorios
sudo xbps-install -Sy 2>&1 | tail -3

# Activar repositorio non-free
if ! xbps-query -Rs void-repo-nonfree 2>/dev/null | grep -q "void-repo-nonfree"; then
  echo -e "${YELLOW}Instalando void-repo-nonfree...${NC}"
  sudo xbps-install -y void-repo-nonfree 2>&1 | tail -3
fi

# Sincronizar de nuevo después de activar non-free
sudo xbps-install -Sy 2>&1 | tail -3
echo -e "${GREEN}✓ Repositorios sincronizados (incluyendo non-free)${NC}"
echo ""

echo -e "${GREEN}=== [2/5] Instalando paquetes de repositorios oficiales ===${NC}"
echo ""

# Paquetes disponibles en Void Linux (mapeados de Arch)
VOID_PKGS=(
  "7zip"
  "aria2"
  "base-devel"
  "bluez"
  "bluez-utils"
  "brightnessctl"
  "btop"
  "btrfs-progs"
  "cava"
  "chromium"
  "fastfetch"
  "fish"
  "foot"
  "fuzzel"
  "gtk+3"
  "gtk4"
  "htop"
  "kitty"
  "libfm"
  "mako"
  "matugen"
  "micro"
  "mpv"
  "nautilus"
  "neovim"
  "nwg-look"
  "obs"
  "pulseaudio"
  "qalculate-gtk"
  "rio"
  "rofi"
  "thunar"
  "waybar"
  "wlogout"
  "xfce4-terminal"
  "yazi"
  "zed"
)

# Instalar cada paquete, reportando los que no existen
for pkg in "${VOID_PKGS[@]}"; do
  if xbps-query -Rs "^${pkg}$" 2>/dev/null | grep -q "^[^ ]* ${pkg} "; then
    sudo xbps-install -y "$pkg" 2>&1 | tail -1
    echo -e "${GREEN}✓ $pkg instalado${NC}"
  else
    echo -e "${YELLOW}⚠ $pkg no encontrado en repos, omitido${NC}"
  fi
done
echo ""

echo -e "${GREEN}=== [3/5] Instalando paquetes de AUR equivalentes ===${NC}"
echo ""

# Paquetes de AUR → equivalentes en Void
VOID_AUR_PKGS=(
  "aurelia-bin"
  "cliamp-bin"
  "google-chrome"
  "hopper"
  "hydra-launcher-bin"
  "jmtpfs"
  "limusic-bin"
  "neofetch"
  "peaclock"
  "tuios-bin"
  "visual-studio-code-bin"
  "ttf-ms-fonts"
)

for pkg in "${VOID_AUR_PKGS[@]}"; do
  if xbps-query -Rs "^${pkg}$" 2>/dev/null | grep -q "^[^ ]* ${pkg} "; then
    sudo xbps-install -y "$pkg" 2>&1 | tail -1
    echo -e "${GREEN}✓ $pkg instalado${NC}"
  else
    echo -e "${YELLOW}⚠ $pkg no encontrado en repos, omitido${NC}"
  fi
done
echo ""

echo -e "${GREEN}=== [4/5] Instalando fuentes ===${NC}"
echo ""
# Fuentes Nerd Fonts
if ! fc-list | grep -qi "VictorMono Nerd"; then
  echo -e "${YELLOW}VictorMono Nerd Font no encontrada.${NC}"
  echo "Instálala manualmente desde: https://www.nerdfonts.com/"
fi
echo ""

echo -e "${GREEN}=== [5/5] Compilando aspire-tools (Rust) ===${NC}"
echo ""

# Verificar si cargo está instalado
if ! command -v cargo &> /dev/null; then
  echo -e "${YELLOW}Cargo no está instalado. Instalando rust...${NC}"
  sudo xbps-install -y rust cargo 2>&1 | tail -3
fi

mkdir -p "$HOME/.dots/bin"
mkdir -p "$HOME/.local/bin"

# Compilar aspire-bg
if [ -d "$HOME/.dots/aspire/aspire-bg" ]; then
  cd "$HOME/.dots/aspire/aspire-bg"
  cargo build --release 2>&1 | tail -3
  if [ -f "target/release/aspire-bg" ]; then
    cp target/release/aspire-bg "$HOME/.dots/bin/" 2>/dev/null
    ln -sf "$HOME/.dots/bin/aspire-bg" "$HOME/.local/bin/aspire-bg" 2>/dev/null
    echo -e "${GREEN}✓ aspire-bg compilado y enlazado${NC}"
  fi
  cd - > /dev/null
fi

# Compilar aspire-bar
if [ -d "$HOME/.dots/aspire/aspire-bar" ]; then
  cd "$HOME/.dots/aspire/aspire-bar"
  cargo build --release 2>&1 | tail -3
  if [ -f "target/release/aspire-bar" ]; then
    cp target/release/aspire-bar "$HOME/.dots/bin/" 2>/dev/null
    ln -sf "$HOME/.dots/bin/aspire-bar" "$HOME/.local/bin/aspire-bar" 2>/dev/null
    echo -e "${GREEN}✓ aspire-bar compilado y enlazado${NC}"
  fi
  cd - > /dev/null
fi

# Compilar aspire-menu
if [ -d "$HOME/.dots/aspire/aspire-menu" ]; then
  cd "$HOME/.dots/aspire/aspire-menu"
  cargo build --release 2>&1 | tail -3
  if [ -f "target/release/aspire-menu" ]; then
    cp target/release/aspire-menu "$HOME/.dots/bin/" 2>/dev/null
    ln -sf "$HOME/.dots/bin/aspire-menu" "$HOME/.local/bin/aspire-menu" 2>/dev/null
    echo -e "${GREEN}✓ aspire-menu compilado y enlazado${NC}"
  fi
  cd - > /dev/null
fi

# Compilar aspire-clip
if [ -d "$HOME/.dots/aspire/aspire-clip" ]; then
  cd "$HOME/.dots/aspire/aspire-clip"
  cargo build --release 2>&1 | tail -3
  if [ -f "target/release/aspire-clip" ]; then
    cp target/release/aspire-clip "$HOME/.dots/bin/" 2>/dev/null
    ln -sf "$HOME/.dots/bin/aspire-clip" "$HOME/.local/bin/aspire-clip" 2>/dev/null
    echo -e "${GREEN}✓ aspire-clip compilado y enlazado${NC}"
  fi
  cd - > /dev/null
fi

echo ""

echo -e "${GREEN}=== Restaurando configuraciones ===${NC}"
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