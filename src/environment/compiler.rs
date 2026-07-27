use std::path::Path;
use std::process::{Command, Stdio};
use serde_json::{Value, json};
use tracing::{info, debug, warn};

use crate::core::messages::{SystemError, TestResult, Diagnostic, Span, Suggestion};

/// Compiler operations for running tests and parsing output
pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    /// Run cargo test with JSON message format
    pub fn run_tests(&self, worktree_path: &str) -> Result<TestResult, SystemError> {
        info!("Running tests in: {}", worktree_path);

        let output = Command::new("cargo")
            .args(["test", "--message-format=json"])
            .current_dir(worktree_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| SystemError::Compiler(format!("Failed to run cargo test: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Parse JSON output
        let (diagnostics, success) = self.parse_diagnostics(&stdout, &stderr);

        let test_output = if !output.status.success() {
            format!("{}\n{}", stdout, stderr)
        } else {
            stdout.to_string()
        };

        Ok(TestResult {
            success,
            diagnostics,
            test_output,
        })
    }

    /// Parse compiler diagnostics from JSON output
    fn parse_diagnostics(&self, stdout: &str, stderr: &str) -> (Vec<Diagnostic>, bool) {
        let mut diagnostics = Vec::new();
        let mut has_errors = false;

        // Parse each line of stdout as JSON
        for line in stdout.lines() {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                // Check for compiler messages
                if let Some(message) = value.get("message") {
                    if let Some(msg_obj) = message.as_object() {
                        has_errors = true;

                        let diag = Diagnostic {
                            file: msg_obj
                                .get("file")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            line: msg_obj
                                .get("line")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32,
                            code: msg_obj
                                .get("code")
                                .and_then(|c| c.get("code"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            message: msg_obj
                                .get("rendered")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            spans: Vec::new(),
                            suggestions: Vec::new(),
                        };

                        diagnostics.push(diag);
                    }
                }

                // Check for test results
                if let Some(reason) = value.get("reason").and_then(|v| v.as_str()) {
                    if reason == "test" {
                        if let Some(event) = value.get("event").and_then(|v| v.as_str()) {
                            if event == "failed" {
                                has_errors = true;
                            }
                        }
                    }
                }
            }
        }

        // Also check stderr for any non-JSON errors
        if !has_errors && !stderr.is_empty() {
            // Look for error patterns in stderr
            if stderr.contains("error:") || stderr.contains("FAILED") {
                has_errors = true;
                
                // Add a diagnostic for stderr content
                diagnostics.push(Diagnostic {
                    file: "stderr".to_string(),
                    line: 0,
                    code: "compiler".to_string(),
                    message: stderr.lines().take(5).collect::<Vec<_>>().join("\n"),
                    spans: Vec::new(),
                    suggestions: Vec::new(),
                });
            }
        }

        (diagnostics, !has_errors)
    }
}
