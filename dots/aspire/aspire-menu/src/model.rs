use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

static SCRIPT_DIR: OnceLock<String> = OnceLock::new();

pub fn script_dir() -> &'static str {
    SCRIPT_DIR.get_or_init(|| {
        std::env::var("SCRIPT_DIR")
            .unwrap_or_else(|_| format!("{}/.dots/scripts", dirs::home_dir().unwrap().display()))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuTree {
    pub levels: HashMap<String, LevelConfig>,
    pub actions: HashMap<String, ActionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelConfig {
    pub prompt: String,
    pub data: String,
    pub parent: String,
    pub items: Vec<MenuItemConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItemConfig {
    #[serde(rename = "type")]
    pub item_type: String,
    pub label: String,
    pub icon: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub state_key: String,
    #[serde(default)]
    pub state_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    #[serde(rename = "type")]
    pub action_type: String,
    pub cmd: String,
}

impl MenuTree {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read menu.toml")?;
        toml::from_str(&content).context("Failed to parse menu.toml")
    }

    pub fn get_level(&self, name: &str) -> Option<&LevelConfig> {
        self.levels.get(name)
    }

    pub fn get_action(&self, name: &str) -> Option<&ActionConfig> {
        self.actions.get(name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub path: String,
    pub action_type: ActionType,
    pub script_path: Option<String>,
    pub is_submenu: bool,
    pub is_active: bool,
    pub level: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionType {
    Submenu,
    Action,
    Toggle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionResult {
    Navigate,
    ActionExecuted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItemModel {
    pub icon: String,
    pub label: String,
    pub path: String,
    pub is_active: bool,
    pub is_submenu: bool,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub tree: MenuTree,
    pub current_level: String,
    pub search_query: String,
    pub search_mode: bool,
    pub filtered_items: Vec<MenuItem>,
    pub level_stack: Vec<String>,
    pub toggle_states: ToggleStates,
}

impl AppState {
    pub fn new(tree: MenuTree) -> Self {
        Self {
            tree,
            current_level: "root".to_string(),
            search_query: String::new(),
            search_mode: false,
            filtered_items: Vec::new(),
            level_stack: vec!["root".to_string()],
            toggle_states: ToggleStates::load_from_files(),
        }
    }

    pub fn current_prompt(&self) -> String {
        self.tree
            .get_level(&self.current_level)
            .map(|l| l.prompt.clone())
            .unwrap_or_else(|| "MangoWM".to_string())
    }

    pub fn current_level_config(&self) -> Option<&LevelConfig> {
        self.tree.get_level(&self.current_level)
    }

    pub fn go_to(&mut self, level: &str) -> bool {
        if self.tree.get_level(level).is_some() {
            self.level_stack.push(self.current_level.clone());
            self.current_level = level.to_string();
            self.search_query.clear();
            self.update_filtered_items();
            true
        } else {
            false
        }
    }

    pub fn go_back(&mut self) -> bool {
        if self.level_stack.len() > 1 {
            self.level_stack.pop();
            self.current_level = self.level_stack.last().unwrap().clone();
            self.search_query.clear();
            self.update_filtered_items();
            true
        } else {
            false
        }
    }

    pub fn update_filtered_items(&mut self) {
        if self.search_mode && !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            let rank = |item: &MenuItem| -> Option<u8> {
                let label = item.label.to_lowercase();
                if label.starts_with(&query) {
                    Some(0) // el nombre empieza por lo escrito
                } else if label.contains(&query) {
                    Some(1) // el nombre lo contiene
                } else {
                    None
                }
            };

            // Si lo escrito ya nombra algo del nivel actual, nos quedamos con
            // eso: escribir "style" no debe listar lo que hay dentro de Style.
            let own: Vec<MenuItem> = self
                .get_level_items()
                .into_iter()
                .filter(|item| item.label.to_lowercase().contains(&query))
                .collect();
            let candidates = if own.is_empty() {
                self.flatten_tree()
            } else {
                own
            };

            let mut matches: Vec<(u8, MenuItem)> = candidates
                .into_iter()
                .filter_map(|item| rank(&item).map(|r| (r, item)))
                .collect();
            matches.sort_by_key(|(rank, _)| *rank);
            self.filtered_items = matches.into_iter().map(|(_, item)| item).collect();
        } else {
            self.filtered_items = self.get_level_items();
        }
    }

    fn get_level_items(&self) -> Vec<MenuItem> {
        if let Some(level) = self.tree.get_level(&self.current_level) {
            level
                .items
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let is_active = if item.item_type == "toggle" {
                        self.toggle_states.is_on(&item.state_key, &item.state_on)
                    } else {
                        false
                    };

                    MenuItem {
                        id: format!("{}:{}", self.current_level, idx),
                        label: item.label.clone(),
                        icon: item.icon.clone(),
                        path: format!("{} › {}", self.current_prompt(), item.label),
                        action_type: match item.item_type.as_str() {
                            "submenu" => ActionType::Submenu,
                            "action" => ActionType::Action,
                            "toggle" => ActionType::Toggle,
                            _ => ActionType::Action,
                        },
                        script_path: self.tree.get_action(&item.action).map(|a| a.cmd.clone()),
                        is_submenu: item.item_type == "submenu",
                        is_active,
                        level: self.current_level.clone(),
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    fn flatten_tree(&self) -> Vec<MenuItem> {
        let mut items = Vec::new();
        self.collect_flat(&self.current_level, String::new(), &mut items);
        items
    }

    fn collect_flat(&self, level_name: &str, prefix: String, items: &mut Vec<MenuItem>) {
        if let Some(level) = self.tree.get_level(level_name) {
            let new_prefix = if prefix.is_empty() {
                level.prompt.clone()
            } else {
                format!("{} › {}", prefix, level.prompt)
            };

            for (idx, item) in level.items.iter().enumerate() {
                let path = format!("{} › {}", new_prefix, item.label);
                let is_active = if item.item_type == "toggle" {
                    self.toggle_states.is_on(&item.state_key, &item.state_on)
                } else {
                    false
                };

                items.push(MenuItem {
                    id: format!("{}:{}", level_name, idx),
                    label: item.label.clone(),
                    icon: item.icon.clone(),
                    path: path.clone(),
                    action_type: match item.item_type.as_str() {
                        "submenu" => ActionType::Submenu,
                        "action" => ActionType::Action,
                        "toggle" => ActionType::Toggle,
                        _ => ActionType::Action,
                    },
                    script_path: self.tree.get_action(&item.action).map(|a| a.cmd.clone()),
                    is_submenu: item.item_type == "submenu",
                    is_active,
                    level: level_name.to_string(),
                });

                if item.item_type == "submenu" {
                    // El nivel hijo ya aporta su propio prompt. Si la etiqueta
                    // coincide con ese prompt no la repetimos, para no generar
                    // rutas como "Style › Style › Engine".
                    let target_prompt =
                        self.tree.get_level(&item.target).map(|l| l.prompt.as_str());
                    let child_prefix = if target_prompt == Some(item.label.as_str()) {
                        new_prefix.clone()
                    } else {
                        path.clone()
                    };
                    self.collect_flat(&item.target, child_prefix, items);
                }
            }
        }
    }

    pub fn get_display_items(&self) -> Vec<MenuItem> {
        if self.search_mode && !self.search_query.is_empty() {
            self.filtered_items.clone()
        } else {
            self.get_level_items()
        }
    }

    pub fn get_filtered_item(&self, index: usize) -> Option<MenuItem> {
        self.get_display_items().get(index).cloned()
    }

    pub fn is_toggle_on(&self, key: &str, on_value: &str) -> bool {
        self.toggle_states.is_on(key, on_value)
    }

    pub fn toggle(&mut self, key: String, on_value: String) {
        self.toggle_states.toggle(key, on_value);
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToggleStates {
    states: std::collections::HashMap<String, String>,
}

impl ToggleStates {
    pub fn is_on(&self, key: &str, on_value: &str) -> bool {
        self.states.get(key) == Some(&on_value.to_string())
    }

    pub fn toggle(&mut self, key: String, on_value: String) {
        if self.is_on(&key, &on_value) {
            self.states.remove(&key);
        } else {
            self.states.insert(key, on_value);
        }
    }

    pub fn load_from_files() -> Self {
        let mut state = Self::default();
        let home = dirs::home_dir().unwrap();

        if let Ok(content) = std::fs::read_to_string(home.join(".config/color-engine/active")) {
            let engine = content.trim();
            if !engine.is_empty() {
                state
                    .states
                    .insert("color_engine".to_string(), engine.to_string());
            }
        }

        if let Ok(content) = std::fs::read_to_string(home.join(".config/bg-engine/active")) {
            let engine = content.trim();
            if !engine.is_empty() {
                state
                    .states
                    .insert("bg_engine".to_string(), engine.to_string());
            }
        }

        if let Ok(content) = std::fs::read_to_string(home.join(".config/rofi/launcher_activo")) {
            let launcher = content.trim();
            if !launcher.is_empty() {
                state
                    .states
                    .insert("launcher".to_string(), launcher.to_string());
            }
        }

        if let Ok(content) = std::fs::read_to_string(home.join(".config/rofi/powermenu_activo")) {
            let pm = content.trim();
            if !pm.is_empty() {
                state.states.insert("powermenu".to_string(), pm.to_string());
            }
        }

        state
    }
}
