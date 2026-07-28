use async_trait::async_trait;
use serde_json::{Value, json};
use reqwest::Client;
use tracing::{info, debug, warn};

use crate::core::messages::SystemError;
use crate::cognition::provider::LlmProvider;

const DEFAULT_BASE_URL: &str = "https://integrate.api.nvidia.com/v1";

/// OpenAI-compatible provider targeting NVIDIA's `integrate.api.nvidia.com` endpoint.
///
/// Works with any model exposed via the NVIDIA NIM OpenAI-compatible API, e.g.
/// `z-ai/glm-5.2`, `thinkingmachines/inkling`, `deepseek-ai/deepseek-v4-pro`,
/// `moonshotai/kimi-k2.6`.
pub struct NvidiaProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    cost_per_1k: f64,
    temperature: f64,
    top_p: f64,
    max_tokens: u32,
    disable_thinking: bool,
}

impl NvidiaProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
            model: "z-ai/glm-5.2".to_string(),
            cost_per_1k: 0.001,
            temperature: 1.0,
            top_p: 1.0,
            max_tokens: 16384,
            disable_thinking: false,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        // Rough cost hints per model family (USD / 1K tokens). NVIDIA credits
        // are not strictly USD, these are only used for budget bookkeeping/logging.
        self.cost_per_1k = match model.as_str() {
            m if m.contains("deepseek") => 0.002,
            m if m.contains("kimi") => 0.0015,
            m if m.contains("glm") => 0.001,
            m if m.contains("inkling") => 0.0008,
            _ => 0.001,
        };
        self.model = model;
        self
    }

    #[allow(dead_code)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn with_sampling(mut self, temperature: f64, top_p: f64, max_tokens: u32) -> Self {
        self.temperature = temperature;
        self.top_p = top_p;
        self.max_tokens = max_tokens;
        self
    }

    /// Disable model "thinking"/reasoning traces for much faster responses.
    /// Sent as `chat_template_kwargs: { "thinking": false }` (NVIDIA NIM extension).
    pub fn with_thinking_disabled(mut self, disabled: bool) -> Self {
        self.disable_thinking = disabled;
        self
    }

    /// Extract a JSON object from an LLM response, handling markdown code fences.
    fn extract_json_from_response(response: &str) -> Option<String> {
        let start = response.find('{')?;
        let end = response.rfind('}')?;

        if end <= start {
            return None;
        }

        let json_str = &response[start..=end];

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

    /// Truncate context to fit within a rough token budget (1 token ≈ 4 chars).
    fn truncate_to_token_budget(&self, text: &str, max_tokens: usize) -> String {
        let max_chars = max_tokens * 4;
        if text.len() <= max_chars {
            return text.to_string();
        }
        format!("{}...[truncated]", &text[..max_chars.saturating_sub(20)])
    }
}

#[async_trait]
impl LlmProvider for NvidiaProvider {
    async fn chat_json(&self, system: &str, user: &str) -> Result<Value, SystemError> {
        info!("Sending request to NVIDIA API (model: {})", self.model);

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let truncated_system = self.truncate_to_token_budget(system, 32000);
        let truncated_user = self.truncate_to_token_budget(user, 96000);

        let request_body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "top_p": self.top_p,
            "stream": false,
            "messages": [
                { "role": "system", "content": truncated_system },
                { "role": "user", "content": truncated_user }
            ]
        });

        let request_body = if self.disable_thinking {
            let mut body = request_body;
            if let Some(obj) = body.as_object_mut() {
                obj.insert("chat_template_kwargs".to_string(), json!({ "thinking": false }));
            }
            body
        } else {
            request_body
        };

        debug!("Request body: {:?}", request_body);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
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

        // OpenAI-compatible: choices[0].message.content
        let content_text = response_value
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        match content_text {
            Some(text) => {
                if let Some(json_str) = Self::extract_json_from_response(&text) {
                    serde_json::from_str(&json_str)
                        .map_err(|e| SystemError::Cognition(format!("Invalid JSON from LLM: {}", e)))
                } else {
                    Ok(json!({ "raw_text": text }))
                }
            }
            None => {
                warn!("No content found in NVIDIA response");
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
        let provider = NvidiaProvider::new("test-key".to_string());
        assert_eq!(provider.model, "z-ai/glm-5.2");
        assert_eq!(provider.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn test_with_model_sets_cost() {
        let provider = NvidiaProvider::new("k".to_string())
            .with_model("deepseek-ai/deepseek-v4-pro".to_string());
        assert_eq!(provider.model, "deepseek-ai/deepseek-v4-pro");
        assert!(provider.cost_per_1k > 0.0);
    }
}
