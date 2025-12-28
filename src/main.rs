mod config;
mod core;
mod formatters;
mod mcp;
mod prompts;
mod registry;
mod shared;
mod terraform;

use clap::{Parser, Subcommand};
use core::tfmcp::TfMcp;
use mcp::server::TfMcpServer;
use shared::logging;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    name = "tfmcp",
    about = "✨ A CLI tool to manage Terraform configurations and operate Terraform through the Model Context Protocol (MCP).",
    version = APP_VERSION,
    disable_version_flag(true)
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(
        long,
        short = 'c',
        value_name = "PATH",
        help = "Path to the configuration file"
    )]
    pub config: Option<String>,

    #[arg(
        long,
        short = 'd',
        value_name = "PATH",
        help = "Terraform project directory"
    )]
    pub dir: Option<String>,

    #[arg(long, short = 'V', help = "Print version")]
    pub version: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(name = "mcp", about = "Launch tfmcp as an MCP server")]
    Mcp,

    #[command(name = "analyze", about = "Analyze Terraform configurations")]
    Analyze,
}

#[tokio::main]
async fn main() {
    // Initialize tracing/logging
    init_logging();

    let cli = Cli::parse();

    if cli.version {
        println!("{}", APP_VERSION);
        std::process::exit(0);
    }

    match &cli.command {
        Some(cmd) => match cmd {
            Commands::Mcp => {
                logging::info("Starting tfmcp in MCP server mode");
                match init_tfmcp(&cli).await {
                    Ok(tfmcp) => {
                        if let Err(err) = TfMcpServer::serve_stdio(tfmcp).await {
                            logging::error(&format!("Error launching MCP server: {:?}", err));
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        logging::error(&format!("Failed to initialize tfmcp: {}", e));
                        std::process::exit(1);
                    }
                }
            }
            Commands::Analyze => {
                logging::info("Starting Terraform configuration analysis");
                match init_tfmcp(&cli).await {
                    Ok(mut tfmcp) => {
                        if let Err(err) = tfmcp.analyze_terraform().await {
                            logging::error(&format!("Error analyzing Terraform: {:?}", err));
                            std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        logging::error(&format!("Failed to initialize tfmcp: {}", e));
                        std::process::exit(1);
                    }
                }
            }
        },
        None => {
            // Default behavior if no command is specified
            println!("No command specified. Use --help for usage information.");
        }
    };
}

async fn init_tfmcp(cli: &Cli) -> anyhow::Result<TfMcp> {
    let config_path = cli.config.clone();
    let dir_path = cli.dir.clone();

    logging::info(&format!(
        "Initializing tfmcp with config: {:?}, dir: {:?}",
        config_path, dir_path
    ));
    TfMcp::new(config_path, dir_path)
}

fn init_logging() {
    let log_level = std::env::var("TFMCP_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .to_lowercase();

    let filter = match log_level.as_str() {
        "trace" => "trace",
        "debug" => "debug",
        "info" => "info",
        "warn" | "warning" => "warn",
        "error" => "error",
        _ => "info",
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("tfmcp={},reqwest=warn,hyper=warn", filter).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
