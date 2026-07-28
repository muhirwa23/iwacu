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
        // Hardcoded routing rules per specification
        let preferred_model = match node {
            "plan" => {
                // Use sonnet if budget > $0.02, else haiku
                if self.budget.remaining() > 0.02 {
                    "claude-sonnet"
                } else {
                    "claude-haiku"
                }
            }
            "generate" => {
                // Always use sonnet unless critically low budget
                if self.budget.remaining() > 0.01 {
                    "claude-sonnet"
                } else {
                    "claude-haiku"
                }
            }
            "debug" => "claude-haiku", // Fast, cheap
            "synthesize" => "claude-haiku",
            _ => "claude-sonnet", // Default
        };
        
        // Try preferred model first, fallback to any available
        if let Some(provider) = self.providers.get(preferred_model) {
            return Ok(Arc::clone(provider));
        }
        
        // Fallback: try alternative names
        let fallbacks = match preferred_model {
            "claude-sonnet" => vec!["claude-sonnet-4-20250514", "claude-3-5-sonnet"],
            "claude-haiku" => vec!["claude-3-haiku", "claude-3-5-haiku"],
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
    
    pub fn budget_mut(&mut self) -> &mut BudgetTracker {
        &mut self.budget
    }
}
