use once_cell::sync::OnceCell;
use std::fs;
use crate::models::config_models::Config;
pub static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

pub const CONFIG_FILE: &str = "config.toml";

impl Config {
    pub fn new() -> Self {
        let path = CONFIG_FILE;
        let content = fs::read_to_string(path).expect("Failed to read config file");
        let config: Config = toml::from_str(&content).expect("Failed to parse config file");
        config
    }
}
