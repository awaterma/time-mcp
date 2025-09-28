use crate::{
    config::{DEFAULT_PROTOCOL_VERSION, FALLBACK_PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION},
    models::{McpResponse, McpError},
    tools::TimeTools,
};
use chrono::Utc;
use chrono_tz::TZ_VARIANTS;
use serde_json::{json, Value};

pub struct McpHandlers;

impl McpHandlers {
    pub async fn handle_initialize(id: Value, params: Option<Value>) -> Value {
        let client_version = params
            .as_ref()
            .and_then(|p| p.get("protocolVersion"))
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_PROTOCOL_VERSION);

        let protocol_version = if client_version == FALLBACK_PROTOCOL_VERSION {
            FALLBACK_PROTOCOL_VERSION
        } else {
            DEFAULT_PROTOCOL_VERSION
        };

        let response = McpResponse::success(
            id,
            json!({
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
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            })
        );

        serde_json::to_value(response).unwrap_or_else(|_| json!({}))
    }

    pub async fn handle_tools_list(id: Value) -> Value {
        let response = McpResponse::success(
            id,
            json!({
                "tools": Self::get_tool_definitions()
            })
        );

        serde_json::to_value(response).unwrap_or_else(|_| json!({}))
    }

    pub async fn handle_tools_call(id: Value, params: Option<Value>) -> Value {
        let params = match params {
            Some(p) => p,
            None => {
                return serde_json::to_value(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Missing params"
                    }
                })).unwrap_or_else(|_| json!({}));
            }
        };

        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => {
                return serde_json::to_value(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Missing tool name"
                    }
                })).unwrap_or_else(|_| json!({}));
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

        let result = Self::execute_tool(name, arguments).await;

        match result {
            Ok(content) => {
                let response = McpResponse::success(
                    id,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": content
                        }]
                    })
                );
                serde_json::to_value(response).unwrap_or_else(|_| json!({}))
            }
            Err(e) => {
                serde_json::to_value(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": e.to_string()
                    }
                })).unwrap_or_else(|_| json!({}))
            }
        }
    }

    pub async fn handle_resources_list(id: Value) -> Value {
        let response = McpResponse::success(
            id,
            json!({
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
            })
        );

        serde_json::to_value(response).unwrap_or_else(|_| json!({}))
    }

    pub async fn handle_resources_read(id: Value, params: Option<Value>) -> Value {
        let uri = match params.as_ref().and_then(|p| p.get("uri")).and_then(|v| v.as_str()) {
            Some(u) => u,
            None => {
                return serde_json::to_value(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Missing URI"
                    }
                })).unwrap_or_else(|_| json!({}));
            }
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
                Self::get_time_formats_resource().to_string()
            }
            _ => {
                return serde_json::to_value(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Unknown resource"
                    }
                })).unwrap_or_else(|_| json!({}));
            }
        };

        let response = McpResponse::success(
            id,
            json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": content
                }]
            })
        );

        serde_json::to_value(response).unwrap_or_else(|_| json!({}))
    }

    pub async fn handle_prompts_list(id: Value) -> Value {
        let response = McpResponse::success(
            id,
            json!({
                "prompts": [{
                    "name": "time_query_assistant",
                    "description": "Template for helping users with time-related queries",
                    "arguments": [{
                        "name": "user_query",
                        "description": "The user's time-related question",
                        "required": true
                    }]
                }]
            })
        );

        serde_json::to_value(response).unwrap_or_else(|_| json!({}))
    }

    pub async fn handle_prompts_get(id: Value, params: Option<Value>) -> Value {
        let name = match params.as_ref().and_then(|p| p.get("name")).and_then(|v| v.as_str()) {
            Some(n) => n,
            None => {
                return serde_json::to_value(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Missing prompt name"
                    }
                })).unwrap_or_else(|_| json!({}));
            }
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
                
                let response = McpResponse::success(
                    id,
                    json!({
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
                    })
                );

                serde_json::to_value(response).unwrap_or_else(|_| json!({}))
            }
            _ => {
                serde_json::to_value(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": "Unknown prompt"
                    }
                })).unwrap_or_else(|_| json!({}))
            }
        }
    }

    async fn execute_tool(name: &str, arguments: Value) -> anyhow::Result<String> {
        match name {
            "get_current_time" => TimeTools::get_current_time(arguments).await,
            "convert_timezone" => TimeTools::convert_timezone(arguments).await,
            "calculate_duration" => TimeTools::calculate_duration(arguments).await,
            "format_time" => TimeTools::format_time(arguments).await,
            "get_timezone_info" => TimeTools::get_timezone_info(arguments).await,
            "list_timezones" => TimeTools::list_timezones(arguments).await,
            _ => Err(anyhow::anyhow!("Tool not found: {}", name)),
        }
    }

    fn get_tool_definitions() -> Value {
        json!([
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
        ])
    }

    fn get_time_formats_resource() -> Value {
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
        })
    }
}