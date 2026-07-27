use async_trait::async_trait;
use serde_json::{Value, json};
use reqwest::Client;
use tracing::{info, debug};

use crate::core::messages::SystemError;
use crate::cognition::provider::LlmProvider;

/// Anthropic Claude API provider
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat_json(&self, system: &str, user: &str) -> Result<Value, SystemError> {
        info!("Sending request to Anthropic API");

        let url = "https://api.anthropic.com/v1/messages";
        
        let request_body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": system,
            "messages": [
                {
                    "role": "user",
                    "content": user
                }
            ]
        });

        debug!("Request body: {:?}", request_body);

        let response = self.client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SystemError::Cognition(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(SystemError::Cognition(format!(
                "API returned error {}: {}",
                status, error_text
            )));
        }

        let response_value: Value = response
            .json()
            .await
            .map_err(|e| SystemError::Cognition(format!("Failed to parse response: {}", e)))?;

        debug!("Response: {:?}", response_value);

        // Extract content from Anthropic response
        if let Some(content) = response_value.get("content").and_then(|c| c.as_array()) {
            if let Some(first) = content.first() {
                if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                    return Ok(Value::String(text.to_string()));
                }
            }
        }

        // Fallback: return the whole response as string
        Ok(response_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = AnthropicProvider::new("test-key".to_string());
        assert_eq!(provider.model, "claude-sonnet-4-20250514");
    }
}
