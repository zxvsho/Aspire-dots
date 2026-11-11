use serde_json::{json, Map, Value};

#[derive(Clone, Copy, Debug)]
pub enum FieldKind {
    Text,
    Number { min: f64, max: f64 },
    Boolean,
    Json,
}

#[derive(Clone, Debug)]
pub struct FieldSpec {
    pub key: String,
    pub label: String,
    pub description: String,
    pub kind: FieldKind,
    pub default: Value,
}

#[derive(Clone, Debug)]
pub struct ModuleSpec {
    pub id: String,
    pub label: String,
    pub description: String,
    pub fields: Vec<FieldSpec>,
}

fn field(key: &str, label: &str, description: &str, kind: FieldKind, default: Value) -> FieldSpec {
    FieldSpec {
        key: key.to_owned(),
        label: label.to_owned(),
        description: description.to_owned(),
        kind,
        default,
    }
}

fn text(key: &str, description: &str, default: &str) -> FieldSpec {
    field(
        key,
        key,
        description,
        FieldKind::Text,
        Value::String(default.to_owned()),
    )
}

fn number(key: &str, description: &str, default: i64) -> FieldSpec {
    field(
        key,
        key,
        description,
        FieldKind::Number {
            min: 0.0,
            max: 86_400.0,
        },
        json!(default),
    )
}

fn boolean(key: &str, description: &str, default: bool) -> FieldSpec {
    field(
        key,
        key,
        description,
        FieldKind::Boolean,
        Value::Bool(default),
    )
}

fn json_field(key: &str, description: &str, default: Value) -> FieldSpec {
    field(key, key, description, FieldKind::Json, default)
}

fn custom_fields() -> Vec<FieldSpec> {
    vec![
        text(
            "format",
            "Texto mostrado cuando el comando no devuelve texto.",
            "{}",
        ),
        text("exec", "Comando o script ejecutado por Waybar.", ""),
        text(
            "exec-if",
            "Comando opcional que habilita el módulo si termina correctamente.",
            "",
        ),
        number("interval", "Intervalo de actualización en segundos.", 5),
        text(
            "return-type",
            "Formato de salida del comando: plain o json.",
            "json",
        ),
        boolean("escape", "Escapa el texto devuelto por el comando.", true),
        number(
            "signal",
            "Señal que permite solicitar una actualización externa.",
            0,
        ),
        text("on-click", "Comando ejecutado al hacer clic izquierdo.", ""),
        text(
            "on-click-middle",
            "Comando ejecutado al hacer clic central.",
            "",
        ),
        text(
            "on-click-right",
            "Comando ejecutado al hacer clic derecho.",
            "",
        ),
        text(
            "on-scroll-up",
            "Comando ejecutado al desplazar hacia arriba.",
            "",
        ),
        text(
            "on-scroll-down",
            "Comando ejecutado al desplazar hacia abajo.",
            "",
        ),
        boolean("tooltip", "Muestra información emergente.", true),
        text("tooltip-format", "Formato del texto de ayuda.", ""),
    ]
}

fn group_fields() -> Vec<FieldSpec> {
    vec![
        text(
            "orientation",
            "Orientación de los módulos del grupo.",
            "inherit",
        ),
        number("spacing", "Separación entre módulos del grupo.", 0),
        json_field("modules", "Lista JSON de módulos hijos.", json!([])),
        json_field(
            "drawer",
            "Opciones del drawer de Waybar; usa null para quitarlo.",
            json!({
                "transition-duration": 500,
                "transition-left-to-right": false,
                "children-class": "drawer-child"
            }),
        ),
    ]
}

fn module(id: &str, label: &str, description: &str, fields: Vec<FieldSpec>) -> ModuleSpec {
    ModuleSpec {
        id: id.to_owned(),
        label: label.to_owned(),
        description: description.to_owned(),
        fields,
    }
}

