//! Test actor - runs tests with mutation gating

use crate::models::{MutationTestResult, TestInput, TestResult};
use crate::tools::{CompilerTools, Mutator};
use anyhow::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::info;

/// Test actor for running tests and mutation testing
pub struct TestActor {
    project_root: PathBuf,
    compiler_tools: CompilerTools,
    mutator: Mutator,
    message_rx: mpsc::Receiver<crate::models::ActorMessage>,
    response_tx: mpsc::Sender<crate::models::ActorMessage>,
}

impl TestActor {
    pub fn new(
        project_root: &std::path::Path,
        message_rx: mpsc::Receiver<crate::models::ActorMessage>,
        response_tx: mpsc::Sender<crate::models::ActorMessage>,
    ) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            compiler_tools: CompilerTools::new(project_root),
            mutator: Mutator::new(10), // Max 10 mutants per file
            message_rx,
            response_tx,
        }
    }

    /// Spawn the test actor
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
        info!("Test actor started");

        while let Some(msg) = self.message_rx.recv().await {
            match msg {
                crate::models::ActorMessage::TestRequest(input) => {
                    if let Err(e) = self.handle_test(input).await {
                        tracing::warn!("Test error: {}", e);
                    }
                }
                _ => {
                    tracing::warn!("Unexpected message in Test actor");
                }
            }
        }
    }

    /// Handle test request
    async fn handle_test(&self, input: TestInput) -> Result<()> {
        info!("Running tests for worktree {}", input.worktree_id);

        let worktree_path = self.project_root.join(".git/worktrees").join(&input.worktree_id);

        // First run cargo check to see if it compiles
        let check_result = self.compiler_tools.check(&worktree_path).await?;

        if !check_result.success {
            // Parse diagnostics
            let diagnostic_graph = self.compiler_tools.parse_diagnostics(&check_result.stdout);

            let result = TestResult {
                passed: false,
                compilation_failed: true,
                diagnostic_graph: Some(diagnostic_graph),
                mutation_result: None,
                output: check_result.stderr,
            };

            self.response_tx
                .send(crate::models::ActorMessage::TestResponse(result))
                .await?;

            return Ok(());
        }

        // Run actual tests
        let test_result = self.compiler_tools.run_tests(&worktree_path).await?;

        if !test_result.success {
            // Parse diagnostics from test failures
            let diagnostic_graph = self.compiler_tools.parse_diagnostics(&test_result.stdout);

            let result = TestResult {
                passed: false,
                compilation_failed: false,
                diagnostic_graph: Some(diagnostic_graph),
                mutation_result: None,
                output: test_result.stderr,
            };

            self.response_tx
                .send(crate::models::ActorMessage::TestResponse(result))
                .await?;

            return Ok(());
        }

        // Tests passed - now run mutation testing if requested
        let mutation_result = if input.run_mutation {
            let files_to_mutate: Vec<&str> = input
                .test_specs
                .iter()
                .map(|s| s.file_path.as_str())
                .collect();

            if !files_to_mutate.is_empty() {
                let result = self
                    .mutator
                    .run_mutation_testing(&worktree_path, &files_to_mutate)
                    .await?;

                Some(MutationTestResult {
                    mutants_generated: result.mutants_generated,
                    mutants_killed: result.mutants_killed,
                    mutants_survived: result.mutants_survived,
                    tests_valid: result.tests_valid,
                })
            } else {
                None
            }
        } else {
            None
        };

        let result = TestResult {
            passed: true,
            compilation_failed: false,
            diagnostic_graph: None,
            mutation_result,
            output: test_result.stdout,
        };

        self.response_tx
            .send(crate::models::ActorMessage::TestResponse(result))
            .await?;

        Ok(())
    }
}
