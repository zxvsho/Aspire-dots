#!/bin/bash
# Actualiza el sistema (pacman + AUR) en una terminal Rio centrada.

rio --app-id aspire-update --title-placeholder "ASPIRE — Actualización del sistema" -e bash -c '
cat ~/.dots/assets/aspire-logo.txt
printf "\n"

echo "Actualizando su sistema..."
echo
if command -v paru >/dev/null 2>&1; then
    paru -Syu
elif command -v yay >/dev/null 2>&1; then
    yay -Syu
else
    sudo pacman -Syu
fi
status=$?

echo
if command -v flatpak >/dev/null 2>&1; then
    flatpak update -y >/dev/null 2>&1
    fstatus=$?
    if [ "$fstatus" -ne 0 ]; then
        status=$fstatus
    fi
fi
echo
if [ "$status" -eq 0 ]; then
    notify-send "ASPIRE" "Sistema actualizado correctamente." 2>/dev/null
    echo "Sistema actualizado. Pulsa cualquier tecla para cerrar."
else
    notify-send "ASPIRE" "La actualización del sistema falló (código $status)." 2>/dev/null
    echo "La actualización falló (código $status). Pulsa cualquier tecla para cerrar."
fi
read -n 1 -s
'