use crate::core::messages::{TaskContext, SymbolInfo};

/// Extended context operations
impl TaskContext {
    /// Add a symbol to the context and track affected files
    pub fn with_symbol(mut self, symbol: SymbolInfo) -> Self {
        let file_path = symbol.file_path.clone();
        self.symbols.push(symbol);
        if !self.affected_files.contains(&file_path) {
            self.affected_files.push(file_path);
        }
        self
    }

    /// Increment attempt counter
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }

    /// Set worktree path
    pub fn with_worktree(mut self, path: String) -> Self {
        self.worktree_path = Some(path);
        self
    }
}
