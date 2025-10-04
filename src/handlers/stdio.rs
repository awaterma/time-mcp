use crate::handlers::mcp::McpHandlers;
use crate::models::{McpError, McpResponse};
use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct StdioHandler;

impl StdioHandler {
    pub async fn run() -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    tracing::info!("EOF reached, shutting down");
                    break;
                }
                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<Value>(&line) {
                        Ok(message) => {
                            if let Some(method) = message.get("method").and_then(|v| v.as_str()) {
                                match method {
                                    "initialized" => {
                                        tracing::info!("Client initialized");
                                    }
                                    _ => {
                                        if message.get("id").is_some() {
                                            let response = Self::handle_request(message).await;
                                            let response_json = serde_json::to_string(&response)?;
                                            stdout.write_all(response_json.as_bytes()).await?;
                                            stdout.write_all(b"\n").await?;
                                            stdout.flush().await?;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to parse request: {} - Input: {}", e, line.trim());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        tracing::info!("STDIO handler shutting down");
        Ok(())
    }

    async fn handle_request(request: Value) -> Value {
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = request.get("id").cloned().unwrap_or(serde_json::json!(0));
        let params = request.get("params").cloned();

        match method {
            "initialize" => McpHandlers::handle_initialize(id, params).await,
            "tools/list" => McpHandlers::handle_tools_list(id).await,
            "tools/call" => McpHandlers::handle_tools_call(id, params).await,
            "resources/list" => McpHandlers::handle_resources_list(id).await,
            "resources/read" => McpHandlers::handle_resources_read(id, params).await,
            "prompts/list" => McpHandlers::handle_prompts_list(id).await,
            "prompts/get" => McpHandlers::handle_prompts_get(id, params).await,
            _ => {
                let error_response =
                    McpResponse::<()>::error(id, McpError::method_not_found("Method not found"));
                serde_json::to_value(error_response).unwrap_or_else(|_| serde_json::json!({}))
            }
        }
    }
}
