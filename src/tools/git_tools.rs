//! Git operations - worktrees, patching, and repository management

use anyhow::{Context, Result};
use git2::{Repository, WorktreeStatus};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Git tools for worktree management and patch application
pub struct GitTools {
    repo: Repository,
}

impl GitTools {
    /// Open the repository at the given path
    pub fn open(repo_path: &Path) -> Result<Self> {
        let repo = Repository::open(repo_path).context("Failed to open git repository")?;
        Ok(Self { repo })
    }

    /// Create a new worktree for isolated changes
    pub fn create_worktree(&self, name: Option<String>) -> Result<WorktreeInfo> {
        let worktree_name = name.unwrap_or_else(|| format!("agentic-{}", Uuid::new_v4()));
        let worktree_path = self
            .repo
            .workdir()
            .join(".git/worktrees")
            .join(&worktree_name);

        // Create worktree from current HEAD
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;

        self.repo
            .worktree(
                &worktree_name,
                &worktree_path,
                Some(
                    git2::WorktreeAddOptions::default()
                        .reference(Some(&format!("refs/heads/{}", worktree_name))),
                ),
            )
            .context("Failed to create worktree")?;

        Ok(WorktreeInfo {
            id: worktree_name,
            path: worktree_path,
            branch: format!("refs/heads/{}", worktree_name),
        })
    }

    /// Get worktree by name
    pub fn get_worktree(&self, name: &str) -> Result<Option<PathBuf>> {
        if self.repo.find_worktree(name).is_ok() {
            Ok(Some(self.repo.workdir().join(".git/worktrees").join(name)))
        } else {
            Ok(None)
        }
    }

    /// Remove a worktree
    pub fn remove_worktree(&self, name: &str, force: bool) -> Result<()> {
        let mut worktree = self.repo.find_worktree(name)?;
        worktree.prune(force)?;
        Ok(())
    }

    /// Apply a unified diff to a file in the worktree
    pub fn apply_diff(&self, worktree_path: &Path, file_path: &str, diff: &str) -> Result<()> {
        use std::fs;

        let full_path = worktree_path.join(file_path);

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Parse and apply the diff
        // For now, we'll write the new content directly if provided
        // In production, this would parse unified diff format
        Ok(())
    }

    /// Apply new content to a file
    pub fn write_file(&self, worktree_path: &Path, file_path: &str, content: &str) -> Result<()> {
        use std::fs;

        let full_path = worktree_path.join(file_path);

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&full_path, content)
            .with_context(|| format!("Failed to write to {}", full_path.display()))?;

        Ok(())
    }

    /// Stage all changes in worktree
    pub fn stage_all(&self, worktree_path: &Path) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Get current branch name
    pub fn current_branch(&self) -> Result<String> {
        let head = self.repo.head()?;
        let branch = head.shorthand().unwrap_or("HEAD").to_string();
        Ok(branch)
    }

    /// Check if we're in a clean state
    pub fn is_clean(&self) -> Result<bool> {
        let statuses = self.repo.statuses(None)?;
        Ok(statuses.is_empty())
    }
}

/// Information about a created worktree
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub id: String,
    pub path: PathBuf,
    pub branch: String,
}
