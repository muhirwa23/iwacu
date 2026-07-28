pub mod lsp;
pub mod git;
pub mod compiler;
pub mod mutator;

use std::sync::Arc;
use tokio::sync::mpsc;
use serde_json::{Value, json};
use tracing::{info, error, warn};

use crate::core::messages::{
    ActorMessage, EnvironmentRequest, SystemError, Location,
};
use git::GitTools;
use lsp::LspClient;
use compiler::Compiler;
use mutator::Mutator;

/// Environment actor that handles file system, Git, LSP, and compiler interactions
pub struct EnvironmentActor {
    git: Arc<GitTools>,
    lsp: Arc<LspClient>,
    compiler: Arc<Compiler>,
    mutator: Arc<Mutator>,
    worktree_path: Option<String>,
}

impl EnvironmentActor {
    pub fn new(repo_path: String) -> Result<Self, SystemError> {
        let git = Arc::new(GitTools::new(&repo_path)?);
        let lsp = Arc::new(LspClient::new()?);
        let compiler = Arc::new(Compiler::new());
        let mutator = Arc::new(Mutator::new());

        Ok(Self {
            git,
            lsp,
            compiler,
            mutator,
            worktree_path: None,
        })
    }

    /// Start the environment actor event loop
    pub async fn run(
        mut self,
        mut rx: mpsc::Receiver<ActorMessage<EnvironmentRequest>>,
    ) -> Result<(), SystemError> {
        info!("Environment actor started");

        while let Some(msg) = rx.recv().await {
            let result = match msg.payload {
                EnvironmentRequest::Observe(task) => self.handle_observe(task).await,
                EnvironmentRequest::ApplyDiff(diff) => self.handle_apply_diff(diff).await,
                EnvironmentRequest::RunTests => self.handle_run_tests().await,
                EnvironmentRequest::CleanupWorktree => self.handle_cleanup_worktree().await,
                EnvironmentRequest::GetDefinition(symbol) => {
                    self.handle_get_definition(symbol).await
                }
                EnvironmentRequest::GetReferences(location) => {
                    self.handle_get_references(location).await
                }
            };

            if let Err(e) = msg.reply_to.send(result) {
                error!("Failed to send reply: {:?}", e);
            }
        }

        // Cleanup worktree on shutdown
        if let Some(path) = &self.worktree_path {
            let _ = self.git.cleanup_worktree(path);
        }

        info!("Environment actor stopped");
        Ok(())
    }

    async fn handle_observe(&mut self, task: String) -> Result<Value, SystemError> {
        info!("Handling observe request for: {}", task);

        // Extract potential symbol names from task
        // In a real implementation, we'd use NLP or keyword extraction
        let symbols = extract_symbols_from_task(&task);

        let mut context = String::new();
        let mut locations = Vec::new();

        for symbol in &symbols {
            match self.lsp.get_definition(symbol).await {
                Ok(loc) => {
                    context.push_str(&format!("Symbol '{}' found at {:?}\n", symbol, loc));
                    locations.push(loc.clone());

                    // Get references
                    if let Ok(refs) = self.lsp.get_references(&loc).await {
                        context.push_str(&format!(
                            "  References: {} locations\n",
                            refs.len()
                        ));
                    }
                }
                Err(e) => {
                    warn!("Could not find definition for '{}': {}", symbol, e);
                }
            }
        }

        // Create worktree for safe modifications
        let worktree_path = self.git.create_worktree()?;
        info!("Created worktree at: {}", worktree_path.display());

        self.worktree_path = Some(worktree_path.to_string_lossy().to_string());

        Ok(json!({
            "context": context,
            "worktree": worktree_path.to_string_lossy().to_string(),
            "symbols_found": symbols.len()
        }))
    }

    async fn handle_apply_diff(&self, diff: String) -> Result<Value, SystemError> {
        info!("Handling apply diff request");

        let worktree_path = self
            .worktree_path
            .as_ref()
            .ok_or_else(|| SystemError::Environment("No worktree created".to_string()))?;

        self.git.apply_patch(worktree_path, &diff)?;

        Ok(json!({"success": true, "message": "Patch applied successfully"}))
    }

    async fn handle_run_tests(&self) -> Result<Value, SystemError> {
        info!("Handling run tests request");

        let worktree_path = self
            .worktree_path
            .as_ref()
            .ok_or_else(|| SystemError::Environment("No worktree created".to_string()))?;

        // Run tests
        let test_result = self.compiler.run_tests(worktree_path)?;

        if test_result.success {
            // Run mutation gate
            let mutation_passed = self.mutator.run_mutation_gate(worktree_path)?;

            Ok(json!({
                "success": true,
                "mutation_passed": mutation_passed,
                "output": test_result.test_output
            }))
        } else {
            let diagnostics: Vec<Value> = test_result
                .diagnostics
                .iter()
                .map(|d| {
                    json!({
                        "file": d.file,
                        "line": d.line,
                        "code": d.code,
                        "message": d.message
                    })
                })
                .collect();

            Ok(json!({
                "success": false,
                "diagnostics": diagnostics,
                "output": test_result.test_output
            }))
        }
    }

    async fn handle_cleanup_worktree(&mut self) -> Result<Value, SystemError> {
        info!("Handling cleanup worktree request");

        if let Some(path) = &self.worktree_path.take() {
            self.git.cleanup_worktree(path)?;
            info!("Cleaned up worktree at: {}", path);
        }

        Ok(json!({"success": true, "message": "Worktree cleaned up"}))
    }

    async fn handle_get_definition(&self, symbol: String) -> Result<Value, SystemError> {
        info!("Getting definition for: {}", symbol);

        let location = self.lsp.get_definition(&symbol).await?;

        Ok(json!(location))
    }

    async fn handle_get_references(&self, location: Location) -> Result<Value, SystemError> {
        info!("Getting references for: {:?}", location);

        let references = self.lsp.get_references(&location).await?;

        Ok(json!(references))
    }
}

/// Extract potential symbol names from a task description
/// Looks for identifier-like words (snake_case or CamelCase)
fn extract_symbols_from_task(task: &str) -> Vec<String> {
    task.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .filter(|w| {
            w.len() > 2 && (w.contains('_') || w.chars().any(|c| c.is_uppercase()))
        })
        .collect()
}

