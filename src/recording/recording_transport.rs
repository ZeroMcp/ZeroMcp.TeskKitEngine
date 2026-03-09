use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::protocol::jsonrpc::JsonRpcMessage;
use crate::transport::{McpTransport, TransportError};

use super::recorder::RecordedSession;

/// A transport middleware that records all sent/received messages while
/// forwarding them to an inner transport. After the session is complete,
/// call `to_session()` to extract the recorded session.
pub struct RecordingTransport {
    inner: Box<dyn McpTransport>,
    session: Arc<Mutex<RecordedSession>>,
}

impl RecordingTransport {
    pub fn wrap(inner: Box<dyn McpTransport>, server_url: &str) -> Self {
        Self {
            inner,
            session: Arc::new(Mutex::new(RecordedSession::new(server_url))),
        }
    }

    pub fn to_session(&self) -> RecordedSession {
        self.session.lock().unwrap().clone()
    }
}

#[async_trait]
impl McpTransport for RecordingTransport {
    async fn send(&mut self, message: &JsonRpcMessage) -> Result<(), TransportError> {
        {
            let mut session = self.session.lock().unwrap();
            session.record_sent(message);
        }
        self.inner.send(message).await
    }

    async fn receive(&mut self) -> Result<JsonRpcMessage, TransportError> {
        let message = self.inner.receive().await?;
        {
            let mut session = self.session.lock().unwrap();
            session.record_received(&message);
        }
        Ok(message)
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.inner.close().await
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::jsonrpc::JsonRpcRequest;
    use crate::transport::mock::{MockTransport, init_response};

    #[tokio::test]
    async fn records_sent_and_received_messages() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));

        let mut recording = RecordingTransport::wrap(Box::new(mock), "http://test:8000/mcp");

        let msg = JsonRpcMessage::Request(JsonRpcRequest::new(1i64, "initialize", None));
        recording.send(&msg).await.unwrap();

        let _resp = recording.receive().await.unwrap();

        let session = recording.to_session();
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.server, "http://test:8000/mcp");
    }

    #[tokio::test]
    async fn recording_does_not_alter_messages() {
        let mut mock = MockTransport::new();
        mock.push_response(init_response(1));

        let mut recording = RecordingTransport::wrap(Box::new(mock), "http://test:8000");

        let msg = JsonRpcMessage::Request(JsonRpcRequest::new(1i64, "initialize", None));
        recording.send(&msg).await.unwrap();

        let resp = recording.receive().await.unwrap();
        assert!(matches!(resp, JsonRpcMessage::Response(_)));
    }

    #[tokio::test]
    async fn close_propagates_to_inner() {
        let mock = MockTransport::new();
        let mut recording = RecordingTransport::wrap(Box::new(mock), "http://test:8000");

        recording.close().await.unwrap();

        let err = recording.receive().await;
        assert!(err.is_err());
    }
}
