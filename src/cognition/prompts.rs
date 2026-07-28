//! Token-optimized prompt templates for the Cognition Actor.
//! All prompts are terse, schema-driven, and fit within <200 tokens of instruction.

/// Plan prompt: outputs constraints and micro-tasks.
pub fn plan_prompt(task: &str, mental_model: &str) -> (String, String) {
    let system = r#"You are a senior architect. Output JSON only.
Schema: {"constraints":[{"id":"...","required":bool,"options":[...],"default":null}],"micro_tasks":["step1","step2"]}.
Constraints: list decisions. If env can resolve (e.g., Cargo.toml has dep), set default. Else null (ask user).
Micro-tasks: atomic, ordered steps."#.to_string();

    let user = format!(
        r#"Task: {}
Mental Model: {}
LSP Context: {{lsp_context}}
Output JSON."#,
        task, mental_model
    );

    (system, user)
}

/// Generate prompt: outputs reasoning, self-critique, diff, and test_diff.
pub fn generate_prompt(task: &str, plan_step: &str, context: &str) -> (String, String) {
    let system = r#"You are a senior coder. Output JSON only.
Schema: {"reasoning":"≤3 sentences","self_critique":"≤3 sentences","diff":"unified diff","test_diff":"unified diff"}.
Reasoning: approach. Self-critique: potential issues. Diff: code change. Test-diff: unit test.
Ensure tests assert logic, not syntax."#.to_string();

    let user = format!(
        r#"Task: {}
Plan Step: {}
Context: {}
Output JSON."#,
        task, plan_step, context
    );

    (system, user)
}

/// Debug prompt: outputs root_cause and fix_diff from error graph.
pub fn debug_prompt(error_graph: &str, previous_diff: &str) -> (String, String) {
    let system = r#"You are a debugger. Output JSON only.
Schema: {"root_cause":"≤2 sentences","fix_diff":"unified diff"}.
Error graph is topologically sorted (root first). Identify root cause. Fix only that.
Do not rewrite unrelated code."#.to_string();

    let user = format!(
        r#"Error Graph: {}
Previous Diff: {}
Output JSON."#,
        error_graph, previous_diff
    );

    (system, user)
}

/// Synthesize prompt: outputs title, summary, files_changed.
pub fn synthesize_prompt(task: &str, final_diff: &str) -> (String, String) {
    let system = r#"You are a tech writer. Output JSON only.
Schema: {"title":"≤10 words","summary":"≤3 sentences","files_changed":["path1","path2"]}.
Summarize the change for a PR description."#.to_string();

    let user = format!(
        r#"Task: {}
Final Diff: {}
Output JSON."#,
        task, final_diff
    );

    (system, user)
}
