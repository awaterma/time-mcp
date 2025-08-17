use clap::{Arg, Command};
use serde_json::{json, Value};
use chrono::{DateTime, Utc};
use chrono_tz::{Tz, TZ_VARIANTS};
use std::io::{self, BufRead, Write};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let _matches = Command::new("time-mcp-server")
        .version("1.0.0")
        .about("Model Context Protocol server for time-related functionality")
        .arg(
            Arg::new("transport")
                .long("transport")
                .value_name("TYPE")
                .help("Transport type: stdio")
                .default_value("stdio")
                .value_parser(["stdio"])
        )
        .get_matches();

    tracing::info!("Starting Time MCP Server with STDIO transport");

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
                                let response = handle_request(message).await;
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
    let id = request.get("id").cloned().unwrap_or(json!(0));
    let params = request.get("params").cloned();

    match method {
        "initialize" => handle_initialize(id, params).await,
        "tools/list" => handle_tools_list(id).await,
        "tools/call" => handle_tools_call(id, params).await,
        "resources/list" => handle_resources_list(id).await,
        "resources/read" => handle_resources_read(id, params).await,
        "prompts/list" => handle_prompts_list(id).await,
        "prompts/get" => handle_prompts_get(id, params).await,
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "Method not found"
            }
        })
    }
}

async fn handle_initialize(id: Value, params: Option<Value>) -> Value {
    let client_version = params
        .as_ref()
        .and_then(|p| p.get("protocolVersion"))
        .and_then(|v| v.as_str())
        .unwrap_or("2025-03-26");

    let protocol_version = if client_version == "2025-06-18" {
        "2025-06-18"
    } else {
        "2025-03-26"
    };

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": protocol_version,
            "capabilities": {
                "tools": {
                    "listChanged": false
                },
                "resources": {
                    "subscribe": false,
                    "listChanged": false
                },
                "prompts": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "time-mcp-server",
                "version": "1.0.0"
            }
        }
    })
}

async fn handle_tools_list(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "get_current_time",
                    "description": "Get the current time in various formats and timezones",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "timezone": {
                                "type": "string",
                                "description": "Target timezone (default: UTC)",
                                "default": "UTC"
                            },
                            "format": {
                                "type": "string",
                                "enum": ["iso", "unix", "human", "custom"],
                                "description": "Output format",
                                "default": "iso"
                            }
                        }
                    }
                },
                {
                    "name": "convert_timezone",
                    "description": "Convert time between different timezones",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "timestamp": {
                                "type": "string",
                                "description": "Input timestamp (ISO 8601 or Unix)"
                            },
                            "from_timezone": {
                                "type": "string",
                                "description": "Source timezone"
                            },
                            "to_timezone": {
                                "type": "string",
                                "description": "Target timezone"
                            }
                        },
                        "required": ["timestamp", "from_timezone", "to_timezone"]
                    }
                },
                {
                    "name": "list_timezones",
                    "description": "List available timezone identifiers",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "region": {
                                "type": "string",
                                "description": "Filter by region (e.g., 'America', 'Europe')"
                            }
                        }
                    }
                }
            ]
        }
    })
}

async fn handle_tools_call(id: Value, params: Option<Value>) -> Value {
    let params = match params {
        Some(p) => p,
        None => return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32602,
                "message": "Invalid params"
            }
        })
    };

    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32602,
                "message": "Missing tool name"
            }
        })
    };

    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

    let result = match name {
        "get_current_time" => get_current_time(arguments).await,
        "convert_timezone" => convert_timezone(arguments).await,
        "list_timezones" => list_timezones(arguments).await,
        _ => return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "Tool not found"
            }
        })
    };

    match result {
        Ok(content) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": content
                    }
                ]
            }
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32603,
                "message": e.to_string()
            }
        })
    }
}

async fn handle_resources_list(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "resources": [
                {
                    "uri": "timezone_database",
                    "name": "Timezone Database",
                    "description": "Complete IANA timezone database",
                    "mimeType": "application/json"
                }
            ]
        }
    })
}

async fn handle_resources_read(id: Value, params: Option<Value>) -> Value {
    let uri = match params.as_ref().and_then(|p| p.get("uri")).and_then(|v| v.as_str()) {
        Some(u) => u,
        None => return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32602,
                "message": "Missing URI"
            }
        })
    };

    let content = match uri {
        "timezone_database" => {
            let timezones: Vec<String> = TZ_VARIANTS
                .iter()
                .map(|tz| tz.name().to_string())
                .collect();
            json!({
                "timezones": timezones,
                "total_count": timezones.len()
            }).to_string()
        }
        _ => return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32602,
                "message": "Unknown resource"
            }
        })
    };

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "contents": [
                {
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }
            ]
        }
    })
}

