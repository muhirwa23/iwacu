//! Orchestrator actor - routes messages between actors

use crate::models::*;
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Message types for the orchestrator channel
#[derive(Debug)]
pub enum OrchestratorCommand {
    /// Start a new task
    RunTask { task: String },
    /// Continue with next step after response
    Continue(ActorMessage),
    /// Stop processing
    Stop,
}

/// The Orchestrator actor - supervises all other actors
pub struct Orchestrator {
    /// Channel to send commands to orchestrator
    pub command_tx: mpsc::Sender<OrchestratorCommand>,
    /// Channel to receive commands from orchestrator
    command_rx: mpsc::Receiver<OrchestratorCommand>,
    /// Channel to send messages to Observe actor
    observe_tx: mpsc::Sender<ActorMessage>,
    /// Channel to send messages to Plan actor
    plan_tx: mpsc::Sender<ActorMessage>,
    /// Channel to send messages to Generate actor
    generate_tx: mpsc::Sender<ActorMessage>,
    /// Channel to send messages to Apply actor
    apply_tx: mpsc::Sender<ActorMessage>,
    /// Channel to send messages to Test actor
    test_tx: mpsc::Sender<ActorMessage>,
    /// Channel to send messages to Debug actor
    debug_tx: mpsc::Sender<ActorMessage>,
    /// Current task state
    current_task: Option<String>,
    /// Number of fix attempts
    fix_attempts: usize,
    /// Maximum fix attempts before escalation
    max_fix_attempts: usize,
}

impl Orchestrator {
    /// Create a new orchestrator with channels
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_tx: mpsc::Sender<OrchestratorCommand>,
        command_rx: mpsc::Receiver<OrchestratorCommand>,
        observe_tx: mpsc::Sender<ActorMessage>,
        plan_tx: mpsc::Sender<ActorMessage>,
        generate_tx: mpsc::Sender<ActorMessage>,
        apply_tx: mpsc::Sender<ActorMessage>,
        test_tx: mpsc::Sender<ActorMessage>,
        debug_tx: mpsc::Sender<ActorMessage>,
    ) -> Self {
        Self {
            command_tx,
            command_rx,
            observe_tx,
            plan_tx,
            generate_tx,
            apply_tx,
            test_tx,
            debug_tx,
            current_task: None,
            fix_attempts: 0,
            max_fix_attempts: 3,
        }
    }

    /// Spawn the orchestrator actor
    pub fn spawn(
        observe_tx: mpsc::Sender<ActorMessage>,
        plan_tx: mpsc::Sender<ActorMessage>,
        generate_tx: mpsc::Sender<ActorMessage>,
        apply_tx: mpsc::Sender<ActorMessage>,
        test_tx: mpsc::Sender<ActorMessage>,
        debug_tx: mpsc::Sender<ActorMessage>,
    ) -> mpsc::Sender<OrchestratorCommand> {
        let (command_tx, command_rx) = mpsc::channel(100);
        let tx_clone = command_tx.clone();

        tokio::spawn(async move {
            let mut orchestrator = Orchestrator::new(
                tx_clone,
                command_rx,
                observe_tx,
                plan_tx,
                generate_tx,
                apply_tx,
                test_tx,
                debug_tx,
            );
            orchestrator.run().await;
        });

        command_tx
    }

    /// Main event loop for the orchestrator
    pub async fn run(&mut self) {
        info!("Orchestrator started");

        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                OrchestratorCommand::RunTask { task } => {
                    info!("Starting task: {}", task);
                    self.current_task = Some(task.clone());
                    self.fix_attempts = 0;

                    // Start with Observe phase
                    let msg = ActorMessage::ObserveRequest { task };
                    if let Err(e) = self.observe_tx.send(msg).await {
                        warn!("Failed to send to Observe: {}", e);
                    }
                }

                OrchestratorCommand::Continue(response) => {
                    if let Err(e) = self.handle_response(response).await {
                        warn!("Error handling response: {}", e);
                    }
                }

                OrchestratorCommand::Stop => {
                    info!("Orchestrator stopping");
                    break;
                }
            }
        }
    }

    /// Handle responses from child actors
    async fn handle_response(&mut self, msg: ActorMessage) -> Result<()> {
        match msg {
            ActorMessage::ObserveResponse(context) => {
                // Route to Plan
                self.plan_tx
                    .send(ActorMessage::PlanRequest(context))
                    .await?;
            }

            ActorMessage::PlanResponse(plan_result) => {
                // Check constraints
                if plan_result.requires_user_input {
                    // In production, would prompt user here
                    warn!("User input required - not implemented");
                }

                // Route to Generate
                let input = GenerateInput {
                    plan: plan_result,
                    context: ContextPayload::new(self.current_task.clone().unwrap_or_default()),
                };
                self.generate_tx
                    .send(ActorMessage::GenerateRequest(input))
                    .await?;
            }

            ActorMessage::GenerateOutput(output) => {
                // Route to Apply
                let input = ApplyInput {
                    worktree_id: "default".to_string(), // Would be generated
                    diffs: output.diffs,
                };
                self.apply_tx
                    .send(ActorMessage::ApplyRequest(input))
                    .await?;
            }

            ActorMessage::ApplyResponse(result) => {
                if result.success {
                    // Route to Test
                    let input = TestInput {
                        worktree_id: result.worktree_id,
                        test_specs: vec![], // Would come from GenerateOutput
                        run_mutation: true,
                    };
                    self.test_tx.send(ActorMessage::TestRequest(input)).await?;
                } else {
                    warn!("Apply failed: {:?}", result.error);
                }
            }

            ActorMessage::TestResponse(result) => {
                if result.passed {
                    info!("All tests passed!");
                    // Task complete - would synthesize PR here
                } else if result.compilation_failed {
                    // Route to Debug
                    if let Some(graph) = result.diagnostic_graph {
                        if self.fix_attempts < self.max_fix_attempts {
                            self.fix_attempts += 1;
                            let input = DebugInput {
                                diagnostic_graph: graph,
                                worktree_id: "default".to_string(),
                                context: ContextPayload::new(
                                    self.current_task.clone().unwrap_or_default(),
                                ),
                            };
                            self.debug_tx
                                .send(ActorMessage::DebugRequest(input))
                                .await?;
                        } else {
                            // Escalate to Plan
                            warn!("Max fix attempts reached, escalating to Plan");
                        }
                    }
                }
            }

            ActorMessage::DebugResponse(output) => {
                if output.escalate {
                    warn!("Debug escalated to Plan: {:?}", output.escalation_reason);
                    // Would send EscalateToPlan message
                } else {
                    // Route fixes back to Apply
                    let input = ApplyInput {
                        worktree_id: "default".to_string(),
                        diffs: output.fixes,
                    };
                    self.apply_tx
                        .send(ActorMessage::ApplyRequest(input))
                        .await?;
                }
            }

            _ => {
                warn!("Unhandled message type");
            }
        }

        Ok(())
    }
}
