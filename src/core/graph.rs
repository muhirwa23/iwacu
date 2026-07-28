use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Duration};
use serde_json::Value;
use tracing::{info, warn, error};

use crate::core::messages::{
    ActorMessage, CognitionRequest, EnvironmentRequest, TaskContext, SystemError,
};

/// Supervisor actor that orchestrates the task execution flow
pub struct Supervisor {
    cognition_tx: mpsc::Sender<ActorMessage<CognitionRequest>>,
    environment_tx: mpsc::Sender<ActorMessage<EnvironmentRequest>>,
    task_timeout: Duration,
}

impl Supervisor {
    pub fn new(
        cognition_tx: mpsc::Sender<ActorMessage<CognitionRequest>>,
        environment_tx: mpsc::Sender<ActorMessage<EnvironmentRequest>>,
    ) -> Self {
        Self {
            cognition_tx,
            environment_tx,
            task_timeout: Duration::from_secs(900), // 15 minutes timeout per phase (slow reasoning models)
        }
    }

    /// Execute the full task graph
    pub async fn run_task(&self, mut context: TaskContext) -> Result<(), SystemError> {
        info!("Starting task execution: {}", context.task);

        // Step 1: Observe - Get LSP context
        info!("Phase 1: Observing codebase");
        let (tx, rx) = oneshot::channel();
        self.environment_tx
            .send(ActorMessage::new(
                EnvironmentRequest::Observe(context.task.clone()),
                tx,
            ))
            .await
            .map_err(|e| SystemError::Environment(e.to_string()))?;

        let lsp_result = timeout(self.task_timeout, rx)
            .await
            .map_err(|_| SystemError::Timeout("Timeout waiting for LSP context".to_string()))?
            .map_err(|e| SystemError::Environment(e.to_string()))??;
        
        context.lsp_context = Some(lsp_result.to_string());
        info!("LSP context gathered");

        // Step 2: Plan - Generate plan from LLM
        info!("Phase 2: Planning");
        let (tx, rx) = oneshot::channel();
        self.cognition_tx
            .send(ActorMessage::new(CognitionRequest::Plan(context.clone()), tx))
            .await
            .map_err(|e| SystemError::Cognition(e.to_string()))?;

        let plan_result = timeout(self.task_timeout, rx)
            .await
            .map_err(|_| SystemError::Timeout("Timeout waiting for plan".to_string()))?
            .map_err(|e| SystemError::Cognition(e.to_string()))??;
        
        if let Value::Array(arr) = plan_result {
            context.plan = Some(arr.into_iter().map(|v| v.as_str().unwrap_or("").to_string()).collect());
        }
        info!("Plan generated");

        // Step 3-7: Generate -> Apply -> Test loop (max 3 attempts)
        for attempt in 0..3 {
            context.attempts = attempt;
            info!("Phase 3: Generation attempt {}", attempt + 1);

            // Step 3: Generate diff
            let (tx, rx) = oneshot::channel();
            self.cognition_tx
                .send(ActorMessage::new(CognitionRequest::Generate(context.clone()), tx))
                .await
                .map_err(|e| SystemError::Cognition(e.to_string()))?;

            let diff_result = timeout(self.task_timeout, rx)
                .await
                .map_err(|_| SystemError::Timeout("Timeout waiting for diff generation".to_string()))?
                .map_err(|e| SystemError::Cognition(e.to_string()))??;
            
            context.current_diff = Some(diff_result.as_str().unwrap_or("").to_string());
            info!("Diff generated");

            // Step 4: Apply diff
            info!("Phase 4: Applying diff");
            let (tx, rx) = oneshot::channel();
            self.environment_tx
                .send(ActorMessage::new(
                    EnvironmentRequest::ApplyDiff(context.current_diff.clone().unwrap()),
                    tx,
                ))
                .await
                .map_err(|e| SystemError::Environment(e.to_string()))?;

            let apply_result = timeout(self.task_timeout, rx)
                .await
                .map_err(|_| SystemError::Timeout("Timeout waiting for diff application".to_string()))?
                .map_err(|e| SystemError::Environment(e.to_string()))??;
            
            info!("Diff applied: {}", apply_result);

            // Step 5: Run tests
            info!("Phase 5: Running tests");
            let (tx, rx) = oneshot::channel();
            self.environment_tx
                .send(ActorMessage::new(EnvironmentRequest::RunTests, tx))
                .await
                .map_err(|e| SystemError::Environment(e.to_string()))?;

            let test_result = timeout(self.task_timeout, rx)
                .await
                .map_err(|_| SystemError::Timeout("Timeout waiting for test results".to_string()))?
                .map_err(|e| SystemError::Environment(e.to_string()))??;
            
            let test_success = test_result.get("success").and_then(|v| v.as_bool()).unwrap_or(false);

            if test_success {
                // Step 6: Mutation gate
                info!("Phase 6: Running mutation tests");
                let mutation_passed = test_result.get("mutation_passed").and_then(|v| v.as_bool()).unwrap_or(false);
                
                if mutation_passed {
                    info!("All tests passed including mutation gate");
                    
                    // Step 7: Synthesize
                    info!("Phase 7: Synthesizing results");
                    let (tx, rx) = oneshot::channel();
                    self.cognition_tx
                        .send(ActorMessage::new(CognitionRequest::Synthesize(context.clone()), tx))
                        .await
                        .map_err(|e| SystemError::Cognition(e.to_string()))?;

                    let _synth_result = timeout(self.task_timeout, rx)
                        .await
                        .map_err(|_| SystemError::Timeout("Timeout waiting for synthesis".to_string()))?
                        .map_err(|e| SystemError::Cognition(e.to_string()))??;
                    
                    info!("Task completed successfully");
                    return Ok(());
                } else {
                    warn!("Mutation gate failed - tests do not properly verify logic");
                    context.compiler_errors = Some(serde_json::json!({
                        "message": "Mutation gate failed: tests pass on mutated code"
                    }));
                }
            } else {
                warn!("Tests failed on attempt {}", attempt + 1);
                context.compiler_errors = test_result.get("diagnostics").cloned();
            }

            // If we reach here, tests failed or mutation gate failed
            if attempt < 2 {
                // Step 8: Debug and retry
                info!("Phase 8: Debugging");
                let (tx, rx) = oneshot::channel();
                self.cognition_tx
                    .send(ActorMessage::new(CognitionRequest::Debug(context.clone()), tx))
                    .await
                    .map_err(|e| SystemError::Cognition(e.to_string()))?;

                let debug_result = timeout(self.task_timeout, rx)
                    .await
                    .map_err(|_| SystemError::Timeout("Timeout waiting for debug response".to_string()))?
                    .map_err(|e| SystemError::Cognition(e.to_string()))??;
                
                context.current_diff = Some(debug_result.as_str().unwrap_or("").to_string());
            } else {
                error!("Max attempts exceeded");
                
                // Cleanup worktree before returning
                let (cleanup_tx, cleanup_rx) = oneshot::channel();
                self.environment_tx
                    .send(ActorMessage::new(EnvironmentRequest::CleanupWorktree, cleanup_tx))
                    .await
                    .ok();
                
                let _ = cleanup_rx.await;
                
                return Err(SystemError::MaxAttemptsExceeded);
            }
        }

        Ok(())
    }
}