async fn handle_prompts_list(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "prompts": [
                {
                    "name": "time_query_assistant",
                    "description": "Template for helping users with time-related queries",
                    "arguments": [
                        {
                            "name": "user_query",
                            "description": "The user's time-related question",
                            "required": true
                        }
                    ]
                }
            ]
        }
    })
}

async fn handle_prompts_get(id: Value, params: Option<Value>) -> Value {
    let name = match params.as_ref().and_then(|p| p.get("name")).and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32602,
                "message": "Missing prompt name"
            }
        })
    };

    match name {
        "time_query_assistant" => {
            let user_query = params
                .as_ref()
                .and_then(|p| p.get("arguments"))
                .and_then(|args| args.get("user_query"))
                .and_then(|v| v.as_str())
                .unwrap_or("general time query");

            let current_time = Utc::now();
            
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "description": "Assistant for time-related queries",
                    "messages": [
                        {
                            "role": "system",
                            "content": {
                                "type": "text",
                                "text": format!(
                                    "You are a time query assistant. Help the user with their time-related question: '{}'. Current UTC time: {}. You have access to comprehensive timezone conversion and time formatting tools.",
                                    user_query,
                                    current_time.to_rfc3339()
                                )
                            }
                        }
                    ]
                }
            })
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32602,
                "message": "Unknown prompt"
            }
        })
    }
}

async fn get_current_time(arguments: Value) -> Result<String> {
    let timezone = arguments.get("timezone")
        .and_then(|v| v.as_str())
        .unwrap_or("UTC");
    let format = arguments.get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("iso");

    let tz: Tz = timezone.parse()
        .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", timezone))?;
    
    let now_utc = Utc::now();
    let now_tz = now_utc.with_timezone(&tz);

    let result = match format {
        "iso" => json!({
            "timestamp": now_tz.to_rfc3339(),
            "unix": now_utc.timestamp(),
            "timezone": timezone,
            "formatted": now_tz.format("%A, %B %d, %Y at %I:%M %p %Z").to_string()
        }),
        "unix" => json!({
            "timestamp": now_utc.timestamp(),
            "timezone": timezone
        }),
        "human" => json!({
            "formatted": now_tz.format("%A, %B %d, %Y at %I:%M %p %Z").to_string(),
            "timezone": timezone
        }),
        "custom" => {
            let custom_format = arguments.get("custom_format")
                .and_then(|v| v.as_str())
                .ok_or(anyhow::anyhow!("custom_format required when format is 'custom'"))?;
            json!({
                "formatted": now_tz.format(custom_format).to_string(),
                "timezone": timezone
            })
        },
        _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
    };

    Ok(result.to_string())
}

async fn convert_timezone(arguments: Value) -> Result<String> {
    let timestamp_str = arguments.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or(anyhow::anyhow!("timestamp required"))?;
    let from_tz_str = arguments.get("from_timezone")
        .and_then(|v| v.as_str())
        .ok_or(anyhow::anyhow!("from_timezone required"))?;
    let to_tz_str = arguments.get("to_timezone")
        .and_then(|v| v.as_str())
        .ok_or(anyhow::anyhow!("to_timezone required"))?;

    let from_tz: Tz = from_tz_str.parse()
        .map_err(|_| anyhow::anyhow!("Invalid from_timezone: {}", from_tz_str))?;
    let to_tz: Tz = to_tz_str.parse()
        .map_err(|_| anyhow::anyhow!("Invalid to_timezone: {}", to_tz_str))?;

    let dt = if let Ok(unix_timestamp) = timestamp_str.parse::<i64>() {
        DateTime::from_timestamp(unix_timestamp, 0)
            .ok_or(anyhow::anyhow!("Invalid Unix timestamp"))?
            .with_timezone(&from_tz)
    } else {
        DateTime::parse_from_rfc3339(timestamp_str)
            .map_err(|_| anyhow::anyhow!("Invalid timestamp format"))?
            .with_timezone(&from_tz)
    };

    let converted = dt.with_timezone(&to_tz);

    let result = json!({
        "original": {
            "timestamp": dt.to_rfc3339(),
            "timezone": from_tz_str
        },
        "converted": {
            "timestamp": converted.to_rfc3339(),
            "timezone": to_tz_str
        }
    });

    Ok(result.to_string())
}

async fn list_timezones(arguments: Value) -> Result<String> {
    let region_filter = arguments.get("region")
        .and_then(|v| v.as_str());

    let timezones: Vec<String> = TZ_VARIANTS
        .iter()
        .map(|tz| tz.name().to_string())
        .filter(|name| {
            if let Some(region) = region_filter {
                name.starts_with(region)
            } else {
                true
            }
        })
        .collect();

    let result = json!({
        "timezones": timezones,
        "count": timezones.len()
    });

    Ok(result.to_string())
}