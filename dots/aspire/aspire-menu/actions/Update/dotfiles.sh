#!/bin/bash
# Actualiza/verifica los dotfiles de ASPIRE en una terminal Rio.

rio --app-id aspire-update-sm --title-placeholder "ASPIRE — Actualización de dotfiles" -e bash -c '
cat ~/.dots/assets/aspire-logo.txt
printf "\n"

echo "Verificando dotfiles de ASPIRE..."
echo
DOTS="$HOME/.dots"
if [ -d "$DOTS/.git" ]; then
    cd "$DOTS" && git pull --ff-only
    status=$?
else
    echo "Los dotfiles no son un repositorio git. No hay actualización automática."
    status=0
fi
echo
if [ "$status" -eq 0 ]; then
    notify-send "ASPIRE" "Dotfiles verificados/actualizados correctamente." 2>/dev/null
    echo "Dotfiles listos. Pulsa cualquier tecla para cerrar."
else
    notify-send "ASPIRE" "La actualización de dotfiles falló (código $status)." 2>/dev/null
    echo "Falló la actualización (código $status). Pulsa cualquier tecla para cerrar."
fi
read -n 1 -s
'