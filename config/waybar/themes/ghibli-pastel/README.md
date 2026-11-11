# Tema Ghibli Pastel para Waybar (MangoWM)

Tema instalado desde `mangowm-waybar-ghibli-theme`, adaptado para este equipo.
Tu configuración actual (tema `capsule-bar`) **no fue modificada ni borrada**.

## Contenido

- `config.jsonc` — barra con cápsulas pastel (distro, workspaces, reloj, CPU,
  audio, Bluetooth, red, batería, fecha y apagado).
- `style.css` — paleta Ghibli pastel (amarillo, cian, lavanda, menta, rosa).

## Cambios respecto al tema original

- Workspaces: `wlr/workspaces` + `hyprland/workspaces` → `ext/workspaces`
  con `mmsg`, que es lo que usa MangoWM aquí.
- Acciones ajustadas a programas instalados:
  - Red → `impala` (en vez del inexistente `wifi-menu.sh`)
  - Audio → `wiremix` (en vez de `pavucontrol`)
  - Bluetooth → `bluetui` (en vez de `blueman-manager`)
  - CPU → `btop`, Apagado → `wlogout`, menú apps → `rofi -show drun`
- Altura 36 px para evitar el aviso de altura mínima de Waybar.
- Se quitó `filter: brightness()` (GTK3 no lo soporta).
- No se instalaron los archivos de rofi, dunst ni mangowm del paquete original.

## Cómo aplicarlo

Con el selector de temas (rofi), que reapunta solo `config.jsonc` y `style.css`:

```bash
~/.config/waybar/core-bar/selector-theme-bar.sh
```

Elige `ghibli-pastel`. Para volver atrás, ejecuta el mismo selector y elige
`capsule-bar`.

También puedes probarlo sin tocar tu configuración actual:

```bash
waybar -c ~/.config/waybar/themes/ghibli-pastel/config.jsonc \
       -s ~/.config/waybar/themes/ghibli-pastel/style.css
```

(cierra esa instancia con `pkill waybar`; tu barra normal vuelve al reiniciar
la sesión o con `waybar &`).
