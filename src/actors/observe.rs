//! Observe actor - AST-Indexed Semantic Grounding

use crate::models::{ContextPayload, SymbolInfo};
use crate::tools::LspTools;
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::info;

/// Observe actor for semantic code analysis
pub struct ObserveActor {
    project_root: PathBuf,
    lsp_tools: LspTools,
    message_rx: mpsc::Receiver<crate::models::ActorMessage>,
    response_tx: mpsc::Sender<crate::models::ActorMessage>,
}

impl ObserveActor {
    pub fn new(
        project_root: &std::path::Path,
        message_rx: mpsc::Receiver<crate::models::ActorMessage>,
        response_tx: mpsc::Sender<crate::models::ActorMessage>,
    ) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            lsp_tools: LspTools::new(project_root),
            message_rx,
            response_tx,
        }
    }

    /// Spawn the observe actor
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
        info!("Observe actor started");

        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                crate::models::ActorMessage::ObserveRequest { task } => {
                    if let Err(e) = self.handle_observe(&task).await {
                        tracing::warn!("Observe error: {}", e);
                    }
                }
                _ => {
                    tracing::warn!("Unexpected message in Observe actor");
                }
            }
        }
    }

    /// Handle observe request
    async fn handle_observe(&self, task: &str) -> Result<()> {
        info!("Observing for task: {}", task);

        let mut context = ContextPayload::new(task);

        // Extract potential symbol names from task
        // In production, would use NLP or keyword extraction
        let symbols = extract_symbols_from_task(task);

        for symbol_name in symbols {
            // Query LSP for symbol info
            if let Some(symbol_info) = self.lsp_tools.query_symbol(&symbol_name).await? {
                context = context.with_symbol(symbol_info);
            }

            // Mine conventions from relevant directories
            let conventions = self.lsp_tools.mine_conventions("src").await?;
            for convention in conventions {
                context = context.with_convention(convention);
            }
        }

        // Send response back to orchestrator
        self.response_tx
            .send(crate::models::ActorMessage::ObserveResponse(context))
            .await?;

        Ok(())
    }
}

/// Extract potential symbol names from a task description
fn extract_symbols_from_task(task: &str) -> Vec<String> {
    // Simple heuristic: extract quoted strings and camelCase words
    let mut symbols = Vec::new();
    
    // Extract quoted identifiers
    for cap in regex::Regex::new(r#""([^"]+)""#).unwrap().captures_iter(task) {
        if let Some(m) = cap.get(1) {
            symbols.push(m.as_str().to_string());
        }
    }
    
    // If no quotes, try to extract key terms
    if symbols.is_empty() {
        let words: Vec<&str> = task.split_whitespace().collect();
        for word in words.iter().skip(1).take(3) {
            symbols.push(word.to_string());
        }
    }
    
    symbols
}

// Need to add regex to dependencies
// For now, simple fallback
#[allow(dead_code)]
fn extract_symbols_simple(task: &str) -> Vec<String> {
    task.split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|s| s.to_string())
        .collect()
}
