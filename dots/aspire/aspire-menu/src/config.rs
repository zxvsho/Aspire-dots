use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct Config {
    pub menu_config_path: PathBuf,
    pub script_dir: PathBuf,
    pub theme_config_path: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let home = dirs::home_dir().context("No home directory")?;
        
        let menu_config_path = std::env::var("ASPIRE_MENU_CONFIG")
            .unwrap_or_else(|_| format!("{}/.config/rofi-menu/menu.toml", home.display()))
            .into();
        
        let script_dir = std::env::var("SCRIPT_DIR")
            .unwrap_or_else(|_| format!("{}/.dots/scripts", home.display()))
            .into();
            
        let theme_config_path = std::env::var("ASPIRE_THEME_CONFIG")
            .unwrap_or_else(|_| format!("{}/.config/theme-engine", home.display()))
            .into();

        Ok(Self {
            menu_config_path,
            script_dir,
            theme_config_path,
        })
    }
}