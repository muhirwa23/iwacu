pub mod provider;
pub mod anthropic;

use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use serde_json::Value;
use tracing::{info, error};

use crate::core::messages::{ActorMessage, CognitionRequest, TaskContext, SystemError};
use provider::LlmProvider;
use anthropic::AnthropicProvider;

/// Cognition actor that handles LLM interactions
pub struct CognitionActor {
    provider: Arc<dyn LlmProvider>,
}

impl CognitionActor {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    /// Start the cognition actor event loop
    pub async fn run(
        mut self,
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
                error!("Failed to send reply: {}", e);
            }
        }

        info!("Cognition actor stopped");
        Ok(())
    }

    async fn handle_plan(&self, context: TaskContext) -> Result<String, SystemError> {
        info!("Handling plan request");
        
        let system_prompt = r#"You are an expert software architect. Given a task and codebase context, create a detailed step-by-step plan.
Output ONLY a JSON array of strings, where each string is a concrete action item."#;

        let user_prompt = format!(
            "Task: {}\n\nContext:\n{}",
            context.task,
            context.lsp_context.unwrap_or_default()
        );

        let response = self.provider.chat_json(&system_prompt, &user_prompt).await?;
        
        // Extract plan from response
        let plan = response
            .as_str()
            .unwrap_or("[]")
            .to_string();

        Ok(plan)
    }

    async fn handle_generate(&self, context: TaskContext) -> Result<String, SystemError> {
        info!("Handling generate request");

        let system_prompt = r#"You are an expert programmer. Generate a unified diff patch to implement the requested changes.
Output ONLY the diff content, nothing else. Use proper unified diff format with --- and +++ headers."#;

        let mut user_prompt = format!("Task: {}\n", context.task);
        
        if let Some(plan) = &context.plan {
            user_prompt.push_str(&format!("Plan: {:?}\n", plan));
        }
        
        if let Some(lsp_ctx) = &context.lsp_context {
            user_prompt.push_str(&format!("Code Context: {}\n", lsp_ctx));
        }

        if let Some(errors) = &context.compiler_errors {
            user_prompt.push_str(&format!("Compiler Errors: {}\n", errors));
        }

        let response = self.provider.chat_json(&system_prompt, &user_prompt).await?;
        
        let diff = response
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(diff)
    }

    async fn handle_debug(&self, context: TaskContext) -> Result<String, SystemError> {
        info!("Handling debug request");

        let system_prompt = r#"You are an expert debugger. Analyze the compiler errors and generate a fix as a unified diff.
Output ONLY the diff content to fix the errors."#;

        let user_prompt = format!(
            "Task: {}\nCompiler Errors: {:?}\nCurrent Code Context: {}",
            context.task,
            context.compiler_errors.unwrap_or_default(),
            context.lsp_context.unwrap_or_default()
        );

        let response = self.provider.chat_json(&system_prompt, &user_prompt).await?;
        
        let diff = response
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(diff)
    }

    async fn handle_synthesize(&self, context: TaskContext) -> Result<String, SystemError> {
        info!("Handling synthesize request");

        let system_prompt = r#"Summarize the changes made in a clear PR description format.
Include: what was changed, why, and any notable implementation details."#;

        let user_prompt = format!(
            "Task: {}\nChanges applied successfully.\nDiff: {}",
            context.task,
            context.current_diff.unwrap_or_default()
        );

        let response = self.provider.chat_json(&system_prompt, &user_prompt).await?;
        
        let summary = response
            .as_str()
            .unwrap_or("")
            .to_string();

        info!("Synthesis complete: {}", summary);
        Ok(summary)
    }
}
