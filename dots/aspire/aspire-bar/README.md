# aspire-bar

Editor de Waybar para MangoWM, integrado con `aspire-core`.

La interfaz usa un tema oscuro monoespaciado inspirado en LazyVim: contraste
compacto, acento violeta y colores semánticos dinámicos de `aspire-core`.

## Interfaz del editor

Ejecuta `aspire-bar` o `aspire-bar ui` para abrir la interfaz GTK4. La vista
permite seleccionar o crear barras, añadir módulos desde el catálogo,
arrastrarlos entre las zonas izquierda/centro/derecha, reordenarlos,
duplicarlos, quitarlos y editar sus propiedades conocidas y JSON adicionales.
Además del catálogo, el campo de ID libre permite añadir cualquier módulo
Waybar (incluidos namespaces de compositores); los campos desconocidos quedan
disponibles en el editor JSON avanzado y se conservan al guardar.
«Guardar borrador» solo escribe en `~/.config/aspire-bar/`; «Validar» revisa
configuración y CSS sin tocar la barra activa.

## CLI

```text
aspire-bar
aspire-bar import [config.jsonc] [style.css]
aspire-bar check [config.jsonc] [style.css]
aspire-bar list
aspire-bar themes
aspire-bar apply <tema>
aspire-bar export <directorio>
aspire-bar import-theme <carpeta|tema.tar.zst>
aspire-bar export-theme <tema> <directorio|tema.tar.zst>
aspire-bar duplicate-theme <tema> <nombre-nuevo>
aspire-bar save-theme <nombre> <autor> <descripción>
aspire-bar restore
```

La importación por defecto lee `~/.config/waybar/config.jsonc` y
`~/.config/waybar/style.css`, y guarda una copia de trabajo en
`~/.config/aspire-bar/`. No modifica la configuración activa. El parser acepta
JSONC y el modelo conserva bloques y propiedades desconocidos. La salida
normaliza el formato JSON, pero conserva los datos y la forma de una o varias
barras.

`check` valida la configuración y analiza el CSS con `gtk::CssProvider`; no
requiere un display. `apply` busca temas en
`~/.dots/assets/waybar-themes/` y `~/.config/waybar/themes/`, crea una copia
rotativa y publica la configuración aplicada en
`~/.config/aspire-bar/applied/`. Los enlaces de Waybar apuntan a esa copia, de
modo que se conserva el tipo y el destino de los symlinks originales.
`restore` restaura también esos destinos, y solicita la recarga de Waybar con
`SIGUSR2`. Si la recarga de un tema falla, `apply` intenta volver al estado
anterior. `export` escribe `config.jsonc` y `style.css` en la carpeta indicada.

La galería «Temas» permite probar un paquete durante 10 segundos (se confirma
en «Estilo» o se revierte automáticamente), restaurar, guardar el borrador
como tema, duplicar, importar carpetas o archivos `.tar.zst`, y exportar
paquetes. El subcomando `apply` aplica directamente con backup.
`themes` enumera los paquetes de `~/.dots/assets/waybar-themes/`;
los paquetes validan sus módulos requeridos y rechazan colores hexadecimales
fijos salvo autorización expresa en `theme.toml`. Se incluyen siete temas:
minimal, capsule-islands, full-edge, vertical-left, retro-terminal,
compact-bottom y dynamic-island. Sus configuraciones de espacios usan
`ext/workspaces` y acciones `mmsg` de MangoWM. «Temas» y «Plantillas» incluyen
una vista previa esquemática; muestra distribución y módulos sin iniciar
módulos reales.

## Paletas y scripts

El selector «Dinámica · Aspire» importa los colores canónicos de
`~/.config/theme-engine/current/colors.css`. `aspire-core` actualiza ese CSS y
notifica a Waybar con `SIGUSR2` según `~/.config/theme-engine/targets.toml`, por
lo que la barra activa cambia de paleta sin editar aspire-bar. Al elegir una
paleta preconfigurada o personalizada, el tema guarda sus siete colores
canónicos como paleta fija; se puede volver al modo dinámico en cualquier
momento. Las paletas incluidas abarcan Catppuccin (Mocha, Latte, Frappe y
Macchiato), Dracula, Everforest, Gruvbox, Nord y Tokyo Night. Las paletas
personales se guardan en `~/.config/aspire-bar/palettes/`.

La pestaña «Scripts» guarda scripts ejecutables en
`~/.config/aspire-bar/scripts/` y crea backup si se reemplaza un archivo.
Genera un módulo `custom/<nombre>` con intervalo y salida de texto o JSON; el
script no se ejecuta en el editor ni durante la vista previa.

## Editor visual de estilo

La pestaña «Estilo» permite cambiar tipografía (con familias sugeridas y campo
libre), tamaño, paleta Aspire, opacidad, bordes, radio, sombra, padding y
márgenes de módulos, además de separación y márgenes de cada barra. El preset
«Retro terminal» usa una fuente monoespaciada, bordes rectos y colores
canónicos del motor; los colores se refrescan al importar
`~/.config/theme-engine/current/colors.css`.

Los overrides de texto y fondo se guardan por módulo y estado
(`hover`, `warning`, `critical`, `urgent`, `active` y `disabled`). La vista
previa muestra el CSS generado y el editor avanzado conserva acceso al CSS
completo. Los ajustes estructurados se serializan dentro de comentarios
reservados de aspire-bar en el CSS, por lo que sobreviven al guardar, exportar e
importar sin archivos auxiliares.

«Aplicar a prueba» crea un backup, recarga Waybar y revierte a los 10 segundos
si no se confirma. Al confirmar, guarda el mismo borrador en
`~/.config/aspire-bar/`; cerrar la ventana durante una prueba también intenta
revertir.

## Integración en los dotfiles

`~/.dots/aspire/aspire-menu/menu.toml` ofrece «Waybar Studio» en el menú
principal y la copia viva está en `~/.config/rofi-menu/menu.toml`. El bind
`SUPER+b` abre aspire-bar; `SUPER+Alt+b` conserva el selector de barras.

`~/.dots/build.sh` compila en release y copia a `~/.dots/bin/aspire-bar`.
Prueba el staging con `timeout 3 ~/.dots/bin/aspire-bar` (124 significa que la
ventana sigue abierta); solo entonces ejecuta `~/.dots/build.sh deploy` para
copiarlo a `~/.local/bin/aspire-bar`.
