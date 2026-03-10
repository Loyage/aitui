use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: now.timestamp_millis().to_string(),
            title: "New Chat".to_string(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

fn data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("share")
        });
    base.join("aitui")
}

pub fn save_conversation(conv: &Conversation) -> anyhow::Result<()> {
    let dir = data_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", conv.id));
    let json = serde_json::to_string_pretty(conv)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_latest_conversation() -> Option<Conversation> {
    load_all_conversations().ok()?.first().cloned()
}

pub fn load_all_conversations() -> anyhow::Result<Vec<Conversation>> {
    let dir = data_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut conversations = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.path().extension().map_or(false, |ext| ext == "json") {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Ok(conv) = serde_json::from_str::<Conversation>(&content) {
                    conversations.push(conv);
                }
            }
        }
    }

    conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(conversations)
}
