mod config;
mod core;
mod mcp;
mod shared;
mod terraform;

use clap::{arg, command, Parser, Subcommand};
use core::tfmcp::TfMcp;
use shared::logging;

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
                    Ok(mut tfmcp) => {
                        if let Err(err) = tfmcp.launch_mcp().await {
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
