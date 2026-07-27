use async_trait::async_trait;
use serde_json::Value;
use crate::core::messages::SystemError;

/// Trait for LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat request and expect JSON response
    async fn chat_json(&self, system: &str, user: &str) -> Result<Value, SystemError>;
}
