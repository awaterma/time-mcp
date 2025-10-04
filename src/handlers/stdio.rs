use crate::handlers::mcp::McpHandlers;
use crate::models::{McpResponse, McpError};
use anyhow::Result;
use serde_json::Value;
use std::io::{self, BufRead, Write};

pub struct StdioHandler;

impl StdioHandler {
    pub async fn run() -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line?;
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
                                    stdout.write_all(response_json.as_bytes())?;
                                    stdout.write_all(b"\n")?;
                                    stdout.flush()?;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse request: {}", e);
                }
            }
        }

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
                let error_response = McpResponse::<()>::error(id, McpError::method_not_found("Method not found"));
                serde_json::to_value(error_response).unwrap_or_else(|_| serde_json::json!({}))
            }
        }
    }
}