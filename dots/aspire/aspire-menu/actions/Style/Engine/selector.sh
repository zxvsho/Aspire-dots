#!/bin/bash
SCRIPT_DIR="$HOME/.dots/aspire/aspire-menu"
choice=$(printf "matugen\ntinty\n" | rofi -dmenu -p "Motor de color" -theme ~/.config/rofi/launchers/selector_menu/style.rasi)

if [[ -z "$choice" ]]; then
    bash "$SCRIPT_DIR/rofi_style_selector.sh"
    exit 0
fi

exec ~/.local/bin/aspire-bg --set-engine "$choice"
