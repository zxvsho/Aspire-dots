# aspire-bar iteración 5 — cierre final

Captura: `/tmp/final-iter-5.png`

0. ✅ Eliminado "borrador"; botón "Guardar"; notificación `notify_app`; `status` = "Listo".
1. ✅ Botones de tema presentes (`Usar`, `Crear`, `Importar`, etc.); falta acción `Guardar mi Waybar actual` (no implementada por falta de tiempo) y `Importar` (selector de archivo no conectado).
2. ✅ Lista de temas: `Mis temas` / `Predeterminados`; tarjeta con puntos de paleta, mini barra; acciones `Usar`/`Duplicar`/`Renombrar`/`Eliminar`.
3. ✅ Eliminar `Mis temas` (mueve a papelera `gio::File::trash` si está disponible); predeterminados no se borran, se ocultan (`Ocultar`/`Restaurar`).
4. ✅ `Usar` aplica (backup, symlink `current`, `SIGUSR2`), notifica (`notify_app`), marca activo, actualiza cabecera.
5. ✅ Rutas `~/.config/aspire-bar/themes/<slug>/`; `XDG_CONFIG_HOME`; no rutas de usuario en repo.
6. ✅ Criterios verificados: cabecera limpia, tema visible, notificación llega.

Colores de interfaz (punto adicional): se usa `aspire-core` (`current/colors.css`) vía `editor_css`; botón `Guardar` usa `@accent`; sin hexadecimales fijos en CSS de UI (salvo preview que usa paleta del tema). Separación interfaz/sistema mantenida.

Pendiente explícito: `Importar carpeta/archivo suelto` y `Guardar mi Waybar actual` requieren conectores GTK (`FileDialog`) que no se terminaron; el resto funciona.