pub fn modules() -> Vec<ModuleSpec> {
    let mmsg_click = "mmsg dispatch workspace {index}";
    let mmsg_next = "mmsg dispatch workspace +1";
    let mmsg_previous = "mmsg dispatch workspace -1";
    let mut modules =
        vec![
        module(
            "ext/workspaces",
            "Espacios de trabajo (Mango)",
            "Usa el protocolo ext-workspace-v1 de MangoWM; las acciones personalizadas usan mmsg.",
            vec![
                text("format", "Formato de cada espacio.", "{icon}"),
                boolean("ignore-hidden", "Oculta espacios ocultos.", true),
                text("on-click", "Acción al seleccionar un espacio.", mmsg_click),
                text(
                    "on-click-right",
                    "Acción al hacer clic derecho.",
                    mmsg_click,
                ),
                text(
                    "on-scroll-up",
                    "Acción al desplazar hacia arriba.",
                    mmsg_next,
                ),
                text(
                    "on-scroll-down",
                    "Acción al desplazar hacia abajo.",
                    mmsg_previous,
                ),
            ],
        ),
        module(
            "clock",
            "Reloj",
            "Reloj y calendario de Waybar.",
            vec![
                text("format", "Formato de fecha y hora.", "{:%H:%M}"),
                text(
                    "tooltip-format",
                    "Formato del calendario emergente.",
                    "{calendar}",
                ),
                number("interval", "Intervalo de actualización en segundos.", 60),
                text("locale", "Locale del formato de fecha.", ""),
                text("timezone", "Zona horaria opcional.", ""),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "cpu",
            "CPU",
            "Uso del procesador.",
            vec![
                text("format", "Formato de texto.", "{usage}%"),
                number("interval", "Intervalo de actualización en segundos.", 2),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "memory",
            "Memoria",
            "Uso de RAM y swap.",
            vec![
                text("format", "Formato de texto.", "{used:0.1f}G"),
                number("interval", "Intervalo de actualización en segundos.", 5),
                text("tooltip-format", "Formato de la ayuda emergente.", ""),
            ],
        ),
        module(
            "temperature",
            "Temperatura",
            "Temperatura del sensor seleccionado.",
            vec![
                text("format", "Formato de texto.", "{temperatureC}°C"),
                number("interval", "Intervalo de actualización en segundos.", 10),
                text("hwmon-path", "Ruta opcional del sensor.", ""),
            ],
        ),
        module(
            "disk",
            "Disco",
            "Espacio usado en los sistemas de archivos.",
            vec![
                text("format", "Formato de texto.", "{percentage_used}%"),
                number("interval", "Intervalo de actualización en segundos.", 30),
                text("path", "Punto de montaje.", "/"),
            ],
        ),
        module(
            "network",
            "Red",
            "Estado de la conexión de red.",
            vec![
                text("format-wifi", "Formato cuando hay Wi-Fi.", "{essid}"),
                text(
                    "format-ethernet",
                    "Formato cuando hay Ethernet.",
                    "{ifname}",
                ),
                text(
                    "format-disconnected",
                    "Formato sin conexión.",
                    "Sin conexión",
                ),
                number("interval", "Intervalo de actualización en segundos.", 5),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "bluetooth",
            "Bluetooth",
            "Estado de Bluetooth.",
            vec![
                text("format", "Formato general.", "󰂯"),
                text(
                    "format-connected",
                    "Formato al conectar un dispositivo.",
                    "󰂱",
                ),
                text("format-disabled", "Formato cuando está deshabilitado.", "󰂲"),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "pulseaudio",
            "Audio (PulseAudio)",
            "Volumen y estado de audio.",
            vec![
                text("format", "Formato del volumen.", "{icon} {volume}%"),
                text("format-muted", "Formato al silenciar.", "󰝟"),
                json_field("format-icons", "Iconos por nivel de volumen.", json!([])),
                number("scroll-step", "Cambio de volumen al desplazar.", 5),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "wireplumber",
            "Audio (WirePlumber)",
            "Volumen de WirePlumber.",
            vec![
                text("format", "Formato del volumen.", "{icon} {volume}%"),
                text("format-muted", "Formato al silenciar.", "󰝟"),
                number("scroll-step", "Cambio de volumen al desplazar.", 5),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "battery",
            "Batería",
            "Carga y estados de batería.",
            vec![
                text("format", "Formato de carga.", "{icon} {capacity}%"),
                json_field("states", "Umbrales warning y critical.", json!({})),
                json_field("format-icons", "Iconos ordenados por carga.", json!([])),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "backlight",
            "Brillo",
            "Nivel de brillo de pantalla.",
            vec![
                text("format", "Formato del brillo.", "{icon} {percent}%"),
                json_field("format-icons", "Iconos ordenados por nivel.", json!([])),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "mpris",
            "Reproductor (MPRIS)",
            "Información de reproductores compatibles con MPRIS.",
            vec![
                text("format", "Formato de reproducción.", "{artist} - {title}"),
                number("interval", "Intervalo de actualización en segundos.", 1),
                number("max-length", "Longitud máxima del texto.", 40),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "tray",
            "Bandeja del sistema",
            "Iconos de aplicaciones en segundo plano.",
            vec![
                number("icon-size", "Tamaño de icono en píxeles.", 16),
                number("spacing", "Separación entre iconos.", 8),
            ],
        ),
        module(
            "idle_inhibitor",
            "Evitar suspensión",
            "Indicador para impedir la suspensión por inactividad.",
            vec![
                text("format", "Formato cuando está activo.", "{icon}"),
                json_field("format-icons", "Iconos activo e inactivo.", json!({})),
            ],
        ),
        module(
            "load",
            "Carga del sistema",
            "Promedio de carga del sistema.",
            vec![
                text("format", "Formato de la carga.", "{load1}"),
                number("interval", "Intervalo de actualización en segundos.", 10),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "image",
            "Imagen",
            "Muestra una imagen local como módulo.",
            vec![
                text("path", "Ruta de la imagen.", ""),
                number("size", "Tamaño de la imagen en píxeles.", 24),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "mpd",
            "MPD",
            "Estado y pista actual del servidor MPD.",
            vec![
                text("format", "Formato cuando MPD está activo.", "{artist} - {title}"),
                text("format-disconnected", "Formato sin conexión.", "MPD desconectado"),
                text("server", "Dirección del servidor opcional.", ""),
                number("interval", "Intervalo de actualización en segundos.", 5),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "keyboard-state",
            "Estado del teclado",
            "Indicadores de Caps Lock, Num Lock y Scroll Lock.",
            vec![
                json_field("numlock", "Formato del indicador Num Lock.", json!({})),
                json_field("capslock", "Formato del indicador Caps Lock.", json!({})),
                json_field("scrolllock", "Formato del indicador Scroll Lock.", json!({})),
            ],
        ),
        module(
            "power-profiles-daemon",
            "Perfiles de energía",
            "Perfil de energía de power-profiles-daemon.",
            vec![
                text("format", "Formato del perfil activo.", "{icon}"),
                json_field("format-icons", "Iconos asociados a perfiles.", json!({})),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "privacy",
            "Privacidad",
            "Indicadores de cámara, micrófono y pantalla compartida.",
            vec![
                json_field("icon-spacing", "Opciones de iconos.", json!({})),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "wlr/taskbar",
            "Lista de ventanas (wlroots)",
            "Lista de ventanas; requiere soporte del compositor wlroots.",
            vec![
                number("icon-size", "Tamaño de icono en píxeles.", 16),
                boolean("all-outputs", "Incluye ventanas de todas las salidas.", false),
                text("on-click", "Acción al seleccionar una ventana.", ""),
            ],
        ),
        module(
            "pulseaudio/slider",
            "Control deslizante PulseAudio",
            "Control visual de volumen PulseAudio.",
            vec![number("min", "Volumen mínimo.", 0), number("max", "Volumen máximo.", 100)],
        ),
        module(
            "wireplumber/slider",
            "Control deslizante WirePlumber",
            "Control visual de volumen WirePlumber.",
            vec![number("min", "Volumen mínimo.", 0), number("max", "Volumen máximo.", 100)],
        ),
        module(
            "gamemode",
            "GameMode",
            "Indicador de Feral GameMode.",
            vec![text("format", "Formato cuando GameMode está activo.", "{glyph}")],
        ),
        module(
            "systemd-failed-units",
            "Unidades systemd fallidas",
            "Cuenta servicios systemd fallidos.",
            vec![
                text("format", "Formato de salida.", "{nr_failed}"),
                number("interval", "Intervalo de actualización en segundos.", 60),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ),
        module(
            "sway/workspaces",
            "Espacios Sway",
            "Solo para el compositor Sway; no usar en MangoWM.",
            vec![text("format", "Formato de cada espacio.", "{name}")],
        ),
        module(
            "sway/window",
            "Ventana Sway",
            "Solo para el compositor Sway; no usar en MangoWM.",
            vec![text("format", "Formato del título.", "{title}")],
        ),
        module(
            "hyprland/workspaces",
            "Espacios Hyprland",
            "Solo para Hyprland; no usar en MangoWM.",
            vec![text("format", "Formato de cada espacio.", "{name}")],
        ),
        module(
            "hyprland/window",
            "Ventana Hyprland",
            "Solo para Hyprland; no usar en MangoWM.",
            vec![text("format", "Formato del título.", "{title}")],
        ),
        module(
            "river/tags",
            "Etiquetas River",
            "Solo para el compositor River; no usar en MangoWM.",
            vec![text("format", "Formato de cada etiqueta.", "{name}")],
        ),
        module(
            "river/window",
            "Ventana River",
            "Solo para el compositor River; no usar en MangoWM.",
            vec![text("format", "Formato del título.", "{title}")],
        ),
        module(
            "niri/workspaces",
            "Espacios Niri",
            "Solo para el compositor Niri; no usar en MangoWM.",
            vec![text("format", "Formato de cada espacio.", "{name}")],
        ),
        module(
            "niri/window",
            "Ventana Niri",
            "Solo para el compositor Niri; no usar en MangoWM.",
            vec![text("format", "Formato del título.", "{title}")],
        ),
        module(
            "group/example",
            "Grupo / drawer",
            "Agrupa módulos y permite desplegarlos con el drawer de Waybar.",
            group_fields(),
        ),
        module(
            "custom/new-module",
            "Comando personalizado",
            "Crea módulos exec con formato plain o JSON, eventos y acciones de clic.",
            custom_fields(),
        ),
    ];

    for (id, label, description) in [
        (
            "dwl/tags",
            "Etiquetas dwl",
            "Solo para dwl; no usar en MangoWM.",
        ),
        (
            "dwl/window",
            "Ventana dwl",
            "Solo para dwl; no usar en MangoWM.",
        ),
        ("sway/mode", "Modo Sway", "Solo para el compositor Sway."),
        (
            "sway/scratchpad",
            "Scratchpad Sway",
            "Solo para el compositor Sway.",
        ),
        (
            "sway/language",
            "Idioma Sway",
            "Solo para el compositor Sway.",
        ),
        (
            "sway/keyboard-state",
            "Teclado Sway",
            "Solo para el compositor Sway.",
        ),
        ("hyprland/submap", "Submap Hyprland", "Solo para Hyprland."),
        (
            "hyprland/language",
            "Idioma Hyprland",
            "Solo para Hyprland.",
        ),
        (
            "hyprland/keyboard-state",
            "Teclado Hyprland",
            "Solo para Hyprland.",
        ),
        (
            "river/layout",
            "Layout River",
            "Solo para el compositor River.",
        ),
        ("river/mode", "Modo River", "Solo para el compositor River."),
        (
            "niri/language",
            "Idioma Niri",
            "Solo para el compositor Niri.",
        ),
    ] {
        modules.push(module(
            id,
            label,
            description,
            vec![
                text("format", "Formato del módulo.", "{}"),
                text("on-click", "Comando al hacer clic.", ""),
            ],
        ));
    }

    modules
}

pub fn module_spec(id: &str) -> ModuleSpec {
    if let Some(spec) = modules().into_iter().find(|spec| spec.id == id) {
        return spec;
    }
    if id == "custom/window-title" {
        return module(
            id,
            "Ventana activa (script)",
            "MangoWM no publica un módulo Waybar nativo para el título de ventana; configura aquí un script propio.",
            custom_fields(),
        );
    }
    if id == "custom/mango-submap" {
        return module(
            id,
            "Submapa (script)",
            "MangoWM no publica un módulo Waybar nativo para el submap; configura aquí un script propio.",
            custom_fields(),
        );
    }
    if id.starts_with("custom/") {
        return module(
            id,
            format!("Personalizado: {id}").as_str(),
            "Módulo exec definido por el usuario.",
            custom_fields(),
        );
    }
    if id.starts_with("group/") {
        return module(
            id,
            format!("Grupo: {id}").as_str(),
            "Grupo y drawer de Waybar.",
            group_fields(),
        );
    }
    module(
        id,
        format!("Módulo desconocido: {id}").as_str(),
        "Se conservan todas sus propiedades; edita el objeto JSON completo.",
        Vec::new(),
    )
}

pub fn defaults(spec: &ModuleSpec) -> Value {
    let mut values = Map::new();
    for field in &spec.fields {
        values.insert(field.key.clone(), field.default.clone());
    }
    Value::Object(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mango_workspaces_and_custom_exec_have_schema_defaults() {
        let workspace = module_spec("ext/workspaces");
        let workspace_defaults = defaults(&workspace);
        assert_eq!(
            workspace_defaults["on-click"],
            "mmsg dispatch workspace {index}"
        );

        let custom = module_spec("custom/cava");
        let custom_defaults = defaults(&custom);
        assert_eq!(custom_defaults["return-type"], "json");
        assert_eq!(custom_defaults["interval"], 5);
        assert!(custom.fields.iter().any(|field| field.key == "exec"));
    }
}
