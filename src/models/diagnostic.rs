//! Diagnostic graph for compiler error dependency tracking

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single compiler diagnostic from cargo test --message-format=json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Error code (e.g., "E0277")
    pub code: Option<String>,
    /// Error message
    pub message: String,
    /// File path where error occurred
    pub file_path: String,
    /// Line number (0-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
    /// Suggested fix if available
    pub suggestion: Option<String>,
    /// Whether this error is localized (can be fixed in isolation)
    pub is_localized: bool,
}

/// Dependency between errors (Error B depends on Error A)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDependency {
    /// The error that must be fixed first
    pub predecessor: String,
    /// The error that depends on the predecessor
    pub dependent: String,
}

/// Graph of compiler errors with dependency tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosticGraph {
    /// All diagnostics indexed by unique ID
    pub diagnostics: HashMap<String, Diagnostic>,
    /// Dependencies between errors
    pub dependencies: Vec<ErrorDependency>,
    /// Order in which errors should be fixed (topologically sorted)
    pub fix_order: Vec<String>,
}

impl DiagnosticGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a diagnostic to the graph
    pub fn add_diagnostic(&mut self, id: String, diagnostic: Diagnostic) {
        self.diagnostics.insert(id, diagnostic);
    }

    /// Add a dependency edge
    pub fn add_dependency(&mut self, predecessor: String, dependent: String) {
        self.dependencies.push(ErrorDependency {
            predecessor,
            dependent,
        });
    }

    /// Compute topological order for fixing errors
    /// Errors with no dependencies are fixed first
    pub fn compute_fix_order(&mut self) {
        use std::collections::VecDeque;

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();

        // Initialize
        for id in self.diagnostics.keys() {
            in_degree.insert(id.clone(), 0);
            adj.insert(id.clone(), Vec::new());
        }

        // Build graph (predecessor -> dependent means we fix predecessor first)
        for dep in &self.dependencies {
            if let Some(neighbors) = adj.get_mut(&dep.predecessor) {
                neighbors.push(dep.dependent.clone());
            }
            if let Some(degree) = in_degree.get_mut(&dep.dependent) {
                *degree += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        self.fix_order.clear();

        while let Some(id) = queue.pop_front() {
            self.fix_order.push(id.clone());
            if let Some(neighbors) = adj.get(&id) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }
    }

    /// Get the next error to fix
    pub fn next_error_to_fix(&self) -> Option<&Diagnostic> {
        self.fix_order.first().and_then(|id| self.diagnostics.get(id))
    }

    /// Remove a fixed error and update the graph
    pub fn mark_fixed(&mut self, id: &str) {
        self.diagnostics.remove(id);
        self.dependencies.retain(|d| d.predecessor != id && d.dependent != id);
        self.fix_order.retain(|x| x != id);
    }
}
