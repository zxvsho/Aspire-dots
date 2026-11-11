#!/bin/bash
set -uo pipefail
cd "$(dirname "$(readlink -f "$0")")"

G='\033[0;32m'; Y='\033[1;33m'; R='\033[0;31m'; N='\033[0m'
[ "$EUID" -eq 0 ] && { echo -e "${R}No ejecutes como root.${N}"; exit 1; }

echo -e "${G}[1/5] Sistema base y paru${N}"
sudo pacman -Syu --noconfirm || exit 1
sudo pacman -S --needed --noconfirm git base-devel rust zsh fish || exit 1
if ! command -v paru &>/dev/null; then
  rm -rf /tmp/paru-bin
  git clone https://aur.archlinux.org/paru-bin.git /tmp/paru-bin
  (cd /tmp/paru-bin && makepkg -si --noconfirm) || exit 1
fi

echo -e "${G}[2/5] Paquetes (uno por uno, sin abortar)${N}"
PKGS=(
  # base gráfica / terminales / launchers
  mesa vulkan-icd-loader xdg-terminal-exec xdg-desktop-portal xdg-desktop-portal-gtk
  xdg-desktop-portal-wlr polkit xfce-polkit wl-clipboard grim slurp
  foot rio kitty ghostty xfce4-terminal rofi fuzzel waybar mako wlogout
  # fuentes
  noto-fonts noto-fonts-emoji ttf-nerd-fonts-symbols ttf-victor-mono-nerd ttf-ms-fonts
  # compositor / estilo
  mango mangobar-git swww wallust matugen tinty-git waypaper nwg-look
  gtk3 gtk4 brightnessctl
  # apps
  android-tools aria2 bluetui bluez bluez-utils btop btrfs-assistant btrfs-progs
  cava chromium google-chrome clipse fastfetch htop kew libfm micro mpv nautilus nemo
  neovim obs-studio onlyoffice-bin opencode procps-ng qalculate-gtk thunar tubetui
  vesktop yazi zed zim visual-studio-code-bin
  # AUR varios
  aurelia-bin cliamp-bin fetch-git freellmapi-desktop-git fsel-bin hopper
  hydra-launcher-bin jmtpfs lavat-git limusic-bin peaclock sherlock-launcher-bin tuios-bin
)
failed=()
for p in "${PKGS[@]}"; do
  paru -S --needed --noconfirm "$p" || failed+=("$p")
done

echo -e "${G}[3/5] Restaurando configuraciones (ANTES de compilar)${N}"
[ -d ~/.config ] && cp -a ~/.config ~/.config.bak.$(date +%s)
mkdir -p ~/.config ~/.dots/bin ~/.local/bin
[ -d config ] && cp -a config/. ~/.config/ && echo "✓ config"
[ -d dots ]   && cp -a dots/.   ~/.dots/   && echo "✓ dots"
for f in .zshrc .bashrc .bash_profile .gitconfig; do
  [ -f "$f" ] && cp -a "$f" ~/"$f" && echo "✓ $f"
done
find ~/.dots ~/.config -type f \( -path '*/bin/*' -o -name '*.sh' \) -exec chmod +x {} + 2>/dev/null

# Terminal por defecto para xdg-terminal-exec
[ -f ~/.config/xdg-terminals.list ] || echo "rio.desktop" > ~/.config/xdg-terminals.list

echo -e "${G}[4/5] Compilando aspire-tools${N}"
if [ -d ~/.dots/aspire ]; then
  for d in ~/.dots/aspire/*/; do
    n=$(basename "$d")
    [ -f "$d/Cargo.toml" ] || continue
    if (cd "$d" && cargo build --release); then
      if [ -f "$d/target/release/$n" ]; then
        cp "$d/target/release/$n" ~/.dots/bin/
        ln -sf ~/.dots/bin/"$n" ~/.local/bin/"$n"
        echo "✓ $n"
      else
        failed+=("build:$n (sin binario)")
      fi
    else
      failed+=("build:$n")
    fi
  done
else
  echo -e "${Y}No existe ~/.dots/aspire${N}"
fi

echo -e "${G}[5/5] PATH${N}"
for rc in ~/.zshrc ~/.bashrc; do
  grep -q '\.dots/bin' "$rc" 2>/dev/null || \
    echo 'export PATH="$HOME/.dots/bin:$HOME/.local/bin:$PATH"' >> "$rc"
done
fish -c 'fish_add_path ~/.dots/bin ~/.local/bin' 2>/dev/null

echo ""
if [ ${#failed[@]} -gt 0 ]; then
  echo -e "${R}Fallaron:${N} ${failed[*]}"
else
  echo -e "${G}Todo instalado.${N}"
fi
echo "Reinicia la sesión."
