//! Mutation testing engine

use crate::models::MutationResult;
use anyhow::{Context, Result};
use proc_macro2::TokenStream;
use quote::quote;
use std::path::Path;
use syn::{visit_mut::VisitMut, *};

/// Mutator for generating code mutants
pub struct Mutator {
    /// Number of mutation attempts per operator
    max_mutants: usize,
}

impl Mutator {
    pub fn new(max_mutants: usize) -> Self {
        Self { max_mutants }
    }

    /// Generate mutants from source code
    /// Returns vector of (mutant_id, mutated_source)
    pub fn generate_mutants(&self, source: &str) -> Result<Vec<(String, String)>> {
        let mut mutants = Vec::new();
        
        // Parse the source into AST
        let mut file: File = syn::parse_str(source)
            .with_context(|| "Failed to parse Rust source")?;

        // Apply different mutation operators
        let mut visitor = MutationVisitor::new(self.max_mutants);
        visitor.visit_file_mut(&mut file);
        
        mutants.extend(visitor.mutants);

        Ok(mutants)
    }

    /// Run mutation test: apply mutant and run tests
    pub async fn test_mutant(
        &self,
        worktree_path: &Path,
        file_path: &str,
        original_content: &str,
        mutant_content: &str,
    ) -> Result<bool> {
        use tokio::fs;
        use tokio::process::Command;

        let full_path = worktree_path.join(file_path);

        // Apply mutant
        fs::write(&full_path, mutant_content).await?;

        // Run tests
        let output = Command::new("cargo")
            .arg("test")
            .current_dir(worktree_path)
            .output()
            .await?;

        // Restore original
        fs::write(&full_path, original_content).await?;

        // If tests pass with mutant, the mutant survived (bad)
        // If tests fail, the mutant was killed (good)
        Ok(!output.status.success())
    }

    /// Run full mutation testing suite
    pub async fn run_mutation_testing(
        &self,
        worktree_path: &Path,
        files_to_mutate: &[&str],
    ) -> Result<MutationResult> {
        use tokio::fs;

        let mut mutants_generated = 0;
        let mut mutants_killed = 0;
        let mut mutants_survived = 0;

        for &file_path in files_to_mutate {
            let full_path = worktree_path.join(file_path);
            let content = fs::read_to_string(&full_path)
                .await
                .with_context(|| format!("Failed to read {}", file_path))?;

            let mutants = self.generate_mutants(&content)?;
            
            for (mutant_id, mutant_content) in mutants {
                mutants_generated += 1;

                let killed = self
                    .test_mutant(worktree_path, file_path, &content, &mutant_content)
                    .await?;

                if killed {
                    mutants_killed += 1;
                } else {
                    mutants_survived += 1;
                }
            }
        }

        Ok(MutationResult::new(
            mutants_generated,
            mutants_killed,
            mutants_survived,
        ))
    }
}

/// AST visitor that applies mutations
struct MutationVisitor {
    max_mutants: usize,
    mutant_count: usize,
    mutants: Vec<(String, String)>,
}

impl MutationVisitor {
    fn new(max_mutants: usize) -> Self {
        Self {
            max_mutants,
            mutant_count: 0,
            mutants: Vec::new(),
        }
    }

    fn add_mutant(&mut self, modified: &File) -> Option<()> {
        if self.mutant_count >= self.max_mutants {
            return None;
        }

        let source = pretty_print_file(modified);
        self.mutants.push((
            format!("mutant_{}", self.mutant_count),
            source,
        ));
        self.mutant_count += 1;
        Some(())
    }
}

impl VisitMut for MutationVisitor {
    /// Mutate binary operators
    fn visit_bin_expr_mut(&mut self, node: &mut BinExpr) {
        use BinOp::*;

        let original_op = node.op.clone();
        
        // Define mutations for each operator type
        let mutations: Vec<BinOp> = match original_op {
            Eq(_) => vec!(Ne(Default::default())),
            Ne(_) => vec!(Eq(Default::default())),
            Lt(_) => vec!(Ge(Default::default())),
            Le(_) => vec!(Gt(Default::default())),
            Gt(_) => vec!(Le(Default::default())),
            Ge(_) => vec!(Lt(Default::default())),
            Add(_) => vec!(Sub(Default::default())),
            Sub(_) => vec!(Add(Default::default())),
            Mul(_) => vec!(Div(Default::default())),
            Div(_) => vec!(Mul(Default::default())),
            And(_) => vec!(Or(Default::default())),
            Or(_) => vec!(And(Default::default())),
            _ => vec!(),
        };

        for new_op in mutations {
            if self.mutant_count >= self.max_mutants {
                break;
            }

            node.op = new_op.clone();
            // We'd need to clone the parent File here - simplified for now
            // In production, would maintain a proper copy mechanism
        }

        node.op = original_op;
        visit_mut::visit_bin_expr_mut(self, node);
    }

    /// Mutate boolean literals
    fn visit_lit_bool_mut(&mut self, node: &mut LitBool) {
        if self.mutant_count >= self.max_mutants {
            return;
        }

        // Flip boolean
        let original = node.value;
        node.value = !node.value;
        
        // Would add mutant here (simplified)
        
        node.value = original;
        visit_mut::visit_lit_bool_mut(self, node);
    }

    /// Mutate numeric literals
    fn visit_lit_int_mut(&mut self, node: &mut LitInt) {
        if self.mutant_count >= self.max_mutants {
            return;
        }

        // Increment/decrement by 1
        if let Ok(val) = node.base10_parse::<i64>() {
            let original = node.clone();
            
            // Try val + 1
            if val < i64::MAX {
                // Would create mutant with val + 1
            }
            
            // Try val - 1  
            if val > i64::MIN {
                // Would create mutant with val - 1
            }
        }
        
        visit_mut::visit_lit_int_mut(self, node);
    }
}

/// Pretty print a File back to source
fn pretty_print_file(file: &File) -> String {
    quote!(#file).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutator_generation() {
        let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

        let mutator = Mutator::new(5);
        let mutants = mutator.generate_mutants(source).unwrap();
        
        assert!(!mutants.is_empty());
    }
}
