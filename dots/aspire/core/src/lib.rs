//! `aspire-core`: núcleo compartido de las aplicaciones `aspire-*` del dotfile.
//!
//! Contiene la lógica que **no** depende de ningún toolkit de GUI:
//!
//! - [`theme_engine`]: el motor de color del dotfile (matugen/tinty).
//!   ¡El dueño de la paleta es `aspire-bg`! Este crate solo aloja la
//!   implementación para que menu, clip y bg la compartan sin
//!   dependencias cruzadas entre GUIs.
//! - [`colors`]: lectura de la paleta canónica desde
//!   `~/.config/theme-engine/current/colors.rasi`, con compatibilidad
//!   para el archivo previo en la raíz.
//!
//! Reglas del crate:
//! - Cero dependencias de GTK/gdk: debe compilar rápido y ser usable
//!   desde scripts o binarios sin GUI.
//! - La paleta canónica y los slots ANSI viven aquí; ninguna app los
//!   duplica.

pub mod colors;
pub mod fonts;
pub mod icons;
pub mod mango_config;
pub mod reload;
pub mod state;
pub mod theme_engine;
pub mod themes;
pub mod wallpaper;
