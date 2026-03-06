use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::config::ProviderConfig;
use crate::event::Event;
use crate::history::{ChatMessage, Role};

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ApiMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
}

#[derive(Debug, Deserialize)]
struct ChunkDelta {
    content: Option<String>,
}

pub fn send_chat_request(
    config: &ProviderConfig,
    messages: &[ChatMessage],
    event_tx: mpsc::UnboundedSender<Event>,
) {
    let api_messages: Vec<ApiMessage> = {
        let mut msgs = Vec::new();
        if let Some(ref system_prompt) = config.system_prompt {
            msgs.push(ApiMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }
        for msg in messages {
            msgs.push(ApiMessage {
                role: match msg.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    Role::System => "system".to_string(),
                },
                content: msg.content.clone(),
            });
        }
        msgs
    };

    let request_body = ChatRequest {
        model: config.model.clone(),
        messages: api_messages,
        stream: true,
        max_tokens: if config.max_tokens > 0 {
            Some(config.max_tokens)
        } else {
            None
        },
        temperature: config.temperature,
    };

    let url = format!(
        "{}/v1/chat/completions",
        config.base_url.trim_end_matches('/')
    );

    let mut builder = reqwest::Client::builder();
    if let Some(ref proxy) = config.proxy {
        if let Ok(p) = reqwest::Proxy::all(proxy) {
            builder = builder.proxy(p);
        }
    }

    let api_key = config.api_key.clone();

    tokio::spawn(async move {
        let client = match builder.build() {
            Ok(c) => c,
            Err(e) => {
                let _ = event_tx.send(Event::ApiError(e.to_string()));
                return;
            }
        };

        let response = match client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request_body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = event_tx.send(Event::ApiError(e.to_string()));
                return;
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let _ = event_tx.send(Event::ApiError(format!("{}: {}", status, body)));
            return;
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    let _ = event_tx.send(Event::ApiError(e.to_string()));
                    return;
                }
            };

            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process complete SSE lines from the buffer
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim_end_matches('\r').to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if data == "[DONE]" {
                        let _ = event_tx.send(Event::ApiDone);
                        return;
                    }

                    match serde_json::from_str::<ChatChunk>(data) {
                        Ok(chunk) => {
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(ref content) = choice.delta.content {
                                    let _ = event_tx.send(Event::ApiToken(content.clone()));
                                }
                            }
                        }
                        Err(e) => {
                            let _ = event_tx.send(Event::ApiError(format!(
                                "Parse error: {}",
                                e
                            )));
                            return;
                        }
                    }
                }
            }
        }

        // Stream ended without [DONE], still mark as done
        let _ = event_tx.send(Event::ApiDone);
    });
}
