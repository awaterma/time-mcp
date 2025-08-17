use clap::{Arg, Command};
use serde_json::{json, Value};
use chrono::{DateTime, Utc, Offset, TimeZone};
use chrono_tz::{Tz, TZ_VARIANTS};
use std::io::{self, BufRead, Write};
use anyhow::Result;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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

    let transport = _matches.get_one::<String>("transport").unwrap();
    let default_host = "localhost".to_string();
    let host = _matches.get_one::<String>("host").unwrap_or(&default_host);
    let port = _matches.get_one::<String>("port")
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);

    match transport.as_str() {
        "stdio" => {
            tracing::info!("Starting Time MCP Server with STDIO transport");
            run_stdio_server().await?
        }
        "http" => {
            tracing::info!("Starting Time MCP Server with HTTP transport on {}:{}", host, port);
            run_http_server(host, port).await?
        }
        _ => {
            tracing::error!("Invalid transport type: {}", transport);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_stdio_server() -> Result<()> {
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

#[derive(Clone)]
struct AppState {
    auth_enabled: bool,
    valid_tokens: Arc<std::sync::RwLock<HashMap<String, TokenInfo>>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct TokenInfo {
    user_id: String,
    scopes: Vec<String>,
    expires_at: SystemTime,
}


#[derive(Deserialize)]
struct McpRequest {
    name: Option<String>,
    arguments: Option<Value>,
    uri: Option<String>,
}

async fn run_http_server(host: &str, port: u16) -> Result<()> {
    let state = AppState {
        auth_enabled: std::env::var("OAUTH_ENABLED").unwrap_or_default() == "true",
        valid_tokens: Arc::new(std::sync::RwLock::new(HashMap::new())),
    };


    let app = Router::new()
        .route("/mcp/capabilities", get(get_capabilities))
        .route("/mcp/tools/call", post(call_tool))
        .route("/mcp/resources/read", post(read_resource))
        .route("/mcp/prompts/get", post(get_prompt))
        .route("/health", get(health_check))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("HTTP server listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn authenticate_request(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    if !state.auth_enabled {
        return Ok(());
    }

    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];
    let tokens = state.valid_tokens.read().unwrap();
    
    match tokens.get(token) {
        Some(token_info) => {
            if token_info.expires_at > SystemTime::now() {
                Ok(())
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        None => Err(StatusCode::UNAUTHORIZED)
    }
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339(),
        "version": "1.0.0"
    }))
}

async fn get_capabilities(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    authenticate_request(&headers, &state).await?;
    
    Ok(Json(json!({
        "protocolVersion": "2025-03-26",
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
    })))
}

async fn call_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<McpRequest>
) -> Result<Json<Value>, StatusCode> {
    authenticate_request(&headers, &state).await?;

    let tool_name = request.name.ok_or(StatusCode::BAD_REQUEST)?;
    let arguments = request.arguments.unwrap_or(Value::Null);

    let result = match tool_name.as_str() {
        "get_current_time" => get_current_time(arguments).await,
        "convert_timezone" => convert_timezone(arguments).await,
        "calculate_duration" => calculate_duration(arguments).await,
        "format_time" => format_time(arguments).await,
        "get_timezone_info" => get_timezone_info(arguments).await,
        "list_timezones" => list_timezones(arguments).await,
        _ => return Err(StatusCode::NOT_FOUND),
    };

    match result {
        Ok(content) => Ok(Json(json!({
            "content": [{
                "type": "text",
                "text": content
            }]
        }))),
        Err(e) => {
            tracing::error!("Tool execution error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn read_resource(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<McpRequest>
) -> Result<Json<Value>, StatusCode> {
    authenticate_request(&headers, &state).await?;

    let uri = request.uri.ok_or(StatusCode::BAD_REQUEST)?;

    let content = match uri.as_str() {
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
        "time_formats" => {
            json!({
                "iso8601_examples": [
                    "2025-08-17T18:30:00Z",
                    "2025-08-17T18:30:00.123Z",
                    "2025-08-17T14:30:00-04:00"
                ],
                "rfc3339_examples": [
                    "2025-08-17T18:30:00Z",
                    "2025-08-17T18:30:00.123456Z"
                ],
                "custom_format_strings": {
                    "examples": [
                        "%Y-%m-%d %H:%M:%S",
                        "%B %d, %Y at %I:%M %p",
                        "%A, %b %d, %Y"
                    ],
                    "documentation": "Uses strftime format codes. Common codes: %Y=year, %m=month, %d=day, %H=hour(24h), %I=hour(12h), %M=minute, %S=second, %Z=timezone"
                },
                "locale_specific_formats": {
                    "us": "%m/%d/%Y %I:%M %p",
                    "eu": "%d/%m/%Y %H:%M",
                    "iso": "%Y-%m-%d %H:%M:%S"
                }
            }).to_string()
        }
        _ => return Err(StatusCode::NOT_FOUND),
    };

    Ok(Json(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": content
        }]
    })))
}

