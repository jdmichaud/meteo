use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RangeConfig {
    pub min: f32,
    pub max: f32,
}

fn default_true() -> bool { true }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub language: String,
    #[serde(default)]
    pub postcode: String,
    #[serde(default = "default_true")]
    pub reverse_order: bool,
    /// Skip TLS certificate validation for all HTTPS requests.
    /// Useful behind intercepting proxies; insecure — leave off unless needed.
    #[serde(default)]
    pub ignore_ssl_cert: bool,
    pub temperature: RangeConfig,
    pub wind: RangeConfig,
    pub water: RangeConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            language: "fr".to_string(),
            postcode: String::new(),
            reverse_order: true,
            ignore_ssl_cert: false,
            temperature: RangeConfig { min: -30.0, max: 45.0 },
            wind: RangeConfig { min: 0.0, max: 150.0 },
            water: RangeConfig { min: 0.0, max: 150.0 },
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("meteo").join("config.toml"))
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(Config::default()),
    };

    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let default = Config::default();
        let content = toml::to_string_pretty(&default)?;
        std::fs::write(&path, content)?;
        return Ok(default);
    }

    let content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let path = match config_path() {
        Some(p) => p,
        None => return Ok(()),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml::to_string_pretty(config)?)?;
    Ok(())
}
