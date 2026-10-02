//! Serve the Terraform-generated demo page on loopback only.
use axum::{Router, extract::State, http::StatusCode, response::Html, routing::get};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "example/local-app/site")]
    directory: PathBuf,
    #[arg(long, default_value_t = 0)]
    port: u16,
}

async fn page(State(directory): State<PathBuf>) -> Result<Html<String>, StatusCode> {
    tokio::fs::read_to_string(directory.join("index.html"))
        .await
        .map(Html)
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let directory = args.directory.canonicalize()?;
    anyhow::ensure!(directory.is_dir(), "Serve directory must be a directory");
    let listener =
        tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, args.port)).await?;
    println!("http://{}", listener.local_addr()?);
    let app = Router::new()
        .route("/", get(page))
        .route("/health", get(|| async { "ok" }))
        .with_state(directory);
    axum::serve(listener, app).await?;
    Ok(())
}
