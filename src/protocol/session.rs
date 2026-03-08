use thiserror::Error;

use super::mcp::{InitializeResult, ServerCapabilities};

/// MCP session lifecycle states.
///
/// The protocol mandates a strict state progression:
/// Disconnected -> Initializing -> Ready -> Closed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Disconnected,
    Initializing,
    Ready,
    Closed,
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: SessionState,
        to: SessionState,
    },

    #[error("Operation '{operation}' not allowed in state {state:?}")]
    NotReady {
        operation: String,
        state: SessionState,
    },

    #[error("Session already closed")]
    AlreadyClosed,
}

/// Tracks the lifecycle of an MCP session.
#[derive(Debug)]
pub struct Session {
    pub state: SessionState,
    pub server_info: Option<InitializeResult>,
    pub server_capabilities: Option<ServerCapabilities>,
    request_counter: i64,
}

impl Session {
    pub fn new() -> Self {
        Self {
            state: SessionState::Disconnected,
            server_info: None,
            server_capabilities: None,
            request_counter: 0,
        }
    }

    pub fn next_request_id(&mut self) -> i64 {
        self.request_counter += 1;
        self.request_counter
    }

    pub fn transition_to_initializing(&mut self) -> Result<(), SessionError> {
        match self.state {
            SessionState::Disconnected => {
                self.state = SessionState::Initializing;
                Ok(())
            }
            _ => Err(SessionError::InvalidTransition {
                from: self.state.clone(),
                to: SessionState::Initializing,
            }),
        }
    }

    pub fn transition_to_ready(
        &mut self,
        init_result: InitializeResult,
    ) -> Result<(), SessionError> {
        match self.state {
            SessionState::Initializing => {
                self.server_capabilities = Some(init_result.capabilities.clone());
                self.server_info = Some(init_result);
                self.state = SessionState::Ready;
                Ok(())
            }
            _ => Err(SessionError::InvalidTransition {
                from: self.state.clone(),
                to: SessionState::Ready,
            }),
        }
    }

    pub fn transition_to_closed(&mut self) -> Result<(), SessionError> {
        if self.state == SessionState::Closed {
            return Err(SessionError::AlreadyClosed);
        }
        self.state = SessionState::Closed;
        Ok(())
    }

    pub fn ensure_ready(&self, operation: &str) -> Result<(), SessionError> {
        if self.state != SessionState::Ready {
            return Err(SessionError::NotReady {
                operation: operation.to_string(),
                state: self.state.clone(),
            });
        }
        Ok(())
    }

    pub fn has_tools_capability(&self) -> bool {
        self.server_capabilities
            .as_ref()
            .is_some_and(|caps| caps.tools.is_some())
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::mcp::{Implementation, ServerCapabilities, ToolsCapability};

    fn mock_init_result() -> InitializeResult {
        InitializeResult {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: true }),
                resources: None,
                prompts: None,
                logging: None,
            },
            server_info: Implementation {
                name: "test-server".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: None,
        }
    }

    #[test]
    fn valid_lifecycle() {
        let mut session = Session::new();
        assert_eq!(session.state, SessionState::Disconnected);

        session.transition_to_initializing().unwrap();
        assert_eq!(session.state, SessionState::Initializing);

        session.transition_to_ready(mock_init_result()).unwrap();
        assert_eq!(session.state, SessionState::Ready);

        session.ensure_ready("tools/list").unwrap();

        session.transition_to_closed().unwrap();
        assert_eq!(session.state, SessionState::Closed);
    }

    #[test]
    fn reject_invalid_transition() {
        let mut session = Session::new();
        let err = session
            .transition_to_ready(mock_init_result())
            .unwrap_err();
        assert!(matches!(err, SessionError::InvalidTransition { .. }));
    }

    #[test]
    fn reject_operation_when_not_ready() {
        let session = Session::new();
        let err = session.ensure_ready("tools/call").unwrap_err();
        assert!(matches!(err, SessionError::NotReady { .. }));
    }

    #[test]
    fn reject_double_close() {
        let mut session = Session::new();
        session.transition_to_initializing().unwrap();
        session.transition_to_ready(mock_init_result()).unwrap();
        session.transition_to_closed().unwrap();
        let err = session.transition_to_closed().unwrap_err();
        assert!(matches!(err, SessionError::AlreadyClosed));
    }

    #[test]
    fn request_id_increments() {
        let mut session = Session::new();
        assert_eq!(session.next_request_id(), 1);
        assert_eq!(session.next_request_id(), 2);
        assert_eq!(session.next_request_id(), 3);
    }

    #[test]
    fn detects_tools_capability() {
        let mut session = Session::new();
        session.transition_to_initializing().unwrap();
        session.transition_to_ready(mock_init_result()).unwrap();
        assert!(session.has_tools_capability());
    }
}
