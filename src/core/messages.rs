use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Error, Debug, miette::Diagnostic)]
pub enum SystemError {
    #[error("Environment error: {0}")]
    #[diagnostic(code(symbiote::environment_error))]
    Environment(String),

    #[error("Cognition error: {0}")]
    #[diagnostic(code(symbiote::cognition_error))]
    Cognition(String),

    #[error("Git error: {0}")]
    #[diagnostic(code(symbiote::git_error))]
    Git(String),

    #[error("Compiler error: {0}")]
    #[diagnostic(code(symbiote::compiler_error))]
    Compiler(String),

    #[error("LSP error: {0}")]
    #[diagnostic(code(symbiote::lsp_error))]
    Lsp(String),

    #[error("Mutation test failed")]
    #[diagnostic(code(symbiote::mutation_failed))]
    MutationFailed,

    #[error("Max attempts exceeded")]
    #[diagnostic(code(symbiote::max_attempts_exceeded))]
    MaxAttemptsExceeded,

    #[error("Timeout error: {0}")]
    #[diagnostic(code(symbiote::timeout_error))]
    Timeout(String),

    #[error("IO error: {0}")]
    #[diagnostic(code(symbiote::io_error))]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    #[diagnostic(code(symbiote::json_error))]
    Json(#[from] serde_json::Error),
}

/// Task context shared between actors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub task: String,
    pub lsp_context: Option<String>,
    pub plan: Option<Vec<String>>,
    pub current_diff: Option<String>,
    pub compiler_errors: Option<Value>,
    pub attempts: u32,
    pub worktree_path: Option<String>,
    pub affected_files: Vec<String>,
    pub symbols: Vec<SymbolInfo>,
}

impl TaskContext {
    pub fn new(task: String) -> Self {
        Self {
            task,
            lsp_context: None,
            plan: None,
            current_diff: None,
            compiler_errors: None,
            attempts: 0,
            worktree_path: None,
            affected_files: Vec::new(),
            symbols: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub kind: String,
}

/// Messages for the Cognition Actor
#[derive(Debug)]
pub enum CognitionRequest {
    Plan(TaskContext),
    Generate(TaskContext),
    Debug(TaskContext),
    Synthesize(TaskContext),
}

/// Messages for the Environment Actor
#[derive(Debug)]
pub enum EnvironmentRequest {
    Observe(String),
    ApplyDiff(String),
    RunTests,
    CleanupWorktree,
    GetDefinition(String),
    GetReferences(Location),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Generic actor message with reply channel
pub struct ActorMessage<T> {
    pub payload: T,
    pub reply_to: oneshot::Sender<Result<Value, SystemError>>,
}

impl<T> ActorMessage<T> {
    pub fn new(payload: T, reply_to: oneshot::Sender<Result<Value, SystemError>>) -> Self {
        Self { payload, reply_to }
    }
}

/// Test result from compiler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub test_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: String,
    pub line: u32,
    pub code: String,
    pub message: String,
    pub spans: Vec<Span>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub file_name: String,
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub text: String,
    pub label: Option<String>,
}
