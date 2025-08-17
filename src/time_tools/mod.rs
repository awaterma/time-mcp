use crate::mcp::{
    Tool, Resource, Prompt, PromptArgument,
    ToolHandler, ResourceHandler, PromptHandler
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use chrono_tz::{Tz, TZ_VARIANTS};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct TimeToolsHandler;

impl TimeToolsHandler {
    pub fn new() -> Self {
        Self
    }
}

impl ToolHandler for TimeToolsHandler {
    async fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            "get_current_time" => self.get_current_time(arguments).await,
            "convert_timezone" => self.convert_timezone(arguments).await,
            "calculate_duration" => self.calculate_duration(arguments).await,
            "format_time" => self.format_time(arguments).await,
            "get_timezone_info" => self.get_timezone_info(arguments).await,
            "list_timezones" => self.list_timezones(arguments).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "get_current_time".to_string(),
                description: "Get the current time in various formats and timezones".to_string(),
                input_schema: json!({
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
                            "description": "Custom strftime format string (required if format is 'custom')"
                        }
                    }
                }),
            },
            Tool {
                name: "convert_timezone".to_string(),
                description: "Convert time between different timezones".to_string(),
                input_schema: json!({
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
                        },
                        "format": {
                            "type": "string",
                            "enum": ["iso", "unix", "human"],
                            "description": "Output format",
                            "default": "iso"
                        }
                    },
                    "required": ["timestamp", "from_timezone", "to_timezone"]
                }),
            },
            Tool {
                name: "calculate_duration".to_string(),
                description: "Calculate time difference between two timestamps".to_string(),
                input_schema: json!({
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
                }),
            },
            Tool {
                name: "format_time".to_string(),
                description: "Format timestamps according to various standards".to_string(),
                input_schema: json!({
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
                            "description": "Custom format string (required if format is 'custom')"
                        },
                        "timezone": {
                            "type": "string",
                            "description": "Target timezone",
                            "default": "UTC"
                        }
                    },
                    "required": ["timestamp", "format"]
                }),
            },
            Tool {
                name: "get_timezone_info".to_string(),
                description: "Get detailed information about a specific timezone".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "timezone": {
                            "type": "string",
                            "description": "Timezone identifier"
                        }
                    },
                    "required": ["timezone"]
                }),
            },
            Tool {
                name: "list_timezones".to_string(),
                description: "List available timezone identifiers".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "region": {
                            "type": "string",
                            "description": "Filter by region (e.g., 'America', 'Europe')"
                        }
                    }
                }),
            },
        ]
    }
}

impl TimeToolsHandler {
    async fn get_current_time(&self, arguments: Value) -> Result<Value> {
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

        Ok(result)
    }

    async fn convert_timezone(&self, arguments: Value) -> Result<Value> {
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

        Ok(json!({
            "original": {
                "timestamp": dt.to_rfc3339(),
                "timezone": from_tz_str
            },
            "converted": {
                "timestamp": converted.to_rfc3339(),
                "timezone": to_tz_str
            }
        }))
    }

    async fn calculate_duration(&self, arguments: Value) -> Result<Value> {
        let start_str = arguments.get("start_time")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("start_time required"))?;
        let end_str = arguments.get("end_time")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("end_time required"))?;

        let start = DateTime::parse_from_rfc3339(start_str)
            .map_err(|_| anyhow::anyhow!("Invalid start_time format"))?;
        let end = DateTime::parse_from_rfc3339(end_str)
            .map_err(|_| anyhow::anyhow!("Invalid end_time format"))?;

        let duration = end.signed_duration_since(start);
        let total_seconds = duration.num_seconds();

        Ok(json!({
            "duration": {
                "total_seconds": total_seconds,
                "minutes": total_seconds / 60,
                "hours": total_seconds / 3600,
                "days": total_seconds / 86400,
                "human_readable": format!("{} seconds", total_seconds)
            }
        }))
    }

    async fn format_time(&self, arguments: Value) -> Result<Value> {
        let timestamp_str = arguments.get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("timestamp required"))?;
        let format = arguments.get("format")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("format required"))?;
        let timezone = arguments.get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("UTC");

        let tz: Tz = timezone.parse()
            .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", timezone))?;

        let dt = DateTime::parse_from_rfc3339(timestamp_str)
            .map_err(|_| anyhow::anyhow!("Invalid timestamp format"))?
            .with_timezone(&tz);

        let formatted = match format {
            "iso8601" => dt.to_rfc3339(),
            "rfc3339" => dt.to_rfc3339(),
            "unix" => dt.timestamp().to_string(),
            "custom" => {
                let custom_format = arguments.get("custom_format")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow::anyhow!("custom_format required for custom format"))?;
                dt.format(custom_format).to_string()
            },
            _ => return Err(anyhow::anyhow!("Invalid format: {}", format)),
        };

        Ok(json!({
            "formatted": formatted,
            "timezone": timezone
        }))
    }

    async fn get_timezone_info(&self, arguments: Value) -> Result<Value> {
        let timezone_str = arguments.get("timezone")
            .and_then(|v| v.as_str())
            .ok_or(anyhow::anyhow!("timezone required"))?;

        let tz: Tz = timezone_str.parse()
            .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", timezone_str))?;

        let now = Utc::now().with_timezone(&tz);
        let offset = now.offset();

        Ok(json!({
            "timezone": timezone_str,
            "offset": offset.to_string(),
            "abbreviation": now.format("%Z").to_string()
        }))
    }

    async fn list_timezones(&self, arguments: Value) -> Result<Value> {
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

        Ok(json!({
            "timezones": timezones,
            "count": timezones.len()
        }))
    }
}

