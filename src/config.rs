use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(rename = "device", default)]
    pub devices: Vec<Device>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Defaults {
    #[serde(default = "default_kmi")]
    pub kmi_generation: u32,
}

fn default_kmi() -> u32 {
    8
}

#[derive(Debug, Deserialize)]
pub struct Device {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    pub repo: String,
    #[serde(default = "main_branch")]
    pub branch: String,
    /// Path (relative to the kernel repo root) of the build entrypoint,
    /// invoked as: bash <build_script> <mode>
    pub build_script: String,
    #[serde(default)]
    pub modes: Vec<String>,
    #[serde(default)]
    pub anykernel_repo: Option<String>,
    #[serde(default = "main_branch")]
    pub anykernel_branch: String,
    pub toolchain: Toolchain,
}

fn yes() -> bool {
    true
}
fn main_branch() -> String {
    "main".to_string()
}

#[derive(Debug, Deserialize)]
pub struct Toolchain {
    /// Absolute path to a prebuilts dir already on disk (local runs).
    #[serde(default)]
    pub local_path: Option<String>,
    /// Tarball URL fetched + extracted for cloud runs.
    #[serde(default)]
    pub url: Option<String>,
    /// Subdir within the extracted tarball that is the prebuilts root.
    #[serde(default)]
    pub subdir: Option<String>,
}

pub fn load(path: &str) -> Result<Config> {
    let s = fs::read_to_string(path).with_context(|| format!("reading config {path}"))?;
    let c: Config = toml::from_str(&s).with_context(|| format!("parsing config {path}"))?;
    Ok(c)
}

impl Config {
    pub fn device(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }
}

impl Device {
    pub fn supports(&self, mode: &str) -> bool {
        self.modes.is_empty() || self.modes.iter().any(|m| m == mode)
    }
}
