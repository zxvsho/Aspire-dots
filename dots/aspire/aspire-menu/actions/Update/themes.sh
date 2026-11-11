#!/bin/bash
# Actualiza/reinstala los temas base de ASPIRE en una terminal Rio.

rio --app-id aspire-update-sm --title-placeholder "ASPIRE — Actualización de temas" -e bash -c '
cat ~/.dots/assets/aspire-logo.txt
printf "\n"

echo "Actualizando temas de ASPIRE..."
echo
# Reinstalar temas base si existe el directorio de origen
SRC="$HOME/.dots/assets/themes"
DST="$HOME/.local/share/aspire/themes"
if [ -d "$SRC" ]; then
    mkdir -p "$DST"
    cp -r "$SRC"/* "$DST"/ 2>/dev/null
    echo "Temas base sincronizados."
else
    echo "No se encontró el directorio de temas base."
fi
notify-send "ASPIRE" "Temas actualizados." 2>/dev/null
echo
echo "Pulsa cualquier tecla para cerrar."
read -n 1 -s
'