async fn get_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<McpRequest>
) -> Result<Json<Value>, StatusCode> {
    authenticate_request(&headers, &state).await?;

    let prompt_name = request.name.ok_or(StatusCode::BAD_REQUEST)?;

    match prompt_name.as_str() {
        "time_query_assistant" => {
            let user_query = request.arguments
                .as_ref()
                .and_then(|args| args.get("user_query"))
                .and_then(|v| v.as_str())
                .unwrap_or("general time query");

            let current_time = Utc::now();
            
            Ok(Json(json!({
                "description": "Assistant for time-related queries",
                "messages": [{
                    "role": "system",
                    "content": {
                        "type": "text",
                        "text": format!(
                            "You are a time query assistant. Help the user with their time-related question: '{}'. Current UTC time: {}. You have access to comprehensive timezone conversion and time formatting tools.",
                            user_query,
                            current_time.to_rfc3339()
                        )
                    }
                }]
            })))
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
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
                            },
                            "custom_format": {
                                "type": "string",
                                "description": "Custom strftime format string"
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
                    "name": "calculate_duration",
                    "description": "Calculate time differences between two timestamps",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "start_time": {
                                "type": "string",
                                "description": "Start timestamp"
                            },
                            "end_time": {
                                "type": "string",
                                "description": "End timestamp"
                            },
                            "units": {
                                "type": "string",
                                "enum": ["seconds", "minutes", "hours", "days"],
                                "description": "Output units",
                                "default": "seconds"
                            }
                        },
                        "required": ["start_time", "end_time"]
                    }
                },
                {
                    "name": "format_time",
                    "description": "Format timestamps according to various standards",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "timestamp": {
                                "type": "string",
                                "description": "Input timestamp"
                            },
                            "format": {
                                "type": "string",
                                "enum": ["iso8601", "rfc3339", "unix", "custom"],
                                "description": "Format type"
                            },
                            "custom_format": {
                                "type": "string",
                                "description": "Custom format string"
                            },
                            "timezone": {
                                "type": "string",
                                "description": "Target timezone"
                            }
                        },
                        "required": ["timestamp", "format"]
                    }
                },
                {
                    "name": "get_timezone_info",
                    "description": "Retrieve timezone information and current offset",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "timezone": {
                                "type": "string",
                                "description": "Timezone identifier"
                            }
                        },
                        "required": ["timezone"]
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
        "calculate_duration" => calculate_duration(arguments).await,
        "format_time" => format_time(arguments).await,
        "get_timezone_info" => get_timezone_info(arguments).await,
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
                },
                {
                    "uri": "time_formats",
                    "name": "Time Formats",
                    "description": "Documentation of supported time formats",
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
        "time_formats" => {
            json!({
                "iso8601_examples": [
                    "2025-08-17T18:30:00Z",
                    "2025-08-17T18:30:00.123Z",
                    "2025-08-17T14:30:00-04:00"
                ],
                "rfc3339_examples": [
                    "2025-08-17T18:30:00Z",
                    "2025-08-17T18:30:00.123456Z"
                ],
                "custom_format_strings": {
                    "examples": [
                        "%Y-%m-%d %H:%M:%S",
                        "%B %d, %Y at %I:%M %p",
                        "%A, %b %d, %Y"
                    ],
                    "documentation": "Uses strftime format codes. Common codes: %Y=year, %m=month, %d=day, %H=hour(24h), %I=hour(12h), %M=minute, %S=second, %Z=timezone"
                },
                "locale_specific_formats": {
                    "us": "%m/%d/%Y %I:%M %p",
                    "eu": "%d/%m/%Y %H:%M",
                    "iso": "%Y-%m-%d %H:%M:%S"
                }
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

async fn calculate_duration(arguments: Value) -> Result<String> {
    let start_str = arguments.get("start_time")
        .and_then(|v| v.as_str())
        .ok_or(anyhow::anyhow!("start_time required"))?;
    let end_str = arguments.get("end_time")
        .and_then(|v| v.as_str())
        .ok_or(anyhow::anyhow!("end_time required"))?;
    let units = arguments.get("units")
        .and_then(|v| v.as_str())
        .unwrap_or("seconds");

    let start_dt = parse_timestamp(start_str)?;
    let end_dt = parse_timestamp(end_str)?;
    
    let duration = end_dt.signed_duration_since(start_dt);
    let total_seconds = duration.num_seconds();
    
    let result = match units {
        "seconds" => json!({
            "duration": {
                "total_seconds": total_seconds,
                "hours": total_seconds / 3600,
                "minutes": total_seconds / 60,
                "human_readable": format!("{} seconds", total_seconds)
            }
        }),
        "minutes" => {
            let minutes = total_seconds / 60;
            json!({
                "duration": {
                    "total_seconds": total_seconds,
                    "minutes": minutes,
                    "hours": minutes / 60,
                    "human_readable": format!("{} minutes", minutes)
                }
            })
        },
        "hours" => {
            let hours = total_seconds / 3600;
            json!({
                "duration": {
                    "total_seconds": total_seconds,
                    "hours": hours,
                    "minutes": total_seconds / 60,
                    "human_readable": format!("{} hours", hours)
                }
            })
        },
        "days" => {
            let days = total_seconds / (24 * 3600);
            json!({
                "duration": {
                    "total_seconds": total_seconds,
                    "days": days,
                    "hours": total_seconds / 3600,
                    "human_readable": format!("{} days", days)
                }
            })
        },
        _ => return Err(anyhow::anyhow!("Invalid units: {}", units)),
    };

    Ok(result.to_string())
}

async fn format_time(arguments: Value) -> Result<String> {
    let timestamp_str = arguments.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or(anyhow::anyhow!("timestamp required"))?;
    let format = arguments.get("format")
        .and_then(|v| v.as_str())
        .ok_or(anyhow::anyhow!("format required"))?;
    let timezone_str = arguments.get("timezone")
        .and_then(|v| v.as_str())
        .unwrap_or("UTC");

    let dt = parse_timestamp(timestamp_str)?;
    let tz: Tz = timezone_str.parse()
        .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", timezone_str))?;
    let dt_tz = dt.with_timezone(&tz);

    let result = match format {
        "iso8601" => json!({
            "formatted": dt_tz.to_rfc3339(),
            "timezone": timezone_str
        }),
        "rfc3339" => json!({
            "formatted": dt_tz.to_rfc3339(),
            "timezone": timezone_str
        }),
        "unix" => json!({
            "formatted": dt.timestamp().to_string(),
            "timezone": timezone_str
        }),
        "custom" => {
            let custom_format = arguments.get("custom_format")
                .and_then(|v| v.as_str())
                .ok_or(anyhow::anyhow!("custom_format required when format is 'custom'"))?;
            json!({
                "formatted": dt_tz.format(custom_format).to_string(),
                "timezone": timezone_str
            })
        },
        _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
    };

    Ok(result.to_string())
}

async fn get_timezone_info(arguments: Value) -> Result<String> {
    let timezone_str = arguments.get("timezone")
        .and_then(|v| v.as_str())
        .ok_or(anyhow::anyhow!("timezone required"))?;

    let tz: Tz = timezone_str.parse()
        .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", timezone_str))?;
    
    let now = Utc::now().with_timezone(&tz);
    let offset = now.offset();
    
    let offset_seconds = offset.fix().local_minus_utc();
    let dst_active = offset_seconds != tz.offset_from_utc_datetime(&now.naive_utc()).fix().local_minus_utc();
    let abbreviation = format!("{}", now.format("%Z"));
    
    let offset_hours = offset_seconds / 3600;
    let offset_minutes = (offset_seconds % 3600) / 60;
    let offset_str = format!("{:+03}:{:02}", offset_hours, offset_minutes.abs());

    let result = json!({
        "timezone": timezone_str,
        "offset": offset_str,
        "dst_active": dst_active,
        "abbreviation": abbreviation
    });

    Ok(result.to_string())
}

fn parse_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>> {
    if let Ok(unix_timestamp) = timestamp_str.parse::<i64>() {
        DateTime::from_timestamp(unix_timestamp, 0)
            .ok_or(anyhow::anyhow!("Invalid Unix timestamp"))
    } else {
        DateTime::parse_from_rfc3339(timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_| anyhow::anyhow!("Invalid timestamp format"))
    }
}