use async_trait::async_trait;
use serde_json::{Value, json};
use reqwest::Client;
use tracing::{info, debug, warn};

use crate::core::messages::SystemError;
use crate::cognition::provider::LlmProvider;

/// Anthropic Claude API provider
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    cost_per_1k: f64,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
            cost_per_1k: 0.003, // Default: sonnet pricing
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        // Adjust cost based on model
        self.cost_per_1k = match model.as_str() {
            m if m.contains("haiku") => 0.00025,
            m if m.contains("opus") => 0.015,
            _ => 0.003, // sonnet
        };
        self
    }

    /// Extract JSON from LLM response, handling markdown wrappers
    fn extract_json_from_response(response: &str) -> Option<String> {
        let start = response.find('{')?;
        let end = response.rfind('}')?;
        
        if end <= start {
            return None;
        }
        
        let json_str = &response[start..=end];
        
        // Quick validation: ensure balanced braces at top level
        let mut depth = 0i32;
        for c in json_str.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth < 0 {
                return None;
            }
        }
        
        if depth != 0 {
            return None;
        }
        
        Some(json_str.to_string())
    }
    
    /// Truncate context to fit within token budget
    fn truncate_to_token_budget(&self, text: &str, max_tokens: usize) -> String {
        // Rough estimation: 1 token ≈ 4 chars
        let max_chars = max_tokens * 4;
        if text.len() <= max_chars {
            return text.to_string();
        }
        
        // Truncate and add ellipsis
        format!("{}...[truncated]", &text[..max_chars.saturating_sub(20)])
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat_json(&self, system: &str, user: &str) -> Result<Value, SystemError> {
        info!("Sending request to Anthropic API (model: {})", self.model);

        let url = "https://api.anthropic.com/v1/messages";
        
        // Truncate inputs to fit token budget (max 128K tokens for context)
        let truncated_system = self.truncate_to_token_budget(system, 32000);
        let truncated_user = self.truncate_to_token_budget(user, 96000);
        
        let request_body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": truncated_system,
            "messages": [
                {
                    "role": "user",
                    "content": truncated_user
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
        let content_text = if let Some(content) = response_value.get("content").and_then(|c| c.as_array()) {
            if let Some(first) = content.first() {
                first.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        } else {
            None
        };

        match content_text {
            Some(text) => {
                // Try to extract JSON if present, otherwise wrap in fallback structure
                if let Some(json_str) = Self::extract_json_from_response(&text) {
                    serde_json::from_str(&json_str)
                        .map_err(|e| SystemError::Cognition(format!("Invalid JSON from LLM: {}", e)))
                } else {
                    // Fallback: wrap raw text in a JSON object
                    Ok(json!({"raw_text": text}))
                }
            }
            None => {
                warn!("No content found in Anthropic response");
                Ok(response_value)
            }
        }
    }
    
    fn cost_per_1k_tokens(&self) -> f64 {
        self.cost_per_1k
    }
    
    fn model_name(&self) -> &str {
        &self.model
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
