use anyhow::Result;

pub const DEFAULT_PROTOCOL_VERSION: &str = "2025-03-26";
pub const FALLBACK_PROTOCOL_VERSION: &str = "2025-06-18";
pub const SERVER_NAME: &str = "time-mcp-server";
pub const SERVER_VERSION: &str = "1.0.0";

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub transport: TransportType,
    pub host: String,
    pub port: u16,
    pub auth_enabled: bool,
}

#[derive(Clone, Debug)]
pub enum TransportType {
    Stdio,
    Http { host: String, port: u16 },
}

impl ServerConfig {
    pub fn from_matches(matches: &clap::ArgMatches) -> Result<Self> {
        let transport_str = matches.get_one::<String>("transport")
            .ok_or_else(|| anyhow::anyhow!("Transport type required"))?;
        
        let host = matches.get_one::<String>("host")
            .cloned()
            .unwrap_or_else(|| "localhost".to_string());
        
        let port = matches.get_one::<String>("port")
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(8080);
        
        let transport = match transport_str.as_str() {
            "stdio" => TransportType::Stdio,
            "http" => TransportType::Http { 
                host: host.clone(), 
                port 
            },
            _ => return Err(anyhow::anyhow!("Invalid transport type: {}", transport_str)),
        };
        
        let auth_enabled = std::env::var("OAUTH_ENABLED")
            .map(|v| v == "true")
            .unwrap_or(false);
        
        Ok(ServerConfig {
            transport,
            host,
            port,
            auth_enabled,
        })
    }
}