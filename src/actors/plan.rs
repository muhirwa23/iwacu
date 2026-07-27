//! Plan actor - generates plans with constraint satisfaction

use crate::models::{ConstraintSatisfactionMatrix, ContextPayload, PlanResult};
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;

/// Plan actor for generating implementation plans
pub struct PlanActor {
    message_rx: mpsc::Receiver<crate::models::ActorMessage>,
    response_tx: mpsc::Sender<crate::models::ActorMessage>,
}

impl PlanActor {
    pub fn new(
        message_rx: mpsc::Receiver<crate::models::ActorMessage>,
        response_tx: mpsc::Sender<crate::models::ActorMessage>,
    ) -> Self {
        Self {
            message_rx,
            response_tx,
        }
    }

    /// Spawn the plan actor
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
        info!("Plan actor started");

        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                crate::models::ActorMessage::PlanRequest(context) => {
                    if let Err(e) = self.handle_plan(context).await {
                        tracing::warn!("Plan error: {}", e);
                    }
                }
                _ => {
                    tracing::warn!("Unexpected message in Plan actor");
                }
            }
        }
    }

    /// Handle plan request
    async fn handle_plan(&self, context: ContextPayload) -> Result<()> {
        info!("Planning for task: {}", context.task);

        // In production, this would call an LLM to generate a plan
        // For now, generate a deterministic plan based on context
        
        let mut constraints = ConstraintSatisfactionMatrix::new();
        
        // Analyze context and add constraints
        for convention in &context.conventions {
            // If conventions exist, constrain the output to match
            constraints.add_constraint(
                format!("convention_{}", convention.description.chars().take(20).collect::<String>()),
                false,
                vec![],
                Some(convention.description.clone()),
            );
        }

        // Determine files to modify from context
        let files_to_modify = context.affected_files.clone();

        // Generate plan description
        let description = format!(
            "Implement: {}\nAffected files: {:?}",
            context.task, files_to_modify
        );

        let plan_result = PlanResult {
            description,
            constraints,
            files_to_modify,
            requires_user_input: false,
        };

        self.response_tx
            .send(crate::models::ActorMessage::PlanResponse(plan_result))
            .await?;

        Ok(())
    }
}
