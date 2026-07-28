pub mod provider;
pub mod anthropic;
pub mod nvidia;
pub mod prompts;

use std::sync::Arc;
use tokio::sync::mpsc;
use serde_json::Value;
use tracing::{info, error, warn};

use crate::core::messages::{ActorMessage, CognitionRequest, TaskContext, SystemError};
use provider::{LlmProvider, ModelRouter};
pub use nvidia::NvidiaProvider;
use prompts::{plan_prompt, generate_prompt, debug_prompt, synthesize_prompt};

/// Cognition actor that handles LLM interactions with multi-model routing
pub struct CognitionActor {
    router: Arc<ModelRouter>,
}

impl CognitionActor {
    pub fn new(router: Arc<ModelRouter>) -> Self {
        Self { router }
    }

    /// Start the cognition actor event loop
    pub async fn run(
        self,
        mut rx: mpsc::Receiver<ActorMessage<CognitionRequest>>,
    ) -> Result<(), SystemError> {
        info!("Cognition actor started");

        while let Some(msg) = rx.recv().await {
            let result = match msg.payload {
                CognitionRequest::Plan(context) => self.handle_plan(context).await,
                CognitionRequest::Generate(context) => self.handle_generate(context).await,
                CognitionRequest::Debug(context) => self.handle_debug(context).await,
                CognitionRequest::Synthesize(context) => self.handle_synthesize(context).await,
            };

            if let Err(e) = msg.reply_to.send(result.map(Value::String)) {
                error!("Failed to send reply: {:?}", e);
            }
        }

        info!("Cognition actor stopped");
        Ok(())
    }

    /// Get provider for a specific node type
    fn get_provider(&self, node: &str) -> Result<Arc<dyn LlmProvider>, SystemError> {
        self.router.route(node, self.router.budget().remaining())
    }

    /// Send a chat request for a stage, trying the preferred model first and
    /// transparently falling back to the next available model on failure
    /// (e.g. a 404 model-not-found for a given key/account).
    async fn chat_with_fallback(
        &self,
        node: &str,
        system: &str,
        user: &str,
    ) -> Result<Value, SystemError> {
        let chain = self.router.route_chain(node);
        if chain.is_empty() {
            return Err(SystemError::Cognition("No LLM providers configured".to_string()));
        }

        let mut last_err: Option<SystemError> = None;
        for (idx, provider) in chain.iter().enumerate() {
            if idx > 0 {
                warn!(
                    "Falling back to model '{}' for '{}' stage",
                    provider.model_name(),
                    node
                );
            }
            match provider.chat_json(system, user).await {
                Ok(value) => return Ok(value),
                Err(e) => {
                    warn!(
                        "Model '{}' failed for '{}' stage: {}",
                        provider.model_name(),
                        node,
                        e
                    );
                    last_err = Some(e);
                }
            }
        }

        Err(last_err
            .unwrap_or_else(|| SystemError::Cognition("All LLM providers failed".to_string())))
    }

    /// Estimate cost based on prompt length (rough estimation)
    fn estimate_cost(&self, system: &str, user: &str, provider: &dyn LlmProvider) -> f64 {
        let total_chars = system.len() + user.len();
        let estimated_tokens = total_chars / 4; // Rough: 1 token ≈ 4 chars
        (estimated_tokens as f64 / 1000.0) * provider.cost_per_1k_tokens()
    }

    async fn handle_plan(&self, context: TaskContext) -> Result<String, SystemError> {
        info!("Handling plan request");
        
        let (system, user) = plan_prompt(&context.task, "architectural_analysis");

        // Single planning call. (Self-consistency voting across multiple
        // iterations is possible, but the reasoning model is slow and only the
        // first successful plan is used, so we issue one request.)
        let full_user = format!("{}\nLSP Context: {}", user,
            context.lsp_context.as_deref().unwrap_or("No LSP context available"));

        let response = self.chat_with_fallback("plan", &system, &full_user).await?;
        let plan = response
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| response.get("raw_text").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| response.to_string());

        if plan.trim().is_empty() {
            return Err(SystemError::Cognition("Planning returned an empty plan".to_string()));
        }

        Ok(plan)
    }

    async fn handle_generate(&self, context: TaskContext) -> Result<String, SystemError> {
        info!("Handling generate request");

        // Chain-of-thought cycle: generate with reasoning and self-critique
        let (system, user) = generate_prompt(
            &context.task,
            context.plan.as_ref().and_then(|p| p.first()).map(|s| s.as_str()).unwrap_or("Implement the task"),
            context.lsp_context.as_deref().unwrap_or("")
        );
        
        let full_user = if let Some(errors) = &context.compiler_errors {
            format!("{}\nCompiler Errors: {}", user, errors)
        } else {
            user
        };

        // First pass: generate with reasoning and self-critique
        let response = self.chat_with_fallback("generate", &system, &full_user).await?;
        
        // Parse response and check self-critique
        let diff = if let Some(diff_val) = response.get("diff").and_then(|d| d.as_str()) {
            // Check if self_critique raises high-severity issues
            let self_critique = response.get("self_critique")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            
            // If self-critique identifies critical issues, re-prompt to fix them
            if self_critique.to_lowercase().contains("critical") || 
               self_critique.to_lowercase().contains("severe") ||
               self_critique.to_lowercase().contains("major issue") {
                info!("Self-critique identified critical issues, re-prompting...");
                
                let fix_system = r#"Fix the identified critical issues in your previous diff.
Output ONLY the corrected diff."#;
                
                let fix_user = format!(
                    "Previous diff:\n{}\n\nSelf-critique: {}\n\nProvide corrected diff.",
                    diff_val, self_critique
                );
                
                let fix_response = self.chat_with_fallback("generate", fix_system, &fix_user).await?;
                fix_response.get("diff")
                    .and_then(|d| d.as_str())
                    .unwrap_or(diff_val)
                    .to_string()
            } else {
                diff_val.to_string()
            }
        } else {
            // Fallback: try to extract diff from raw response
            response.as_str().unwrap_or("").to_string()
        };

        Ok(diff)
    }

    async fn handle_debug(&self, context: TaskContext) -> Result<String, SystemError> {
        info!("Handling debug request");

        // Format error graph (should be topologically sorted by compiler.rs)
        let error_graph = context.compiler_errors
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "No error details available".to_string());
        
        let (system, user) = debug_prompt(
            &error_graph,
            &context.current_diff.as_deref().unwrap_or("")
        );

        let response = self.chat_with_fallback("debug", &system, &user).await?;
        
        // Extract root_cause and fix_diff
        let diff = if let Some(fix_diff) = response.get("fix_diff").and_then(|d| d.as_str()) {
            info!("Root cause: {}", 
                response.get("root_cause").and_then(|r| r.as_str()).unwrap_or("Unknown"));
            fix_diff.to_string()
        } else {
            // Fallback
            response.as_str().unwrap_or("").to_string()
        };

        Ok(diff)
    }

    async fn handle_synthesize(&self, context: TaskContext) -> Result<String, SystemError> {
        info!("Handling synthesize request");

        let (system, user) = synthesize_prompt(&context.task, &context.current_diff.unwrap_or_default());

        let response = self.chat_with_fallback("synthesize", &system, &user).await?;
        
        // Extract summary
        let summary = if let Some(title) = response.get("title").and_then(|t| t.as_str()) {
            let files = response.get("files_changed")
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "unknown files".to_string());
            
            format!("{} - {}", title, files)
        } else {
            response.as_str().unwrap_or("").to_string()
        };

        info!("Synthesis complete: {}", summary);
        Ok(summary)
    }
}
