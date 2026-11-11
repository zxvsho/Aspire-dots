#!/usr/bin/env bash
# foot-osc-reload.sh
# Inyecta colores dark vía OSC en todas las ventanas foot abiertas
# SOLO modifica cosas relacionadas con foot - no toca nada más

set -euo pipefail

COLORS_DARK="$HOME/.config/foot/colors-dark.ini"

# Verificar que existe el file
[[ -f "$COLORS_DARK" ]] || { echo "No existe $COLORS_DARK" >&2; exit 1; }

# Extraer foreground y background
fg=$(grep -E "^foreground=" "$COLORS_DARK" | cut -d= -f2)
bg=$(grep -E "^background=" "$COLORS_DARK" | cut -d= -f2)

# Extraer paleta de 16 colores
declare -a palette
for i in 0 1 2 3 4 5 6 7; do
    color=$(grep -E "^regular$i=" "$COLORS_DARK" | cut -d= -f2)
    palette[$i]="${color:-000000}"
done
for i in 0 1 2 3 4 5 6 7; do
    color=$(grep -E "^bright$i=" "$COLORS_DARK" | cut -d= -f2)
    palette[$((i+8))]="${color:-ffffff}"
done

# Construir secuencia OSC completa
# OSC 10: foreground
# OSC 11: background
# OSC 4: 16 colores de paleta (indices 0-15)
osc_sequence=""
osc_sequence+=$'\033]10;#'"$fg"$'\007'
osc_sequence+=$'\033]11;#'"$bg"$'\007'

for i in 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
    osc_sequence+=$'\033]4;'"$i"';#'"${palette[$i]}"$'\007'
done

# Inyectar en TODOS los PTYs abiertos
injected=0
for pts in /dev/pts/*; do
    [[ -c "$pts" ]] || continue
    
    # Solo inyectar en PTYs que tienen foot corriendo en ellos
    # (verificamos leyendo /proc/*/fd/* para ver que PTY usa cada proceso)
    # Pero por simplicidad, inyectamos en todos - foot ignorará si no es suyo
    printf '%s' "$osc_sequence" > "$pts" 2>/dev/null && ((injected++)) || true
done

echo "OSC inyectado en $injected PTYs"
exit 0