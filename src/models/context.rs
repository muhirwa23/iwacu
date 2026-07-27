//! Context payload from AST-Indexed Semantic Grounding

use serde::{Deserialize, Serialize};

/// Information about a symbol resolved via LSP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    /// Full path to the symbol (e.g., "crate::auth::login")
    pub path: String,
    /// File location
    pub file_path: String,
    /// Line number (0-indexed)
    pub line: usize,
    /// Column number (0-indexed)
    pub column: usize,
    /// Function signature if applicable
    pub signature: Option<String>,
    /// All callers up to depth 2
    pub callers: Vec<SymbolReference>,
    /// All callees (depth 1)
    pub callees: Vec<SymbolReference>,
}

/// A reference to a symbol (caller or callee)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference {
    pub path: String,
    pub file_path: String,
    pub line: usize,
}

/// Detected convention pattern (e.g., "All routes return Result<Json<T>, AppError>")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConventionPattern {
    pub description: String,
    pub example_path: String,
    pub confidence: f32,
}

/// Context payload sent from Observe to Plan actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPayload {
    /// The original query/task
    pub task: String,
    /// Resolved symbols relevant to the task
    pub symbols: Vec<SymbolInfo>,
    /// Detected conventions in the codebase
    pub conventions: Vec<ConventionPattern>,
    /// Files that may need modification
    pub affected_files: Vec<String>,
}

impl ContextPayload {
    pub fn new(task: impl Into<String>) -> Self {
        Self {
            task: task.into(),
            symbols: Vec::new(),
            conventions: Vec::new(),
            affected_files: Vec::new(),
        }
    }

    pub fn with_symbol(mut self, symbol: SymbolInfo) -> Self {
        self.symbols.push(symbol);
        self.affected_files.push(symbol.file_path.clone());
        self
    }

    pub fn with_convention(mut self, pattern: ConventionPattern) -> Self {
        self.conventions.push(pattern);
        self
    }
}
