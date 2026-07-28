use clap::Parser;
use std::path::PathBuf;
use miette::{IntoDiagnostic, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod core;
mod cognition;
mod environment;

use core::messages::TaskContext;
use core::graph::Supervisor;
use cognition::{CognitionActor, AnthropicProvider};
use cognition::provider::ModelRouter;
use environment::EnvironmentActor;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Parser, Debug)]
#[command(name = "agentic")]
#[command(about = "The Autonomous Symbiote v3.0 - AI-powered code engineering CLI")]
struct Args {
    /// The task to execute
    #[arg(required = true)]
    task: String,

    /// Path to the repository (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    repo: PathBuf,

    /// Anthropic API key (or set ANTHROPIC_API_KEY env var)
    #[arg(short, long, env = "ANTHROPIC_API_KEY")]
    api_key: Option<String>,

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

    // Get API key
    let api_key = args.api_key.ok_or_else(|| {
        miette::miette!(
            "Anthropic API key required. Pass --api-key or set ANTHROPIC_API_KEY environment variable"
        )
    })?;

    println!("🧬 Autonomous Symbiote v3.0");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Task: {}", args.task);
    println!("Budget: ${:.2}", args.max_cost);
    println!();

    // Resolve repo path
    let repo_path = args.repo.canonicalize().into_diagnostic()?;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    // Initialize multi-model router with budget tracking
    let mut router = ModelRouter::new(args.max_cost);
    
    // Create providers for different models
    let sonnet_provider = Arc::new(
        AnthropicProvider::new(api_key.clone())
            .with_model("claude-sonnet-4-20250514".to_string())
    );
    
    let haiku_provider = Arc::new(
        AnthropicProvider::new(api_key.clone())
            .with_model("claude-3-haiku-20240307".to_string())
    );
    
    // Register providers with the router
    router.add_provider("claude-sonnet".to_string(), sonnet_provider);
    router.add_provider("claude-haiku".to_string(), haiku_provider);
    router.add_provider("claude-sonnet-4-20250514".to_string(), 
        Arc::new(AnthropicProvider::new(api_key.clone()).with_model("claude-sonnet-4-20250514".to_string()))
    );
    
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

    // Create Supervisor and run task
    let supervisor = Supervisor::new(cognition_tx, environment_tx);
    let context = TaskContext::new(args.task);

    match supervisor.run_task(context).await {
        Ok(_) => {
            println!("\n✅ Task completed successfully!");
            println!("💰 Total cost: ${:.4}", router.budget().spent());
        }
        Err(e) => {
            eprintln!("\n❌ Task failed: {}", e);
            println!("💰 Cost so far: ${:.4}", router.budget().spent());
            return Err(miette::miette!("{}", e));
        }
    }

    // Cleanup: drop senders to signal actors to stop
    drop(supervisor);
    
    // Wait for actors to finish
    let _ = tokio::join!(env_handle, cog_handle);

    Ok(())
}
