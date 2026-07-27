//! Debug actor - handles compiler errors with RAD

use crate::models::{DebugInput, DebugOutput, FileDiff};
use crate::tools::CompilerTools;
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::info;

/// Debug actor for fixing compiler errors
pub struct DebugActor {
    project_root: PathBuf,
    compiler_tools: CompilerTools,
    message_rx: mpsc::Receiver<crate::models::ActorMessage>,
    response_tx: mpsc::Sender<crate::models::ActorMessage>,
}

impl DebugActor {
    pub fn new(
        project_root: &std::path::Path,
        message_rx: mpsc::Receiver<crate::models::ActorMessage>,
        response_tx: mpsc::Sender<crate::models::ActorMessage>,
    ) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            compiler_tools: CompilerTools::new(project_root),
            message_rx,
            response_tx,
        }
    }

    /// Spawn the debug actor
    pub fn spawn(
        project_root: &std::path::Path,
        response_tx: mpsc::Sender<crate::models::ActorMessage>,
    ) -> mpsc::Sender<crate::models::ActorMessage> {
        let (tx, rx) = mpsc::channel(100);
        let actor = Self::new(project_root, rx, response_tx);

        tokio::spawn(async move {
            actor.run().await;
        });

        tx
    }

    /// Main event loop
    pub async fn run(mut self) {
        info!("Debug actor started");

        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                crate::models::ActorMessage::DebugRequest(input) => {
                    if let Err(e) = self.handle_debug(input).await {
                        tracing::warn!("Debug error: {}", e);
                    }
                }
                _ => {
                    tracing::warn!("Unexpected message in Debug actor");
                }
            }
        }
    }

    /// Handle debug request
    async fn handle_debug(&self, input: DebugInput) -> Result<()> {
        info!("Debugging {} diagnostics", input.diagnostic_graph.diagnostics.len());

        let worktree_path = self
            .project_root
            .join(".git/worktrees")
            .join(&input.worktree_id);

        let mut fixes = Vec::new();
        let mut escalate = false;
        let mut escalation_reason = None;

        // Get next error to fix from the graph
        if let Some(diagnostic) = input.diagnostic_graph.next_error_to_fix() {
            // Try shadow compilation to determine if error is localized
            let is_localized = self
                .compiler_tools
                .shadow_compile(
                    &worktree_path,
                    &diagnostic.file_path,
                    diagnostic.line,
                    diagnostic.line + 5, // Approximate end line
                )
                .await
                .unwrap_or(false);

            if !is_localized {
                // Error is non-local, escalate to Plan
                escalate = true;
                escalation_reason = Some(format!(
                    "Error in {} is non-local and affects other parts of the codebase",
                    diagnostic.file_path
                ));
            } else {
                // Fetch rustc documentation for the error code (RAD)
                let docs = if let Some(ref code) = diagnostic.code {
                    self.compiler_tools.fetch_error_docs(code).await?
                } else {
                    String::new()
                };

                // In production, would call LLM with:
                // - The diagnostic
                // - The affected code block
                // - The rustc documentation
                // - Suggested fix from compiler
                
                // For now, create a placeholder fix
                let fix = FileDiff {
                    file_path: diagnostic.file_path.clone(),
                    diff: format!(
                        "// TODO: Fix error {}: {}\n// Documentation: {}",
                        diagnostic.code.as_deref().unwrap_or("unknown"),
                        diagnostic.message,
                        docs.chars().take(200).collect::<String>()
                    ),
                    new_content: None,
                };
                fixes.push(fix);
            }
        }

        let output = DebugOutput {
            fixes,
            escalate,
            escalation_reason,
        };

        self.response_tx
            .send(crate::models::ActorMessage::DebugResponse(output))
            .await?;

        Ok(())
    }
}
