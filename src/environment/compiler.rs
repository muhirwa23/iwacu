use std::path::Path;
use std::process::{Command, Stdio};
use serde_json::{Value, json};
use tracing::{info, debug, warn};

use crate::core::messages::{SystemError, TestResult, Diagnostic, Span, Suggestion};

/// Represents a dependency edge in the error graph: (parent_error_idx, child_error_idx)
/// where child_error is caused by parent_error
pub type ErrorEdge = (usize, usize);

/// Compiler error dependency graph for root-cause analysis
pub struct CompileErrorGraph {
    pub errors: Vec<Diagnostic>,
    pub dependency_edges: Vec<ErrorEdge>,
}

impl CompileErrorGraph {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            dependency_edges: Vec::new(),
        }
    }
    
    /// Add an error to the graph
    pub fn add_error(&mut self, error: Diagnostic) -> usize {
        let idx = self.errors.len();
        self.errors.push(error);
        idx
    }
    
    /// Add a dependency edge: child depends on parent (parent must be fixed first)
    pub fn add_dependency(&mut self, parent: usize, child: usize) {
        self.dependency_edges.push((parent, child));
    }
    
    /// Return errors in topological order (root causes first)
    pub fn topological_sort(&self) -> Vec<usize> {
        let n = self.errors.len();
        if n == 0 {
            return Vec::new();
        }
        
        // Build adjacency list and in-degree count
        let mut adj = vec![Vec::new(); n];
        let mut in_degree = vec![0; n];
        
        for &(parent, child) in &self.dependency_edges {
            if parent < n && child < n {
                adj[parent].push(child);
                in_degree[child] += 1;
            }
        }
        
        // Kahn's algorithm for topological sort
        let mut queue = Vec::new();
        for i in 0..n {
            if in_degree[i] == 0 {
                queue.push(i);
            }
        }
        
        let mut result = Vec::new();
        let mut head = 0;
        while head < queue.len() {
            let node = queue[head];
            head += 1;
            result.push(node);
            
            for &neighbor in &adj[node] {
                in_degree[neighbor] -= 1;
                if in_degree[neighbor] == 0 {
                    queue.push(neighbor);
                }
            }
        }
        
        // If not all nodes are in result, there's a cycle; just return what we have
        result
    }
    
    /// Convert to JSON string for LLM consumption
    pub fn to_json(&self) -> String {
        let sorted_indices = self.topological_sort();
        let sorted_errors: Vec<&Diagnostic> = sorted_indices
            .iter()
            .filter_map(|&i| self.errors.get(i))
            .collect();
        
        json!({
            "errors": sorted_errors,
            "root_causes": sorted_indices.iter().take(3).collect::<Vec<_>>()
        })
        .to_string()
    }
}

/// Compiler operations for running tests and parsing output
pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    /// Run cargo test with JSON message format and return error graph
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

        // Parse JSON output into error graph
        let (error_graph, success) = self.parse_diagnostics_with_graph(&stdout, &stderr);

        let test_output = if !output.status.success() {
            format!("{}\n{}", stdout, stderr)
        } else {
            stdout.to_string()
        };

        Ok(TestResult {
            success,
            diagnostics: error_graph.errors,
            test_output,
        })
    }

    /// Parse compiler diagnostics from JSON output and build dependency graph
    fn parse_diagnostics_with_graph(&self, stdout: &str, stderr: &str) -> (CompileErrorGraph, bool) {
        let mut graph = CompileErrorGraph::new();
        let mut has_errors = false;
        let mut error_indices_by_file_line: std::collections::HashMap<(String, u32), usize> = std::collections::HashMap::new();

        // Parse each line of stdout as JSON
        for line in stdout.lines() {
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                // Check for compiler messages
                if let Some(message) = value.get("message") {
                    if let Some(msg_obj) = message.as_object() {
                        has_errors = true;

                        let file = msg_obj
                            .get("file")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        let line_num = msg_obj
                            .get("line")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32;

                        let diag = Diagnostic {
                            file: file.clone(),
                            line: line_num,
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

                        let idx = graph.add_error(diag);
                        error_indices_by_file_line.insert((file.clone(), line_num), idx);
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

        // Build dependency edges based on file/line proximity
        // Heuristic: errors on earlier lines in the same file often cause later errors
        let mut errors_by_file: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
        for (idx, error) in graph.errors.iter().enumerate() {
            errors_by_file.entry(error.file.clone()).or_default().push(idx);
        }
        
        // For each file, sort errors by line and create dependency edges
        for (_, mut indices) in errors_by_file {
            indices.sort_by_key(|&idx| graph.errors[idx].line);
            
            // Create edges: earlier errors may cause later ones
            for i in 1..indices.len() {
                let parent = indices[i - 1];
                let child = indices[i];
                
                // Check if child error message references parent location
                if graph.errors[child].message.contains(&graph.errors[parent].file) ||
                   graph.errors[child].message.contains("defined here") ||
                   graph.errors[child].message.contains("previous location") {
                    graph.add_dependency(parent, child);
                }
            }
        }

        // Also check stderr for any non-JSON errors
        if !has_errors && !stderr.is_empty() {
            // Look for error patterns in stderr
            if stderr.contains("error:") || stderr.contains("FAILED") {
                has_errors = true;
                
                // Add a diagnostic for stderr content
                let idx = graph.add_error(Diagnostic {
                    file: "stderr".to_string(),
                    line: 0,
                    code: "compiler".to_string(),
                    message: stderr.lines().take(5).collect::<Vec<_>>().join("\n"),
                    spans: Vec::new(),
                    suggestions: Vec::new(),
                });
                
                // Make stderr errors depend on nothing (they're root causes)
                // No edges needed since it's already a root
            }
        }

        (graph, !has_errors)
    }
    
    /// Legacy method for backward compatibility
    fn parse_diagnostics(&self, stdout: &str, stderr: &str) -> (Vec<Diagnostic>, bool) {
        let (graph, success) = self.parse_diagnostics_with_graph(stdout, stderr);
        (graph.errors, success)
    }
}
