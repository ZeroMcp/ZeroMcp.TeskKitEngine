use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::protocol::jsonrpc::JsonRpcMessage;

/// A single recorded message with direction and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedMessage {
    pub direction: MessageDirection,
    pub timestamp: String,
    pub message: JsonRpcMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    Sent,
    Received,
}

/// A complete recorded session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedSession {
    pub version: String,
    pub server: String,
    pub recorded_at: String,
    pub messages: Vec<RecordedMessage>,
}

impl RecordedSession {
    pub fn new(server: &str) -> Self {
        Self {
            version: "1".to_string(),
            server: server.to_string(),
            recorded_at: Utc::now().to_rfc3339(),
            messages: Vec::new(),
        }
    }

    pub fn record_sent(&mut self, message: &JsonRpcMessage) {
        self.messages.push(RecordedMessage {
            direction: MessageDirection::Sent,
            timestamp: Utc::now().to_rfc3339(),
            message: message.clone(),
        });
    }

    pub fn record_received(&mut self, message: &JsonRpcMessage) {
        self.messages.push(RecordedMessage {
            direction: MessageDirection::Received,
            timestamp: Utc::now().to_rfc3339(),
            message: message.clone(),
        });
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::jsonrpc::{JsonRpcMessage, JsonRpcRequest};

    #[test]
    fn record_and_serialize() {
        let mut session = RecordedSession::new("http://localhost:8000/mcp");

        let msg = JsonRpcMessage::Request(JsonRpcRequest::new(1i64, "initialize", None));
        session.record_sent(&msg);
        session.record_received(&msg);

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].direction, MessageDirection::Sent);
        assert_eq!(session.messages[1].direction, MessageDirection::Received);

        let json = serde_json::to_string_pretty(&session).unwrap();
        let parsed: RecordedSession = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.messages.len(), 2);
    }
}
