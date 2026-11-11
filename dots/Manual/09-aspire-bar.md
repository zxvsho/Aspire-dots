# 09 — aspire-bar

Editor visual de Waybar en Rust + GTK4 para MangoWM. La interfaz mantiene
la estética monoespaciada inspirada en LazyVim, integra los colores de
aspire-core y no usa WebKit.

## Compilar y desplegar

Desde `~/.dots/`:

```bash
./build.sh
timeout 3 ~/.dots/bin/aspire-bar  # 124 = la ventana quedó abierta
./build.sh deploy                 # despliegue explícito a ~/.local/bin
```

La compilación deja el ejecutable en `~/.dots/bin/aspire-bar`; `deploy` lo
copia a `~/.local/bin/aspire-bar`.

## Navegación y vista previa

El riel lateral de pestañas usa iconos simbólicos GTK y permite abrir Módulos,
Ajustes, Temas, Plantillas, Scripts, Editar archivos y Acerca de. La pestaña
Módulos presenta el catálogo a la izquierda, una vista previa persistente en el
centro y un inspector a la derecha con resumen, zonas, módulos activos,
propiedades y acciones. «‹» contrae el panel de secciones/catálogo; el divisor
entre la vista previa y el inspector también se puede ajustar.

El catálogo se organiza en MangoWM, Sistema, Media, Red y Otros. Cada tarjeta
muestra icono, nombre, ID de Waybar, estado y botones para colocar o mover el
módulo a la izquierda, centro o derecha. También se puede arrastrar una tarjeta
a una zona de la vista previa. La búsqueda filtra el catálogo y los módulos
custom ya existentes se agregan a «Otros». Seleccionar un elemento del
catálogo, de «Activos» o de la vista previa sincroniza la selección y el
inspector. La vista previa es esquemática: no ejecuta módulos ni scripts.

La configuración viva de este sistema es Waybar 0.15.0 y MangoWM 0.18.4; usa
`ext/workspaces` para los espacios de trabajo. Waybar no ofrece módulos Mango
nativos confirmados para título de ventana o submapa, por lo que se muestran
como `custom/window-title` y `custom/mango-submap`; requieren un script propio
configurado desde Scripts o las propiedades del módulo. No se usan los IDs
`mango/workspaces`, `mango/window` ni `mango/submap`.

Además de los controles del catálogo y el editor de propiedades adicionales,
cada módulo tiene «Edición manual completa». «Actualizar desde controles»
carga la definición actual al editor JSON; «Aplicar JSON» acepta un objeto
completo, incluidos campos no catalogados como `exec`, `interval`,
`return-type` y `on-click`. El JSON inválido se informa y no modifica el
borrador. Después de editar, guarda el borrador para conservar los cambios.

El selector «Tema» de la cabecera lista los temas predeterminados y personales
con sus colores; elegir uno crea backup, aplica la configuración y recarga
Waybar. «Anterior» vuelve al tema usado antes. El tema actual se muestra
también en el estado de la cabecera. La vista previa central permanece visible
al navegar por las demás secciones; al pasar el cursor sobre una tarjeta de
tema muestra temporalmente ese tema.

«Acerca de» muestra la versión del crate y el commit de compilación cuando se
proporciona `ASPIRE_BAR_GIT_COMMIT` o `GIT_COMMIT`. El repositorio y la
documentación se configuran mediante `ASPIRE_BAR_REPOSITORY_URL` y
`ASPIRE_BAR_DOCUMENTATION_URL`. El diagnóstico se genera en tiempo de
ejecución y se puede copiar; no hay rutas absolutas de usuario codificadas en
el código. Después de editar, guarda el borrador para conservar los cambios.

En «Plantillas» hay tarjetas Minimal, Capsule / Islas, Isla dinámica, Barra
completa, Gaming y Productivity. Cada tarjeta indica categoría, descripción,
iconos de los módulos, recuento `L · C · R`, posición y altura. Al seleccionarla
se reemplazan únicamente las zonas, posición y altura de la barra activa;
se conserva el estilo, la paleta, los módulos personalizados existentes y las
opciones desconocidas. Si el borrador cambió desde el último guardado, se pide
confirmación antes de aplicar la plantilla.

## Temas y paletas

«Temas» separa los predeterminados de «Mis temas». Incluye ocho presets fijos:
Catppuccin Mocha, Catppuccin Latte, Tokyo Night, Gruvbox Dark, Dracula, Nord,
Rosé Pine y Everforest, además del tema dinámico «Isla dinámica». Cada tarjeta
muestra la paleta, una mini barra y `radius / gap / pad`. Los ocho presets
fijos comparten sus layouts de familia cuando corresponde (Mocha y Latte usan
la misma estructura); sus geometrías se definen en
`assets/waybar-themes/defaults.toml`.

El selector de paleta de un tema ofrece primero «Dinámica · wallpaper» y luego
las paletas fijas. Los presets incluidos son de solo lectura: duplícalos para
cambiar su paleta. «Crear nuevo tema» pide un nombre y genera un identificador
seguro y único. Los temas propios se guardan bajo
`~/.config/aspire-bar/themes/<slug>/`; se pueden usar, duplicar, renombrar,
exportar o eliminar. El tema activo no se puede borrar.

Las paletas vienen como archivos independientes:

- Incluidas: `assets/waybar-palettes/palettes/<slug>.toml`.
- Personales: `~/.config/aspire-bar/palettes/<slug>.toml`.

Cada archivo define los colores canónicos `background`, `background-alt`,
`foreground`, `selected`, `selectedtext`, `active` y `urgent`. Añadir una
paleta no requiere cambiar el código. Las fuentes oficiales de las paletas
incluidas están anotadas en cada TOML.

Cuando no hay un tema guardado como activo, se conserva el modo dinámico del
wallpaper. Los temas dinámicos importan `current/colors.css`; los temas fijos
generan sus propias definiciones `@define-color` y no importan encima los
colores dinámicos. «Usar tema» crea un backup, aplica la configuración y
recarga Waybar. También se puede restaurar el último backup. Para mejorar la
legibilidad de estados, `warning` usa el acento `selected`; `critical` y
`urgent` conservan texto legible y marcan el estado con el borde `urgent`.
Los tests comprueban contraste mínimo para texto y estados de los ocho presets,
incluido Catppuccin Latte.

Los backups de los archivos activos de Waybar se guardan bajo
`~/.config/waybar/backups/aspire-bar/`; las configuraciones aplicadas se
conservan en `~/.config/aspire-bar/applied/`.

## Acceso desde MangoWM y aspire-menu

- El bind principal instalado es `SUPER+b`; también se conserva
  `SUPER+ALT+SHIFT+b` como alias. Sus acciones están en
  `~/.config/mango/bind.toml`.
- aspire-menu ofrece «Waybar Studio» desde
  `aspire/aspire-menu/menu.toml`.
- Las acciones de MangoWM usan sus nombres `snake_case`.

## CLI

Los comandos sin UI incluyen `list`, `themes`, `apply <tema>`, `check`,
`restore`, `import`, `export`, `import-theme`, `export-theme` y
`duplicate-theme`. Consulta `aspire-bar --help` para las rutas y argumentos.
La edición inicial importa la configuración viva cuando aún no existe un
borrador en `~/.config/aspire-bar/`.
