use clap::{Arg, Command};
use std::io;
use tokio::net::TcpListener;
use tracing::{info, error};

mod mcp;
mod time_tools;
mod transports;

use mcp::McpServer;
use transports::{stdio::StdioTransport, http::HttpTransport};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

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
            Arg::new("port")
                .long("port")
                .value_name("PORT")
                .help("Port for HTTP transport")
                .default_value("8080")
        )
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("Host for HTTP transport")
                .default_value("localhost")
        )
        .get_matches();

    let transport_type = matches.get_one::<String>("transport").unwrap();
    
    match transport_type.as_str() {
        "stdio" => {
            info!("Starting Time MCP Server with STDIO transport");
            let transport = StdioTransport::new(io::stdin(), io::stdout());
            let server = McpServer::new(transport);
            server.run().await?;
        }
        "http" => {
            let host = matches.get_one::<String>("host").unwrap();
            let port = matches.get_one::<String>("port").unwrap();
            let addr = format!("{}:{}", host, port);
            
            info!("Starting Time MCP Server with HTTP transport on {}", addr);
            let listener = TcpListener::bind(&addr).await?;
            let transport = HttpTransport::new(listener);
            let server = McpServer::new(transport);
            server.run().await?;
        }
        _ => {
            error!("Invalid transport type: {}", transport_type);
            std::process::exit(1);
        }
    }

    Ok(())
}