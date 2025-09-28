mod auth;
mod config;
mod handlers;
mod models;
mod tools;

use anyhow::Result;
use clap::{Arg, Command};
use config::{ServerConfig, TransportType};
use handlers::{stdio::StdioHandler, http::HttpHandler};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let matches = Command::new("time-mcp-server")
        .version("1.0.0")
        .about("Model Context Protocol server for time-related functionality")
        .arg(
            Arg::new("transport")
                .long("transport")
                .value_name("TYPE")
                .help("Transport type: stdio or http")
                .default_value("stdio")
                .value_parser(["stdio", "http"])
        )
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("Host to bind HTTP server to")
                .default_value("localhost")
        )
        .arg(
            Arg::new("port")
                .long("port")
                .value_name("PORT")
                .help("Port to bind HTTP server to")
                .default_value("8080")
        )
        .get_matches();

    let config = ServerConfig::from_matches(&matches)?;

    match config.transport.clone() {
        TransportType::Stdio => {
            tracing::info!("Starting Time MCP Server with STDIO transport");
            StdioHandler::run().await
        }
        TransportType::Http { host, port } => {
            tracing::info!("Starting Time MCP Server with HTTP transport on {}:{}", host, port);
            HttpHandler::new(config).run(&host, port).await
        }
    }
}