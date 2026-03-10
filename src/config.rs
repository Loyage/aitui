use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(rename = "provider")]
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
    pub proxy: Option<String>,
    #[serde(default)]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    pub system_prompt: Option<String>,
}

impl ProviderConfig {
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
            proxy: None,
            max_tokens: 0,
            temperature: 1.0,
            system_prompt: None,
        }
    }
}

pub const PRESET_PROVIDERS: &[(&str, &str, &str)] = &[
    ("DeepSeek", "https://api.deepseek.com", "deepseek-chat"),
    ("OpenAI", "https://api.openai.com", "gpt-4o"),
    ("Mimo", "https://api.mimo.org/v1", "mimo-chat"),
    ("Groq", "https://api.groq.com/openai", "llama3-8b-8192"),
    ("Custom", "", ""),
];

fn default_model() -> String {
    "deepseek-chat".to_string()
}

fn default_temperature() -> f32 {
    1.0
}

impl Config {
    pub fn config_path() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config")
            });
        base.join("aitui").join("config.toml")
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Config { providers: vec![] });
        }
        let content = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
