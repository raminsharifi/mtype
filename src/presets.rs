//! Offline named settings slots. Kept deliberately simple and portable.

use crate::config::Config;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "monkeytype", "mtype")
        .context("could not resolve data directory")?;
    Ok(dirs.data_dir().join("presets.json"))
}

fn load_all() -> BTreeMap<String, Config> {
    path()
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub fn save(name: &str, config: &Config) -> Result<()> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut presets = load_all();
    presets.insert(name.to_string(), config.clone());
    std::fs::write(path, serde_json::to_string_pretty(&presets)?)?;
    Ok(())
}

pub fn load(name: &str) -> Option<Config> {
    load_all().remove(name)
}

pub fn names() -> Vec<String> {
    load_all().into_keys().collect()
}
