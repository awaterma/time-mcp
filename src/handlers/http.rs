use crate::{
    auth::AuthManager,
    config::{ServerConfig, DEFAULT_PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION},
    models::McpRequest,
    tools::TimeTools,
};
use anyhow::Result;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use chrono_tz::TZ_VARIANTS;
use serde_json::{json, Value};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Clone)]
pub struct HttpHandler {
    auth: AuthManager,
}

impl HttpHandler {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            auth: AuthManager::new(config.auth_enabled),
        }
    }

    pub async fn run(self, host: &str, port: u16) -> Result<()> {
        let app = Router::new()
            .route("/", get(Self::health_check))
            .route("/mcp/capabilities", get(Self::get_capabilities))
            .route("/mcp/tools/call", post(Self::call_tool))
            .route("/mcp/resources/read", post(Self::read_resource))
            .route("/mcp/prompts/get", post(Self::get_prompt))
            .route("/health", get(Self::health_check))
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive())
            .with_state(self);

        let addr = format!("{}:{}", host, port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        tracing::info!("HTTP server listening on {}", addr);
        axum::serve(listener, app).await?;

        Ok(())
    }

    async fn health_check() -> Json<Value> {
        Json(json!({
            "status": "healthy",
            "timestamp": Utc::now().to_rfc3339(),
            "version": SERVER_VERSION
        }))
    }

    async fn get_capabilities(
        State(handler): State<HttpHandler>,
        headers: HeaderMap,
    ) -> Result<Json<Value>, StatusCode> {
        handler
            .auth
            .authenticate(&headers)
            .await
            .map_err(StatusCode::from)?;

        Ok(Json(json!({
            "protocolVersion": DEFAULT_PROTOCOL_VERSION,
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
        })))
    }

    async fn call_tool(
        State(handler): State<HttpHandler>,
        headers: HeaderMap,
        Json(request): Json<McpRequest>,
    ) -> Result<Json<Value>, StatusCode> {
        handler
            .auth
            .authenticate(&headers)
            .await
            .map_err(StatusCode::from)?;

        let tool_name = request.name.ok_or(StatusCode::BAD_REQUEST)?;
        let arguments = request.arguments.unwrap_or(Value::Null);

        let result = Self::execute_tool(&tool_name, arguments).await;

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
        State(handler): State<HttpHandler>,
        headers: HeaderMap,
        Json(request): Json<McpRequest>,
    ) -> Result<Json<Value>, StatusCode> {
        handler
            .auth
            .authenticate(&headers)
            .await
            .map_err(StatusCode::from)?;

        let uri = request.uri.ok_or(StatusCode::BAD_REQUEST)?;

        let content = match uri.as_str() {
            "timezone_database" => {
                let timezones: Vec<String> =
                    TZ_VARIANTS.iter().map(|tz| tz.name().to_string()).collect();
                json!({
                    "timezones": timezones,
                    "total_count": timezones.len()
                })
                .to_string()
            }
            "time_formats" => Self::get_time_formats_resource().to_string(),
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
        State(handler): State<HttpHandler>,
        headers: HeaderMap,
        Json(request): Json<McpRequest>,
    ) -> Result<Json<Value>, StatusCode> {
        handler
            .auth
            .authenticate(&headers)
            .await
            .map_err(StatusCode::from)?;

        let prompt_name = request.name.ok_or(StatusCode::BAD_REQUEST)?;

        match prompt_name.as_str() {
            "time_query_assistant" => {
                let user_query = request
                    .arguments
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
