// Intentional scaffolding retained for the full architecture (alternate LLM
// backend, budget/router helpers, LSP definition/reference lookups, mutation
// variants) that isn't wired into every code path yet.
#![allow(dead_code)]

use clap::Parser;
use std::path::PathBuf;
use miette::{IntoDiagnostic, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod core;
mod cognition;
mod environment;

use core::messages::TaskContext;
use core::graph::Supervisor;
use cognition::{CognitionActor, NvidiaProvider};
use cognition::provider::ModelRouter;
use environment::EnvironmentActor;
use std::sync::Arc;
use std::io::Write;
use tokio::sync::mpsc;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Parser, Debug)]
#[command(name = "agentic")]
#[command(about = "The Autonomous Symbiote v3.0 - AI-powered code engineering CLI")]
struct Args {
    /// The task to execute. If omitted, the CLI starts in interactive mode.
    task: Option<String>,

    /// Path to the repository (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    repo: PathBuf,

    /// NVIDIA API key (or set NVIDIA_API_KEY env var)
    #[arg(short, long, env = "NVIDIA_API_KEY")]
    api_key: Option<String>,

    /// Start an interactive REPL session (default when no task is given)
    #[arg(short, long)]
    interactive: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Maximum budget for LLM calls in USD (default: $1.00)
    #[arg(long, default_value = "1.0")]
    max_cost: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(log_level))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get the shared/fallback NVIDIA API key (optional if per-model keys are set).
    let shared_key = args.api_key;

    // Resolve a per-model API key: model-specific env var takes precedence,
    // otherwise fall back to the shared NVIDIA_API_KEY / --api-key value.
    fn model_key(env_var: &str, shared: &Option<String>) -> Result<String> {
        std::env::var(env_var)
            .ok()
            .filter(|k| !k.is_empty())
            .or_else(|| shared.clone())
            .ok_or_else(|| {
                miette::miette!(
                    "No NVIDIA API key for model. Set {} (per-model) or NVIDIA_API_KEY / --api-key (shared).",
                    env_var
                )
            })
    }

    let glm_key = model_key("NVIDIA_API_KEY_GLM", &shared_key)?;
    let inkling_key = model_key("NVIDIA_API_KEY_INKLING", &shared_key)?;
    let deepseek_key = model_key("NVIDIA_API_KEY_DEEPSEEK", &shared_key)?;
    let kimi_key = model_key("NVIDIA_API_KEY_KIMI", &shared_key)?;

    println!("🧬 Autonomous Symbiote v3.0");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Budget: ${:.2}", args.max_cost);
    println!();

    // Resolve repo path
    let repo_path = args.repo.canonicalize().into_diagnostic()?;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    // Initialize multi-model router with budget tracking
    let mut router = ModelRouter::new(args.max_cost);

    // Register NVIDIA-hosted models (OpenAI-compatible endpoint), each with its own key.
    // Router aliases: plan->deepseek, generate->glm, debug->kimi, synthesize->inkling.
    let glm = Arc::new(
        NvidiaProvider::new(glm_key)
            .with_model("z-ai/glm-5.2".to_string())
            .with_sampling(1.0, 1.0, 16384),
    );
    let inkling = Arc::new(
        NvidiaProvider::new(inkling_key)
            .with_model("thinkingmachines/inkling".to_string())
            .with_sampling(1.0, 0.95, 8192),
    );
    let deepseek = Arc::new(
        NvidiaProvider::new(deepseek_key)
            .with_model("deepseek-ai/deepseek-v4-pro".to_string())
            .with_sampling(1.0, 0.95, 16384)
            .with_thinking_disabled(true),
    );
    let kimi = Arc::new(
        NvidiaProvider::new(kimi_key)
            .with_model("moonshotai/kimi-k2.6".to_string())
            .with_sampling(1.0, 1.0, 16384),
    );

    // Register providers under router alias keys.
    router.add_provider("glm".to_string(), glm);
    router.add_provider("inkling".to_string(), inkling);
    router.add_provider("deepseek".to_string(), deepseek);
    router.add_provider("kimi".to_string(), kimi);

    let router = Arc::new(router);
    
    // Create channels for actor communication
    let (cognition_tx, cognition_rx) = mpsc::channel(32);
    let (environment_tx, environment_rx) = mpsc::channel(32);

    // Start Environment Actor
    let env_actor = EnvironmentActor::new(repo_path_str.clone())?;
    let env_handle = tokio::spawn(async move {
        env_actor.run(environment_rx).await
    });

    // Start Cognition Actor with model router
    let cog_actor = CognitionActor::new(Arc::clone(&router));
    let cog_handle = tokio::spawn(async move {
        cog_actor.run(cognition_rx).await
    });

    // Create Supervisor (reused across tasks in interactive mode).
    let supervisor = Supervisor::new(cognition_tx.clone(), environment_tx.clone());

    // Interactive when explicitly requested or when no task argument is given.
    let interactive = args.interactive || args.task.is_none();

    if interactive {
        println!("Interactive mode — type a task and press Enter. Type 'exit' or 'quit' to leave.");
        println!("Repo: {}\n", repo_path_str);

        let mut reader = BufReader::new(tokio::io::stdin());
        let mut line = String::new();

        loop {
            print!("rekon> ");
            std::io::stdout().flush().ok();

            line.clear();
            let n = reader.read_line(&mut line).await.into_diagnostic()?;
            if n == 0 {
                println!();
                break; // EOF (Ctrl-D)
            }

            let task = line.trim();
            if task.is_empty() {
                continue;
            }
            if task.eq_ignore_ascii_case("exit") || task.eq_ignore_ascii_case("quit") {
                break;
            }

            println!("\n▶ Running task: {}\n", task);
            let context = TaskContext::new(task.to_string());
            match supervisor.run_task(context).await {
                Ok(_) => {
                    println!("\n✅ Task completed successfully!");
                    println!("💰 Total cost so far: ${:.4}", router.budget().spent());
                }
                Err(e) => {
                    eprintln!("\n❌ Task failed: {}", e);
                    println!("💰 Cost so far: ${:.4}", router.budget().spent());
                }
            }
            println!();
        }

        println!("👋 Exiting interactive session.");
    } else {
        let task = args.task.clone().expect("task present in non-interactive mode");
        println!("Task: {}\n", task);
        let context = TaskContext::new(task);

        match supervisor.run_task(context).await {
            Ok(_) => {
                println!("\n✅ Task completed successfully!");
                println!("💰 Total cost: ${:.4}", router.budget().spent());
            }
            Err(e) => {
                eprintln!("\n❌ Task failed: {}", e);
                println!("💰 Cost so far: ${:.4}", router.budget().spent());
                drop(supervisor);
                drop(cognition_tx);
                drop(environment_tx);
                let _ = tokio::join!(env_handle, cog_handle);
                return Err(miette::miette!("{}", e));
            }
        }
    }

    // Cleanup: drop senders to signal actors to stop
    drop(supervisor);
    drop(cognition_tx);
    drop(environment_tx);

    // Wait for actors to finish
    let _ = tokio::join!(env_handle, cog_handle);

    Ok(())
}
