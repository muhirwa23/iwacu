//! Compiler tools - cargo test parsing and diagnostic extraction

use crate::models::{Diagnostic, DiagnosticGraph};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use tokio::process::Command;

/// Compiler tools for running tests and parsing diagnostics
pub struct CompilerTools {
    project_root: std::path::PathBuf,
}

impl CompilerTools {
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
        }
    }

    /// Run cargo test with JSON message format
    /// Returns raw JSON output for parsing
    pub async fn run_tests(&self, worktree_path: &Path) -> Result<TestOutput> {
        let output = Command::new("cargo")
            .arg("test")
            .arg("--message-format=json")
            .current_dir(worktree_path)
            .output()
            .await
            .context("Failed to run cargo test")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(TestOutput {
            success: output.status.success(),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        })
    }

    /// Run cargo check to verify compilation without running tests
    pub async fn check(&self, worktree_path: &Path) -> Result<TestOutput> {
        let output = Command::new("cargo")
            .arg("check")
            .arg("--message-format=json")
            .current_dir(worktree_path)
            .output()
            .await
            .context("Failed to run cargo check")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(TestOutput {
            success: output.status.success(),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        })
    }

    /// Parse JSON compiler messages into a diagnostic graph
    pub fn parse_diagnostics(&self, json_output: &str) -> DiagnosticGraph {
        use std::collections::HashSet;

        let mut graph = DiagnosticGraph::new();
        let mut error_locations: HashSet<(String, usize)> = HashSet::new();

        for line in json_output.lines() {
            if let Ok(msg) = serde_json::from_str::<CargoMessage>(line) {
                if msg.reason == "compiler-message" {
                    if let Some(rendered) = msg.message {
                        // Parse the diagnostic
                        let diagnostic = Diagnostic {
                            code: rendered.code.map(|c| c.code),
                            message: rendered.message.clone(),
                            file_path: rendered
                                .spans
                                .first()
                                .map(|s| s.file_name.clone())
                                .unwrap_or_default(),
                            line: rendered
                                .spans
                                .first()
                                .map(|s| s.line_start)
                                .unwrap_or(0),
                            column: rendered
                                .spans
                                .first()
                                .map(|s| s.column_start)
                                .unwrap_or(0),
                            suggestion: rendered.suggestions.first().and_then(|s| {
                                s.replacement.as_ref().map(|r| r.text.clone())
                            }),
                            is_localized: true, // Will be refined by shadow compilation
                        };

                        // Create unique ID from file + line
                        let id = format!("{}:{}", diagnostic.file_path, diagnostic.line);
                        
                        // Track error locations for dependency detection
                        let loc = (diagnostic.file_path.clone(), diagnostic.line);
                        
                        if !error_locations.contains(&loc) {
                            error_locations.insert(loc);
                            graph.add_diagnostic(id, diagnostic);
                        }
                    }
                }
            }
        }

        // Infer dependencies based on file order and cascading errors
        self.infer_dependencies(&mut graph);

        graph.compute_fix_order();
        graph
    }

    /// Infer dependencies between errors
    /// Heuristic: errors in the same file, later lines often depend on earlier lines
    fn infer_dependencies(&self, graph: &mut DiagnosticGraph) {
        use std::collections::HashMap;

        let mut errors_by_file: HashMap<String, Vec<String>> = HashMap::new();

        for (id, diag) in &graph.diagnostics {
            errors_by_file
                .entry(diag.file_path.clone())
                .or_insert_with(Vec::new)
                .push((id.clone(), diag.line));
        }

        // Sort by line number and create dependencies
        for (_, mut errors) in errors_by_file {
            errors.sort_by_key(|(_, line)| *line);
            
            for i in 1..errors.len() {
                let prev_id = errors[i - 1].0.clone();
                let curr_id = errors[i].0.clone();
                graph.add_dependency(prev_id, curr_id);
            }
        }
    }

    /// Shadow compilation: comment out failing function and re-check
    /// Returns true if error is localized (rest of file compiles)
    pub async fn shadow_compile(
        &self,
        worktree_path: &Path,
        file_path: &str,
        start_line: usize,
        end_line: usize,
    ) -> Result<bool> {
        use std::fs;

        let full_path = worktree_path.join(file_path);
        let content = fs::read_to_string(&full_path)?;
        
        // Comment out the failing region
        let modified = comment_out_region(&content, start_line, end_line);
        
        // Write modified version
        fs::write(&full_path, &modified)?;
        
        // Check if rest compiles
        let result = self.check(worktree_path).await?;
        
        // Restore original
        fs::write(&full_path, &content)?;
        
        Ok(result.success)
    }

    /// Fetch rustc documentation for an error code
    pub async fn fetch_error_docs(&self, error_code: &str) -> Result<String> {
        // E.g., E0277 -> https://doc.rust-lang.org/error_codes/E0277.html
        let url = format!(
            "https://doc.rust-lang.org/error_codes/{}.html",
            error_code.trim_start_matches('E')
        );

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await?;
        
        if response.status().is_success() {
            let html = response.text().await?;
            // Extract main content (simplified - would use HTML parser in production)
            Ok(extract_text_from_html(&html))
        } else {
            Ok(format!("No documentation found for {}", error_code))
        }
    }
}

/// Output from cargo test/check
#[derive(Debug)]
pub struct TestOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Cargo JSON message structure
#[derive(Debug, Deserialize)]
struct CargoMessage {
    reason: String,
    #[serde(flatten)]
    message: Option<RenderedDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct RenderedDiagnostic {
    message: Option<String>,
    code: Option<ErrorCode>,
    spans: Vec<Span>,
    suggestions: Vec<Suggestion>,
}

#[derive(Debug, Deserialize)]
struct ErrorCode {
    code: String,
    explanation: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Span {
    file_name: String,
    line_start: usize,
    line_end: usize,
    column_start: usize,
    column_end: usize,
    is_primary: bool,
}

#[derive(Debug, Deserialize)]
struct Suggestion {
    replacement: Option<Replacement>,
}

#[derive(Debug, Deserialize)]
struct Replacement {
    text: String,
}

/// Comment out a region of Rust code
fn comment_out_region(content: &str, start_line: usize, end_line: usize) -> String {
    let mut result = String::new();
    
    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1; // 1-indexed
        if line_num >= start_line && line_num <= end_line {
            result.push_str("// [SHADOW COMPILE]: ");
            result.push_str(line);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    
    result
}

/// Extract readable text from HTML (simplified)
fn extract_text_from_html(html: &str) -> String {
    // In production, would use scraper or similar
    // For now, return a truncated version
    html.chars().take(2000).collect()
}
