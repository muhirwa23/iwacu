//! Mutation testing result model

use serde::{Deserialize, Serialize};

/// Result of mutation testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationResult {
    /// Total mutants generated
    pub mutants_generated: usize,
    /// Mutants that were killed by tests (good - tests caught the bug)
    pub mutants_killed: usize,
    /// Mutants that survived (bad - tests didn't catch the bug)
    pub mutants_survived: usize,
    /// Whether tests are considered valid (>80% kill rate)
    pub tests_valid: bool,
}

impl MutationResult {
    pub fn new(mutants_generated: usize, mutants_killed: usize, mutants_survived: usize) -> Self {
        let tests_valid = if mutants_generated > 0 {
            (mutants_killed as f64 / mutants_generated as f64) >= 0.8
        } else {
            false
        };

        Self {
            mutants_generated,
            mutants_killed,
            mutants_survived,
            tests_valid,
        }
    }

    pub fn kill_rate(&self) -> f64 {
        if self.mutants_generated == 0 {
            0.0
        } else {
            self.mutants_killed as f64 / self.mutants_generated as f64
        }
    }
}
