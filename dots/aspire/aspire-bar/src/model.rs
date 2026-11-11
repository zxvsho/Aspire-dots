use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BarConfig {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
    #[serde(rename = "name", default)]
    pub name: Option<Value>,
    #[serde(rename = "position", default)]
    pub position: Option<Value>,
    #[serde(rename = "layer", default)]
    pub layer: Option<Value>,
    #[serde(rename = "height", default)]
    pub height: Option<Value>,
    #[serde(rename = "spacing", default)]
    pub spacing: Option<Value>,
    #[serde(rename = "margin-top", default)]
    pub margin_top: Option<Value>,
    #[serde(rename = "margin-left", default)]
    pub margin_left: Option<Value>,
    #[serde(rename = "margin-right", default)]
    pub margin_right: Option<Value>,
    #[serde(rename = "margin-bottom", default)]
    pub margin_bottom: Option<Value>,
    #[serde(rename = "modules-left", default)]
    pub modules_left: Option<Vec<String>>,
    #[serde(rename = "modules-center", default)]
    pub modules_center: Option<Vec<String>>,
    #[serde(rename = "modules-right", default)]
    pub modules_right: Option<Vec<String>>,
}

impl BarConfig {
    pub fn zones(&self) -> Vec<Vec<String>> {
        vec![
            self.modules_left.clone().unwrap_or_default(),
            self.modules_center.clone().unwrap_or_default(),
            self.modules_right.clone().unwrap_or_default(),
        ]
    }
    pub fn zone(&self, index: usize) -> Option<Vec<String>> {
        match index {
            0 => self.modules_left.clone(),
            1 => self.modules_center.clone(),
            2 => self.modules_right.clone(),
            _ => None,
        }
    }
    pub fn zone_mut(&mut self, index: usize) -> Option<&mut Vec<String>> {
        match index {
            0 => self.modules_left.as_mut(),
            1 => self.modules_center.as_mut(),
            2 => self.modules_right.as_mut(),
            _ => None,
        }
    }
    pub fn module_definition(&self, name: &str) -> Option<&Value> {
        self.extra.get(name)
    }
    pub fn module_count(&self) -> usize {
        self.modules_left.as_ref().map(|v| v.len()).unwrap_or(0)
            + self.modules_center.as_ref().map(|v| v.len()).unwrap_or(0)
            + self.modules_right.as_ref().map(|v| v.len()).unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WaybarConfig {
    Single(Box<BarConfig>),
    Multiple(Vec<BarConfig>),
}

impl Default for WaybarConfig {
    fn default() -> Self {
        WaybarConfig::Single(Box::default())
    }
}

impl WaybarConfig {
    pub fn bars(&self) -> &[BarConfig] {
        match self {
            WaybarConfig::Single(b) => std::slice::from_ref(b.as_ref()),
            WaybarConfig::Multiple(v) => v.as_slice(),
        }
    }
    pub fn bar_mut(&mut self, index: usize) -> Option<&mut BarConfig> {
        match self {
            WaybarConfig::Single(b) if index == 0 => Some(b.as_mut()),
            WaybarConfig::Multiple(v) => v.get_mut(index),
            _ => None,
        }
    }
    pub fn push_bar(&mut self) {
        match self {
            WaybarConfig::Multiple(v) => v.push(BarConfig::default()),
            WaybarConfig::Single(b) => {
                let mut v = vec![*b.clone()];
                v.push(BarConfig::default());
                *self = WaybarConfig::Multiple(v);
            }
        }
    }
    pub fn remove_bar(&mut self, index: usize) {
        match self {
            WaybarConfig::Multiple(v) => {
                if v.len() > 1 && index < v.len() {
                    v.remove(index);
                }
                if v.len() == 1 {
                    let single = v.pop().unwrap();
                    *self = WaybarConfig::Single(Box::new(single));
                }
            }
            WaybarConfig::Single(_) => {}
        }
    }
    pub fn module_count(&self) -> usize {
        self.bars().iter().map(|b| b.module_count()).sum()
    }
}

#[derive(Debug, Clone)]
pub struct ModuleDef {
    pub name: String,
    pub props: BTreeMap<String, Value>,
}

impl ModuleDef {
    pub fn from_value(name: &str, v: &Value) -> Self {
        let props = match v {
            Value::Object(m) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => BTreeMap::new(),
        };
        Self { name: name.to_string(), props }
    }
}