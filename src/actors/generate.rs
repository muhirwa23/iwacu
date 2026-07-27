//! Generate actor - generates code diffs

use crate::models::{FileDiff, GenerateInput, GenerateOutput, TestSpec};
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;

/// Generate actor for code generation
pub struct GenerateActor {
    message_rx: mpsc::Receiver<crate::models::ActorMessage>,
    response_tx: mpsc::Sender<crate::models::ActorMessage>,
}

impl GenerateActor {
    pub fn new(
        message_rx: mpsc::Receiver<crate::models::ActorMessage>,
        response_tx: mpsc::Sender<crate::models::ActorMessage>,
    ) -> Self {
        Self {
            message_rx,
            response_tx,
        }
    }

    /// Spawn the generate actor
    pub fn spawn(
        response_tx: mpsc::Sender<crate::models::ActorMessage>,
    ) -> mpsc::Sender<crate::models::ActorMessage> {
        let (tx, rx) = mpsc::channel(100);
        let actor = Self::new(rx, response_tx);

        tokio::spawn(async move {
            actor.run().await;
        });

        tx
    }

    /// Main event loop
    pub async fn run(mut self) {
        info!("Generate actor started");

        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                crate::models::ActorMessage::GenerateRequest(input) => {
                    if let Err(e) = self.handle_generate(input).await {
                        tracing::warn!("Generate error: {}", e);
                    }
                }
                _ => {
                    tracing::warn!("Unexpected message in Generate actor");
                }
            }
        }
    }

    /// Handle generate request
    async fn handle_generate(&self, input: GenerateInput) -> Result<()> {
        info!("Generating code for plan: {}", input.plan.description);

        // In production, this would call an LLM to generate diffs
        // For now, generate placeholder output
        
        let mut diffs = Vec::new();
        let mut tests = Vec::new();

        for file_path in &input.plan.files_to_modify {
            // Placeholder diff - in production would be LLM-generated
            let diff = FileDiff {
                file_path: file_path.clone(),
                diff: format!("// TODO: Generate diff for {}", file_path),
                new_content: None,
            };
            diffs.push(diff);
        }

        // Generate test specs
        for file_path in &input.plan.files_to_modify {
            let test = TestSpec {
                file_path: format!("tests/{}_test.rs", file_path.replace('/', "_").replace(".rs", "")),
                content: "// TODO: Generate test".to_string(),
                test_name: "generated_test".to_string(),
            };
            tests.push(test);
        }

        let output = GenerateOutput { diffs, tests };

        self.response_tx
            .send(crate::models::ActorMessage::GenerateResponse(output))
            .await?;

        Ok(())
    }
}
