use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use crate::core::messages::SystemError;

/// Trait for LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat request and expect JSON response
    async fn chat_json(&self, system: &str, user: &str) -> Result<Value, SystemError>;
    
    /// Cost per 1K tokens in USD
    fn cost_per_1k_tokens(&self) -> f64;
    
    /// Model name for logging
    fn model_name(&self) -> &str;
}

/// Tracks budget consumption
pub struct BudgetTracker {
    remaining: f64,
    spent: f64,
}

impl BudgetTracker {
    pub fn new(initial_budget: f64) -> Self {
        Self {
            remaining: initial_budget,
            spent: 0.0,
        }
    }
    
    pub fn remaining(&self) -> f64 {
        self.remaining
    }
    
    pub fn spent(&self) -> f64 {
        self.spent
    }
    
    pub fn record_cost(&mut self, cost: f64) -> Result<(), SystemError> {
        if cost > self.remaining {
            return Err(SystemError::Cognition(format!(
                "Budget exceeded: requested ${:.4}, remaining ${:.4}",
                cost, self.remaining
            )));
        }
        self.spent += cost;
        self.remaining -= cost;
        Ok(())
    }
}

/// Routes requests to appropriate models based on task type and budget
pub struct ModelRouter {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    budget: BudgetTracker,
}

impl ModelRouter {
    pub fn new(initial_budget: f64) -> Self {
        Self {
            providers: HashMap::new(),
            budget: BudgetTracker::new(initial_budget),
        }
    }
    
    pub fn add_provider(&mut self, name: String, provider: Arc<dyn LlmProvider>) {
        self.providers.insert(name, provider);
    }
    
    pub fn get_provider(&self, name: &str) -> Option<&Arc<dyn LlmProvider>> {
        self.providers.get(name)
    }
    
    /// Route request to appropriate model based on node type and budget
    pub fn route(&self, node: &str, _remaining_budget: f64) -> Result<Arc<dyn LlmProvider>, SystemError> {
        // Routing rules mapping each reasoning stage to an NVIDIA-hosted model.
        let preferred_model = match node {
            // Planning: strong reasoning model
            "plan" => "deepseek",
            // Code generation: capable general model
            "generate" => "glm",
            // Debugging: fast, long-context model
            "debug" => "kimi",
            // Synthesis / summary: lightweight model
            "synthesize" => "inkling",
            _ => "glm", // Default
        };

        // Try preferred model first, fallback to any available
        if let Some(provider) = self.providers.get(preferred_model) {
            return Ok(Arc::clone(provider));
        }

        // Fallback: try fully-qualified model names
        let fallbacks = match preferred_model {
            "deepseek" => vec!["deepseek-ai/deepseek-v4-pro"],
            "glm" => vec!["z-ai/glm-5.2"],
            "kimi" => vec!["moonshotai/kimi-k2.6"],
            "inkling" => vec!["thinkingmachines/inkling"],
            _ => vec![],
        };

        for fallback in fallbacks {
            if let Some(provider) = self.providers.get(fallback) {
                return Ok(Arc::clone(provider));
            }
        }

        // Last resort: return any available provider
        self.providers.values().next().map(Arc::clone).ok_or_else(|| {
            SystemError::Cognition("No LLM providers configured".to_string())
        })
    }
    
    pub fn budget(&self) -> &BudgetTracker {
        &self.budget
    }

    /// Build an ordered chain of providers for a stage: the preferred model
    /// first, followed by every other configured model as fallbacks. This lets
    /// the models "work hand in hand" — if one model's key/account fails (e.g.
    /// a 404 model-not-found), the next available model transparently takes over.
    pub fn route_chain(&self, node: &str) -> Vec<Arc<dyn LlmProvider>> {
        let primary = match node {
            "plan" => "deepseek",
            "generate" => "glm",
            "debug" => "kimi",
            "synthesize" => "inkling",
            _ => "glm",
        };

        // Global fallback preference order.
        let order = ["glm", "deepseek", "kimi", "inkling"];

        let mut chain: Vec<Arc<dyn LlmProvider>> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        if let Some(p) = self.providers.get(primary) {
            chain.push(Arc::clone(p));
            seen.insert(primary.to_string());
        }
        for name in order {
            if seen.contains(name) {
                continue;
            }
            if let Some(p) = self.providers.get(name) {
                chain.push(Arc::clone(p));
                seen.insert(name.to_string());
            }
        }
        // Include any other providers registered under non-standard keys.
        for (k, p) in &self.providers {
            if !seen.contains(k) {
                chain.push(Arc::clone(p));
                seen.insert(k.clone());
            }
        }

        chain
    }
    
    pub fn budget_mut(&mut self) -> &mut BudgetTracker {
        &mut self.budget
    }
}
