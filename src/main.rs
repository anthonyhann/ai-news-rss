mod browser;
mod config;
mod dedup;
mod llm;
mod models;
mod notify;
mod pipeline;
mod server;
mod sources;
mod store;

use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use config::{AppConfig, command_exists};
use pipeline::Pipeline;
use serde_json::json;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "ainews", version, about = "Self-hosted AI intelligence radar")]
struct Cli {
    #[arg(long, default_value = "config/app.toml", global = true)]
    config: PathBuf,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run one collection + scoring pass, optionally generating briefs.
    Run {
        #[arg(long)]
        fixtures: bool,
        #[arg(long)]
        no_brief: bool,
        #[arg(long, default_value = "morning")]
        slot: String,
        #[arg(long)]
        only: Option<String>,
    },
    /// Collect sources and export the live feed without generating a brief.
    Collect {
        #[arg(long)]
        fixtures: bool,
        #[arg(long)]
        only: Option<String>,
    },
    /// Generate the morning/evening brief from the current state.
    Brief {
        #[arg(long, default_value = "morning")]
        slot: String,
        #[arg(long)]
        fixtures: bool,
    },
    /// Print configuration and dependency diagnostics.
    Doctor,
    /// Inspect the source catalog.
    Sources {
        #[arg(default_value = "list")]
        action: String,
        target: Option<String>,
    },
    /// Rebuild static public JSON from the current SQLite state.
    Export,
    /// Serve the React app and read-only API.
    Serve,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    init_tracing();
    if let Err(error) = run_cli().await {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

async fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let cfg = AppConfig::load(&cli.config)?;
    match cli.command.unwrap_or(Commands::Run {
        fixtures: false,
        no_brief: false,
        slot: "morning".into(),
        only: None,
    }) {
        Commands::Run {
            fixtures,
            no_brief,
            slot,
            only,
        } => {
            let mut pipeline = Pipeline::new(cfg, fixtures).await?;
            pipeline.only_source = only;
            let run = pipeline.run(fixtures, !no_brief, &slot).await?;
            println!("{}", serde_json::to_string_pretty(&run)?);
        }
        Commands::Collect { fixtures, only } => {
            let mut pipeline = Pipeline::new(cfg, fixtures).await?;
            pipeline.only_source = only;
            let run = pipeline.run(fixtures, false, "collect").await?;
            println!("{}", serde_json::to_string_pretty(&run)?);
        }
        Commands::Brief { slot, fixtures } => {
            let pipeline = Pipeline::new(cfg, fixtures).await?;
            let run = if fixtures {
                pipeline.run(true, true, &slot).await?
            } else {
                pipeline.brief_only(&slot).await?
            };
            println!("{}", serde_json::to_string_pretty(&run)?);
        }
        Commands::Doctor => doctor(&cfg)?,
        Commands::Sources { action, target } => {
            sources_cmd(&cfg, &action, target.as_deref()).await?
        }
        Commands::Export => {
            let pipeline = Pipeline::new(cfg, true).await?;
            let run = crate::models::RunStats {
                id: "manual-export".into(),
                started_at: Utc::now(),
                finished_at: Some(Utc::now()),
                status: "success".into(),
                ..Default::default()
            };
            pipeline.export(&run).await?;
            println!("exported to {}", pipeline.cfg.public_dir().display());
        }
        Commands::Serve => server::serve(cfg).await?,
    }
    Ok(())
}

fn doctor(cfg: &AppConfig) -> Result<()> {
    let specs = cfg.load_sources()?;
    let enabled = specs.iter().filter(|s| s.enabled).count();
    let mut kinds = std::collections::BTreeMap::new();
    for spec in &specs {
        *kinds.entry(spec.kind.as_str()).or_insert(0usize) += 1;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "config": {
                "timezone": cfg.app.timezone,
                "data_dir": cfg.app.data_dir,
                "public_dir": cfg.app.public_dir,
            },
            "sources": {
                "total": specs.len(),
                "enabled": enabled,
                "by_kind": kinds,
            },
            "llm": {
                "configured": cfg.llm_available(),
                "base_url": cfg.ai.base_url,
                "scoring_model": cfg.ai.scoring_model,
                "editor_model": cfg.ai.editor_model,
                "api_key_env": cfg.ai.api_key_env,
                "api_key_present": std::env::var(&cfg.ai.api_key_env).map(|v| !v.is_empty()).unwrap_or(false),
            },
            "browser": {
                "provider": cfg.browser_provider(),
                "playwright_worker": std::path::Path::new(&cfg.browser.worker_dir).join("playwright-x.mjs").exists(),
                "ego_browser": command_exists("ego-browser"),
                "node": command_exists("node"),
                "yt_dlp": command_exists("yt-dlp"),
            },
            "paths": {
                "db": cfg.db_path(),
                "public": cfg.public_dir(),
                "fixtures": cfg.app.fixture_file,
            }
        }))?
    );
    Ok(())
}

async fn sources_cmd(cfg: &AppConfig, action: &str, target: Option<&str>) -> Result<()> {
    let specs = cfg.load_sources()?;
    match action {
        "count" => {
            println!("{}", specs.len());
        }
        "check" => {
            let mut bad = Vec::new();
            for spec in &specs {
                let missing = match spec.kind {
                    models::SourceKind::HackerNews | models::SourceKind::OssInsight => false,
                    models::SourceKind::Reddit => spec.subreddit.is_none(),
                    models::SourceKind::Telegram => spec.channel.is_none(),
                    models::SourceKind::Github => spec.owner.is_none() || spec.repo.is_none(),
                    _ => spec.target_url().is_none(),
                };
                if spec.enabled && missing {
                    bad.push(spec.id.clone());
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "total": specs.len(),
                    "enabled": specs.iter().filter(|s| s.enabled).count(),
                    "missing_target": bad,
                }))?
            );
        }
        "reset" => {
            let id = target.ok_or_else(|| anyhow::anyhow!("sources reset requires a source id"))?;
            let store = store::Store::open(cfg.db_path()).await?;
            let changed = store.reset_source_failure(id).await?;
            println!("reset {id}: {changed} row(s)");
        }
        _ => {
            for spec in specs {
                println!(
                    "{}\t{}\t{}\tT{}",
                    spec.id,
                    spec.kind.as_str(),
                    if spec.enabled { "on" } else { "off" },
                    spec.tier
                );
            }
        }
    }
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("ainews=info,tower_http=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
