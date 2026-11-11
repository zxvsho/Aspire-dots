#!/bin/bash

set +e

# Portales y Seguridad indispensables
/usr/lib/xdg-desktop-portal-wlr >/dev/null 2>&1 &
/usr/lib/xfce-polkit/xfce-polkit >/dev/null 2>&1 &

# Notificaciones
mako >/dev/null 2>&1 &

# Fondos y colores del escritorio (sustituye a waypaper)
~/.local/bin/aspire-bg --restore &

# Top bar
waybar >/dev/null 2>&1 &

# Foot server para live reload de temas

# Iniciar WOB (Wayland Overlay Bar)
rm -f /tmp/wobpipe
mkfifo /tmp/wobpipe
tail -f /tmp/wobpipe | wob &

# Escala de pantalla para aplicaciones XWayland
echo "Xft.dpi: 140" | xrdb -merge

# Portapapeles (persistencia)
wl-clip-persist --clipboard regular --reconnect-tries 0 >/dev/null 2>&1 &

# Iniciar backend de Walker si no está corriendo
#if ! systemctl --user is-active --quiet elephant.service; then
 #  systemctl --user start elephant.service
  # sleep 1
#fi

# Iniciar Walker como servicio D-Bus
#walker --gapplication-service >/dev/null 2>&1 &
bash ~/.dots/scripts/Utils/status-bar-selector.sh auto &
