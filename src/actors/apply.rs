//! Apply actor - applies patches to worktrees

use crate::models::{ApplyInput, ApplyResult};
use crate::tools::GitTools;
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::info;

/// Apply actor for applying code changes
pub struct ApplyActor {
    project_root: PathBuf,
    git_tools: Option<GitTools>,
    message_rx: mpsc::Receiver<crate::models::ActorMessage>,
    response_tx: mpsc::Sender<crate::models::ActorMessage>,
}

impl ApplyActor {
    pub fn new(
        project_root: &std::path::Path,
        message_rx: mpsc::Receiver<crate::models::ActorMessage>,
        response_tx: mpsc::Sender<crate::models::ActorMessage>,
    ) -> Self {
        let git_tools = GitTools::open(project_root).ok();
        
        Self {
            project_root: project_root.to_path_buf(),
            git_tools,
            message_rx,
            response_tx,
        }
    }

    /// Spawn the apply actor
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
        info!("Apply actor started");

        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                crate::models::ActorMessage::ApplyRequest(input) => {
                    if let Err(e) = self.handle_apply(input).await {
                        tracing::warn!("Apply error: {}", e);
                    }
                }
                _ => {
                    tracing::warn!("Unexpected message in Apply actor");
                }
            }
        }
    }

    /// Handle apply request
    async fn handle_apply(&self, input: ApplyInput) -> Result<()> {
        info!("Applying {} diffs to worktree {}", input.diffs.len(), input.worktree_id);

        let mut applied_files = Vec::new();
        let mut error = None;

        if let Some(ref git) = self.git_tools {
            // Get worktree path
            let worktree_path = self.project_root.join(".git/worktrees").join(&input.worktree_id);
            
            // Apply each diff
            for diff in &input.diffs {
                if let Some(ref content) = diff.new_content {
                    if let Err(e) = git.write_file(&worktree_path, &diff.file_path, content) {
                        error = Some(format!("Failed to apply {}: {}", diff.file_path, e));
                        break;
                    }
                    applied_files.push(diff.file_path.clone());
                } else {
                    // Would parse unified diff in production
                    applied_files.push(diff.file_path.clone());
                }
            }
        } else {
            error = Some("Git tools not initialized".to_string());
        }

        let result = ApplyResult {
            success: error.is_none(),
            worktree_id: input.worktree_id,
            applied_files,
            error,
        };

        self.response_tx
            .send(crate::models::ActorMessage::ApplyResponse(result))
            .await?;

        Ok(())
    }
}
