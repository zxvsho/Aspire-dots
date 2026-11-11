#![allow(dead_code, unused_variables, unused_mut)]
pub mod apply;
pub mod catalog;
pub mod css;
pub mod document;
pub mod generator;
pub mod model;
pub mod palette;
pub mod preview;
pub mod scripts;
pub mod style;
pub mod templates;
pub mod themes;
pub mod ui;

use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn config_home() -> Result<PathBuf> {
    dirs::config_dir().context("no se pudo determinar XDG_CONFIG_HOME")
}

pub fn aspire_bar_dir() -> Result<PathBuf> {
    Ok(config_home()?.join("aspire-bar"))
}

pub fn waybar_dir() -> Result<PathBuf> {
    Ok(config_home()?.join("waybar"))
}
