use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub author: String,
    #[serde(default)]
    pub github_account: String,
}

fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("soil")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load() -> anyhow::Result<Config> {
    let path = config_path();
    if !path.exists() {
        return wizard();
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;
    let mut config: Config = toml::from_str(&content).context("Failed to parse config")?;
    if config.github_account.is_empty() {
        config.github_account = std::env::var("GITHUB_USER")
            .or_else(|_| std::env::var("GITHUB_ACTOR"))
            .unwrap_or_default();
    }
    Ok(config)
}

pub fn is_initialized() -> bool {
    config_path().exists()
}

fn env_github_account() -> String {
    std::env::var("GITHUB_USER")
        .or_else(|_| std::env::var("GITHUB_ACTOR"))
        .unwrap_or_default()
}

pub fn wizard() -> anyhow::Result<Config> {
    println!("Welcome to soil! Let's set things up.");

    let author: String = dialoguer::Input::new()
        .with_prompt("Your pen name")
        .interact_text()?;

    let default_account = env_github_account();
    let github_account: String = dialoguer::Input::new()
        .with_prompt("GitHub account")
        .default(default_account)
        .interact_text()?;

    let config = Config {
        author,
        github_account,
    };

    save(&config)?;
    println!("Config saved.");

    Ok(config)
}

pub fn save(config: &Config) -> anyhow::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create config dir at {}", dir.display()))?;
    let path = config_path();
    let content =
        toml::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write config at {}", path.display()))?;
    Ok(())
}
