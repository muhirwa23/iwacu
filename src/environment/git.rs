use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;
use tracing::{info, debug, warn};

use crate::core::messages::SystemError;

/// Git operations for worktree management
pub struct GitTools {
    repo_path: PathBuf,
}

impl GitTools {
    pub fn new(repo_path: &str) -> Result<Self, SystemError> {
        let path = PathBuf::from(repo_path);
        
        // Verify it's a git repository
        if !path.join(".git").exists() {
            return Err(SystemError::Git(
                "Not a git repository".to_string()
            ));
        }

        Ok(Self { repo_path: path })
    }

    /// Create a new git worktree
    pub fn create_worktree(&self) -> Result<PathBuf, SystemError> {
        let worktree_name = format!("symbiote-{}", Uuid::new_v4());
        let worktree_path = std::env::temp_dir().join(&worktree_name);

        info!("Creating worktree at: {:?}", worktree_path);

        let output = Command::new("git")
            .args([
                "-C",
                self.repo_path.to_str().unwrap(),
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| SystemError::Git(format!("Failed to create worktree: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SystemError::Git(format!(
                "Git worktree creation failed: {}",
                stderr
            )));
        }

        debug!("Worktree created successfully");
        Ok(worktree_path)
    }

    /// Apply a unified diff patch to a worktree
    pub fn apply_patch(&self, worktree_path: &str, diff: &str) -> Result<(), SystemError> {
        let worktree = PathBuf::from(worktree_path);

        info!("Applying patch to worktree: {}", worktree_path);

        // Write diff to temp file
        let diff_path = std::env::temp_dir().join(format!("patch-{}.diff", Uuid::new_v4()));
        std::fs::write(&diff_path, diff).map_err(|e| {
            SystemError::Git(format!("Failed to write diff file: {}", e))
        })?;

        // Apply the patch
        let output = Command::new("git")
            .args(["apply", "--allow-empty"])
            .current_dir(&worktree)
            .stdin(std::fs::File::open(&diff_path).map_err(|e| {
                SystemError::Git(format!("Failed to open diff file: {}", e))
            })?)
            .output()
            .map_err(|e| SystemError::Git(format!("Failed to apply patch: {}", e)))?;

        // Cleanup temp file
        let _ = std::fs::remove_file(&diff_path);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SystemError::Git(format!(
                "Patch application failed: {}",
                stderr
            )));
        }

        debug!("Patch applied successfully");
        Ok(())
    }

    /// Remove a worktree
    pub fn cleanup_worktree(&self, worktree_path: &str) -> Result<(), SystemError> {
        let worktree = PathBuf::from(worktree_path);

        info!("Cleaning up worktree: {}", worktree_path);

        // First remove from git
        let output = Command::new("git")
            .args([
                "-C",
                self.repo_path.to_str().unwrap(),
                "worktree",
                "remove",
                worktree_path,
                "--force",
            ])
            .output();

        match output {
            Ok(out) => {
                if !out.status.success() {
                    warn!("Git worktree remove failed, trying manual cleanup");
                }
            }
            Err(e) => {
                warn!("Git command failed: {}, trying manual cleanup", e);
            }
        }

        // Manual cleanup as fallback
        if worktree.exists() {
            std::fs::remove_dir_all(&worktree).map_err(|e| {
                SystemError::Git(format!("Failed to remove worktree directory: {}", e))
            })?;
        }

        debug!("Worktree cleaned up");
        Ok(())
    }
}