impl ResourceHandler for TimeToolsHandler {
    async fn read_resource(&self, uri: &str) -> Result<Value> {
        match uri {
            "timezone_database" => {
                let timezones: Vec<String> = TZ_VARIANTS
                    .iter()
                    .map(|tz| tz.name().to_string())
                    .collect();
                Ok(json!({
                    "timezones": timezones,
                    "total_count": timezones.len(),
                    "regions": ["Africa", "America", "Antarctica", "Arctic", "Asia", "Atlantic", "Australia", "Europe", "Indian", "Pacific"]
                }))
            },
            "time_formats" => {
                Ok(json!({
                    "supported_formats": {
                        "iso8601": "2025-08-17T10:30:00Z",
                        "rfc3339": "2025-08-17T10:30:00Z",
                        "unix": "1723892200",
                        "custom": "Use strftime format strings like '%Y-%m-%d %H:%M:%S'"
                    },
                    "examples": {
                        "iso8601": "2025-08-17T10:30:00Z",
                        "human_readable": "Saturday, August 17, 2025 at 10:30 AM UTC",
                        "custom_formats": [
                            "%Y-%m-%d %H:%M:%S",
                            "%B %d, %Y",
                            "%I:%M %p"
                        ]
                    }
                }))
            },
            _ => Err(anyhow::anyhow!("Unknown resource: {}", uri)),
        }
    }

    fn list_resources(&self) -> Vec<Resource> {
        vec![
            Resource {
                uri: "timezone_database".to_string(),
                name: "Timezone Database".to_string(),
                description: "Complete IANA timezone database with all available timezones".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "time_formats".to_string(),
                name: "Time Formats".to_string(),
                description: "Documentation of supported time formats and examples".to_string(),
                mime_type: Some("application/json".to_string()),
            },
        ]
    }
}

impl PromptHandler for TimeToolsHandler {
    async fn get_prompt(&self, name: &str, arguments: Option<HashMap<String, Value>>) -> Result<Value> {
        match name {
            "time_query_assistant" => {
                let user_query = arguments
                    .as_ref()
                    .and_then(|args| args.get("user_query"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("general time query");

                let current_time = Utc::now();
                
                Ok(json!({
                    "description": "Assistant for time-related queries",
                    "messages": [
                        {
                            "role": "system",
                            "content": {
                                "type": "text",
                                "text": format!(
                                    "You are a time query assistant. Help the user with their time-related question: '{}'. Current UTC time: {}. You have access to comprehensive timezone conversion, duration calculation, and time formatting tools.",
                                    user_query,
                                    current_time.to_rfc3339()
                                )
                            }
                        }
                    ]
                }))
            },
            _ => Err(anyhow::anyhow!("Unknown prompt: {}", name)),
        }
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        vec![
            Prompt {
                name: "time_query_assistant".to_string(),
                description: "Template for helping users with time-related queries".to_string(),
                arguments: vec![
                    PromptArgument {
                        name: "user_query".to_string(),
                        description: "The user's time-related question".to_string(),
                        required: true,
                    },
                ],
            },
        ]
    }
}