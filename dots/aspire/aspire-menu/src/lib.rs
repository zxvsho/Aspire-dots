// El motor de color vive en `aspire-core` (crate compartido, sin GTK).
// `menu_logic` lo re-exporta para mantener la compatibilidad de imports.

pub mod menu_logic;
pub mod model;
pub mod search;

pub use model::{
    ActionConfig, ActionType, AppState, LevelConfig, MenuItem, MenuItemConfig, MenuTree,
};
pub use search::FuzzySearch;
