use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Crosses the Tauri IPC boundary as `{ code, message, context }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[error("{message}")]
#[serde(rename_all = "camelCase")]
pub struct PcfError {
    pub code: String,
    pub message: String,
    pub context: Option<String>,
}

impl PcfError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            context: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}
