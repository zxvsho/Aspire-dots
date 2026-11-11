#!/usr/bin/env bash
# Status Bar Selector - Utils/Status Bar
# Permite elegir entre waybar y mangobar

set -euo pipefail

SCRIPT_DIR="$HOME/.dots/scripts"
STATE_FILE="$HOME/.config/mango/status-bar-choice"

# Funciones de ayuda
notify() {
    notify-send -u normal -a "MangoWM" "Status Bar" "$1"
}

# Obtener elección actual
get_current() {
    if [[ -f "$STATE_FILE" ]]; then
        cat "$STATE_FILE"
    else
        echo "waybar"  # default
    fi
}

# Guardar elección
save_choice() {
    echo "$1" > "$STATE_FILE"
}

# Iniciar waybar
start_waybar() {
    pkill -9 waybar 2>/dev/null
    sleep 0.3
    waybar >/dev/null 2>&1 &
    disown
    notify "waybar iniciado"
}

# Iniciar mangobar
start_mangobar() {
    pkill -9 mangobar 2>/dev/null
    sleep 0.3
    mangobar >/dev/null 2>&1 &
    disown
    notify "mangobar iniciado"
}

# Detener ambas
stop_bars() {
    pkill -9 waybar 2>/dev/null
    pkill -9 mangobar 2>/dev/null
}

# Cambiar a waybar
switch_waybar() {
    stop_bars
    start_waybar
    save_choice "waybar"
}

# Cambiar a mangobar
switch_mangobar() {
    stop_bars
    start_mangobar
    save_choice "mangobar"
}

# Menú principal
main() {
    CURRENT=$(get_current)
    
    opciones="waybar\nmangobar"
    if [[ "$CURRENT" == "waybar" ]]; then
        opciones="🟢 waybar (actual)\nmangobar"
    else
        opciones="waybar\n🟢 mangobar (actual)"
    fi
    
    eleccion=$(echo -e "$opciones" | rofi -dmenu -i -p "Status Bar:" -theme ~/.config/rofi/launchers/selector_menu/style.rasi)
    
    case "$eleccion" in
        "waybar"|"🟢 waybar (actual)")
            switch_waybar
            ;;
        "mangobar"|"🟢 mangobar (actual)")
            switch_mangobar
            ;;
        *)
            exit 0
            ;;
    esac
}

# Auto-start al logearse (lee la preferencia guardada)
auto_start() {
    CHOICE=$(get_current)
    if [[ "$CHOICE" == "mangobar" ]]; then
        start_mangobar
    else
        start_waybar
    fi
}

case "${1:-}" in
    auto)
        auto_start
        ;;
    waybar)
        switch_waybar
        ;;
    mangobar)
        switch_mangobar
        ;;
    *)
        main
        ;;
esac