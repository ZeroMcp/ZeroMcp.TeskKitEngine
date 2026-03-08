use async_trait::async_trait;

use crate::protocol::jsonrpc::JsonRpcMessage;
use crate::transport::{McpTransport, TransportError};

use super::recorder::{MessageDirection, RecordedSession};

/// A transport that replays recorded sessions without a live server.
pub struct ReplayTransport {
    messages: Vec<(MessageDirection, JsonRpcMessage)>,
    cursor: usize,
}

impl ReplayTransport {
    pub fn from_session(session: RecordedSession) -> Self {
        let messages = session
            .messages
            .into_iter()
            .map(|m| (m.direction, m.message))
            .collect();

        Self {
            messages,
            cursor: 0,
        }
    }
}

#[async_trait]
impl McpTransport for ReplayTransport {
    async fn send(&mut self, _message: &JsonRpcMessage) -> Result<(), TransportError> {
        // In replay mode, we skip past the next "sent" message in the recording
        while self.cursor < self.messages.len() {
            if self.messages[self.cursor].0 == MessageDirection::Sent {
                self.cursor += 1;
                return Ok(());
            }
            self.cursor += 1;
        }
        Err(TransportError::Closed)
    }

    async fn receive(&mut self) -> Result<JsonRpcMessage, TransportError> {
        while self.cursor < self.messages.len() {
            if self.messages[self.cursor].0 == MessageDirection::Received {
                let msg = self.messages[self.cursor].1.clone();
                self.cursor += 1;
                return Ok(msg);
            }
            self.cursor += 1;
        }
        Err(TransportError::Closed)
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.cursor = self.messages.len();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
