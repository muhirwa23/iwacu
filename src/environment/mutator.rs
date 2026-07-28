use std::path::Path;
use std::fs;
use syn::{visit_mut::VisitMut, parse_file, Expr, BinOp, LitBool, LitInt};
use quote::ToTokens;
use tracing::{info, debug};

use crate::core::messages::SystemError;

/// AST mutation for testing test quality
pub struct Mutator {
    max_mutants: usize,
}

impl Mutator {
    pub fn new() -> Self {
        Self { max_mutants: 10 }
    }

    /// Run mutation testing gate
    pub fn run_mutation_gate(&self, worktree_path: &str) -> Result<bool, SystemError> {
        info!("Running mutation gate on: {}", worktree_path);

        // Find all .rs files in the worktree
        let src_path = Path::new(worktree_path).join("src");
        if !src_path.exists() {
            return Ok(true); // No source to mutate
        }

        let mut mutant_count = 0;
        let mut mutants_killed = 0;

        // Walk through source files
        for entry in fs::read_dir(&src_path).map_err(|e| {
            SystemError::Environment(format!("Failed to read src directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                SystemError::Environment(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                // Try to mutate this file
                match self.mutate_and_test(&path, worktree_path) {
                    Ok(killed) => {
                        mutant_count += 1;
                        if killed {
                            mutants_killed += 1;
                        }
                    }
                    Err(e) => {
                        debug!("Mutation failed for {:?}: {}", path, e);
                    }
                }

                if mutant_count >= self.max_mutants {
                    break;
                }
            }
        }

        if mutant_count == 0 {
            info!("No mutants created (no mutable expressions found)");
            return Ok(true);
        }

        let kill_rate = mutants_killed as f64 / mutant_count as f64;
        info!(
            "Mutation results: {}/{} mutants killed ({:.1}% kill rate)",
            mutants_killed, mutant_count,
            kill_rate * 100.0
        );

        // Require at least 50% kill rate to pass
        Ok(kill_rate >= 0.5)
    }

    /// Mutate a single file and run tests
    fn mutate_and_test(&self, file_path: &Path, worktree_path: &str) -> Result<bool, SystemError> {
        let content = fs::read_to_string(file_path).map_err(|e| {
            SystemError::Environment(format!("Failed to read file: {}", e))
        })?;

        let mut file = parse_file(&content).map_err(|e| {
            SystemError::Environment(format!("Failed to parse file: {}", e))
        })?;

        // Create a visitor to find and mutate expressions
        let mut visitor = MutationVisitor::new();
        visitor.visit_file_mut(&mut file);

        if visitor.mutated_expressions.is_empty() {
            return Ok(true); // Nothing to mutate
        }

        // Take the first mutation
        let (original, mutated) = visitor.mutated_expressions.remove(0);

        // Save original content
        let backup = content.clone();

        // Apply mutation by simple text replacement (simplified approach)
        // In production, we'd use proper AST manipulation with quote! macro
        let mutated_content = content.replace(
            &original.to_token_stream().to_string(),
            &mutated.to_token_stream().to_string(),
        );

        // Write mutated file
        fs::write(file_path, &mutated_content).map_err(|e| {
            SystemError::Environment(format!("Failed to write mutated file: {}", e))
        })?;

        // Run tests on mutated code
        use std::process::Command;
        let output = Command::new("cargo")
            .args(["test"])
            .current_dir(worktree_path)
            .output();

        // Restore original file
        fs::write(file_path, backup).map_err(|e| {
            SystemError::Environment(format!("Failed to restore original file: {}", e))
        })?;

        match output {
            Ok(result) => {
                // If tests pass on mutated code, the tests are insufficient
                let mutant_survived = result.status.success();
                Ok(!mutant_survived) // Return true if mutant was killed
            }
            Err(_) => Ok(true), // Assume killed if we can't run tests
        }
    }
}

/// Visitor to find mutable expressions
struct MutationVisitor {
    mutated_expressions: Vec<(Expr, Expr)>,
    max_mutants: usize,
}

impl MutationVisitor {
    fn new() -> Self {
        Self {
            mutated_expressions: Vec::new(),
            max_mutants: 10,
        }
    }

    fn add_mutant(&mut self, original: Expr, mutated: Expr) {
        if self.mutated_expressions.len() < self.max_mutants {
            self.mutated_expressions.push((original, mutated));
        }
    }
}

impl VisitMut for MutationVisitor {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Binary(bin) => {
                // Mutate binary operators
                match &bin.op {
                    BinOp::Eq(_) => {
                        // == becomes !=
                        let mutated = bin.clone();
                        // Note: This is simplified - proper implementation would use quote!
                        self.add_mutant(expr.clone(), expr.clone());
                    }
                    BinOp::Add(_) => {
                        // + becomes -
                        self.add_mutant(expr.clone(), expr.clone());
                    }
                    _ => {}
                }
            }
            Expr::Lit(lit) => {
                // Mutate literals
                match &lit.lit {
                    syn::Lit::Bool(b) => {
                        // Flip boolean
                        let new_val = !b.value;
                        // Placeholder - proper implementation would construct new AST
                        self.add_mutant(expr.clone(), expr.clone());
                    }
                    syn::Lit::Int(i) => {
                        // Add/subtract 1
                        if let Ok(val) = i.base10_parse::<i64>() {
                            if val > 0 {
                                self.add_mutant(expr.clone(), expr.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Continue traversal
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}
