use std::path::PathBuf;
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

    /// Apply a unified diff patch to a worktree.
    ///
    /// LLM-generated diffs are frequently slightly malformed (markdown fences,
    /// prose, wrong hunk counts, missing context-line prefixes), so the diff is
    /// sanitized first and then applied with a cascade of increasingly tolerant
    /// strategies until one succeeds.
    pub fn apply_patch(&self, worktree_path: &str, diff: &str) -> Result<(), SystemError> {
        let worktree = PathBuf::from(worktree_path);

        info!("Applying patch to worktree: {}", worktree_path);

        let sanitized = Self::sanitize_diff(diff);

        // Write sanitized diff to temp file
        let diff_path = std::env::temp_dir().join(format!("patch-{}.diff", Uuid::new_v4()));
        std::fs::write(&diff_path, &sanitized).map_err(|e| {
            SystemError::Git(format!("Failed to write diff file: {}", e))
        })?;

        // Increasingly tolerant apply strategies.
        let strategies: &[&[&str]] = &[
            &["apply", "--allow-empty", "--whitespace=nowarn"],
            &["apply", "--allow-empty", "--recount", "--whitespace=fix"],
            &["apply", "--allow-empty", "--recount", "--ignore-whitespace", "--whitespace=fix"],
            &["apply", "--allow-empty", "--recount", "-p0"],
            &["apply", "--allow-empty", "--recount", "--3way"],
        ];

        let mut last_err = String::new();
        for (idx, args) in strategies.iter().enumerate() {
            let output = Command::new("git")
                .args(*args)
                .current_dir(&worktree)
                .stdin(std::fs::File::open(&diff_path).map_err(|e| {
                    SystemError::Git(format!("Failed to open diff file: {}", e))
                })?)
                .output()
                .map_err(|e| SystemError::Git(format!("Failed to apply patch: {}", e)))?;

            if output.status.success() {
                if idx > 0 {
                    info!("Patch applied using fallback strategy: git {}", args.join(" "));
                }
                let _ = std::fs::remove_file(&diff_path);
                debug!("Patch applied successfully");
                return Ok(());
            }

            last_err = String::from_utf8_lossy(&output.stderr).to_string();
            debug!("git {} failed: {}", args.join(" "), last_err.trim());
        }

        // Last resort: GNU patch with fuzzy matching (dry-run first so a partial
        // application never dirties the worktree).
        if let Some(path_str) = diff_path.to_str() {
            let fuzz_args = ["-p1", "--fuzz=3", "--batch", "--forward", "-i", path_str];
            let dry = Command::new("patch")
                .args(fuzz_args)
                .arg("--dry-run")
                .current_dir(&worktree)
                .output();
            if let Ok(dry) = dry {
                if dry.status.success() {
                    let real = Command::new("patch")
                        .args(fuzz_args)
                        .current_dir(&worktree)
                        .output();
                    if let Ok(real) = real {
                        if real.status.success() {
                            info!("Patch applied using fuzzy fallback: patch --fuzz=3");
                            let _ = std::fs::remove_file(&diff_path);
                            return Ok(());
                        }
                        last_err = String::from_utf8_lossy(&real.stderr).to_string();
                    }
                }
            }
        }

        // Cleanup temp file
        let _ = std::fs::remove_file(&diff_path);

        Err(SystemError::Git(format!(
            "Patch application failed: {}",
            last_err
        )))
    }

    /// Best-effort repair of an LLM-generated unified diff:
    /// - strips markdown code fences and leading prose
    /// - normalizes CRLF line endings
    /// - re-prefixes hunk body lines that lost their leading space
    /// - recounts hunk header line counts (`@@ -a,b +c,d @@`)
    fn sanitize_diff(diff: &str) -> String {
        let normalized = diff.replace("\r\n", "\n");
        let mut lines: Vec<String> = normalized
            .lines()
            .filter(|l| !l.trim_start().starts_with("```"))
            .map(|l| l.to_string())
            .collect();

        // Drop any prose before the first recognizable diff header.
        if let Some(start) = lines.iter().enumerate().position(|(i, l)| {
            l.starts_with("diff --git")
                || (l.starts_with("--- ")
                    && lines.get(i + 1).map(|n| n.starts_with("+++ ")).unwrap_or(false))
        }) {
            lines.drain(..start);
        }

        let mut out: Vec<String> = Vec::with_capacity(lines.len());
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].clone();
            if line.starts_with("@@") {
                let (old_start, new_start, suffix) =
                    Self::parse_hunk_header(&line).unwrap_or((1, 1, String::new()));

                // Collect the hunk body, repairing line prefixes as we go.
                let mut body: Vec<String> = Vec::new();
                let mut j = i + 1;
                while j < lines.len() {
                    let l = &lines[j];
                    let is_file_header = l.starts_with("--- ")
                        && lines.get(j + 1).map(|n| n.starts_with("+++ ")).unwrap_or(false);
                    if l.starts_with("@@") || l.starts_with("diff --git") || is_file_header {
                        break;
                    }
                    let fixed = if l.is_empty() {
                        " ".to_string()
                    } else if l.starts_with(' ')
                        || l.starts_with('+')
                        || l.starts_with('-')
                        || l.starts_with('\\')
                    {
                        l.clone()
                    } else {
                        // Context line that lost its leading space.
                        format!(" {}", l)
                    };
                    body.push(fixed);
                    j += 1;
                }

                let old_count = body
                    .iter()
                    .filter(|l| l.starts_with(' ') || l.starts_with('-'))
                    .count();
                let new_count = body
                    .iter()
                    .filter(|l| l.starts_with(' ') || l.starts_with('+'))
                    .count();

                out.push(format!(
                    "@@ -{},{} +{},{} @@{}",
                    old_start, old_count, new_start, new_count, suffix
                ));
                out.extend(body);
                i = j;
            } else {
                out.push(line);
                i += 1;
            }
        }

        let mut result = out.join("\n");
        result.push('\n');
        result
    }

    /// Parse `@@ -old[,cnt] +new[,cnt] @@ suffix` returning (old_start, new_start, suffix).
    fn parse_hunk_header(line: &str) -> Option<(usize, usize, String)> {
        let rest = line.strip_prefix("@@")?;
        let end = rest.find("@@")?;
        let (ranges, tail) = rest.split_at(end);
        let suffix = tail.trim_start_matches("@@").to_string();

        let mut old_start = 1usize;
        let mut new_start = 1usize;
        for part in ranges.split_whitespace() {
            if let Some(p) = part.strip_prefix('-') {
                old_start = p.split(',').next()?.parse().ok()?;
            } else if let Some(p) = part.strip_prefix('+') {
                new_start = p.split(',').next()?.parse().ok()?;
            }
        }
        Some((old_start, new_start, suffix))
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

#[cfg(test)]
mod tests {
    use super::GitTools;

    #[test]
    fn sanitize_strips_fences_and_prose_and_recounts_hunks() {
        // Malformed LLM output: prose preamble, markdown fences, wrong hunk
        // counts, and a context line missing its leading space.
        let raw = "Here is the patch you asked for:\n```diff\ndiff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,99 +1,99 @@\nfn main() {\n-    let x = 1;\n+    let x = 2;\n }\n```\n";

        let fixed = GitTools::sanitize_diff(raw);

        assert!(fixed.starts_with("diff --git"), "prose/fence not stripped: {}", fixed);
        assert!(!fixed.contains("```"), "fences not stripped");
        // 3 old lines (context+context+minus... actually: ' fn main()', '-let x=1', ' }') => old=3, new=3
        assert!(fixed.contains("@@ -1,3 +1,3 @@"), "hunk not recounted: {}", fixed);
        // The bare 'fn main() {' context line must have gained a leading space.
        assert!(fixed.contains("\n fn main() {\n"), "context prefix not repaired: {}", fixed);
    }

    #[test]
    fn sanitize_preserves_wellformed_diff() {
        let good = "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n context\n-old\n+new\n";
        let fixed = GitTools::sanitize_diff(good);
        assert_eq!(fixed, good);
    }

    #[test]
    fn parse_hunk_header_variants() {
        assert_eq!(
            GitTools::parse_hunk_header("@@ -10,5 +12,6 @@ fn foo()"),
            Some((10, 12, " fn foo()".to_string()))
        );
        assert_eq!(
            GitTools::parse_hunk_header("@@ -3 +3 @@"),
            Some((3, 3, String::new()))
        );
    }
}
