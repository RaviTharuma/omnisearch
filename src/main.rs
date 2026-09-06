//! omnisearch binary: stdio MCP (default), HTTP transport, or provider bench.

use std::sync::Arc;

use clap::{Parser, Subcommand};
use omnisearch::orchestrator::AppState;
use omnisearch::tools::OmniServer;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "omnisearch",
    version,
    about = "Unified multi-provider MCP search server"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Serve MCP over stdin/stdout (default).
    Stdio,
    /// Serve MCP over streamable HTTP.
    Http,
    /// Benchmark configured providers.
    Bench {
        #[arg(long, default_value = "omnisearch rust mcp")]
        query: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env().add_directive("omnisearch=info".parse()?))
        .init();

    let cli = Cli::parse();
    let state = AppState::from_env()?;

    match cli.command.unwrap_or(Command::Stdio) {
        Command::Stdio => serve_stdio(state).await,
        Command::Http => omnisearch::http_server::serve_http(state).await,
        Command::Bench { query } => {
            let rows = omnisearch::bench::run_bench(&state, &query).await;
            println!("{}", serde_json::to_string_pretty(&rows)?);
            Ok(())
        }
    }
}

async fn serve_stdio(state: Arc<AppState>) -> anyhow::Result<()> {
    let server = OmniServer::new(state);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
