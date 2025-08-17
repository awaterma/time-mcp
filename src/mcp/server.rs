use crate::mcp::{
    messages::{Message, JsonRpcMessage, Request, Response, JsonRpcError},
    Transport, ToolHandler, ResourceHandler, PromptHandler, ServerCapabilities,
    ToolsCapability, ResourcesCapability, PromptsCapability
};
use crate::time_tools::TimeToolsHandler;
use anyhow::Result;
use serde_json::{json, Value};
use tracing::{info, warn, error};

pub struct McpServer<T: Transport> {
    transport: T,
    time_handler: TimeToolsHandler,
}

impl<T: Transport> McpServer<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            time_handler: TimeToolsHandler::new(),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        info!("Time MCP Server started");

        loop {
            match self.transport.receive_message().await {
                Ok(Some(message)) => {
                    if let Err(e) = self.handle_message(message).await {
                        error!("Error handling message: {}", e);
                    }
                }
                Ok(None) => {
                    info!("Connection closed");
                    break;
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::JsonRpc(JsonRpcMessage::Request(request)) => {
                let response = self.handle_request(request).await;
                self.transport.send_message(&Message::JsonRpc(JsonRpcMessage::Response(response))).await?;
            }
            Message::JsonRpc(JsonRpcMessage::Notification(notification)) => {
                info!("Received notification: {}", notification.method);
            }
            Message::JsonRpc(JsonRpcMessage::Response(_)) => {
                warn!("Unexpected response message received");
            }
        }
        Ok(())
    }

    async fn handle_request(&self, request: Request) -> Response {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tools_call(request.params).await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resources_read(request.params).await,
            "prompts/list" => self.handle_prompts_list().await,
            "prompts/get" => self.handle_prompts_get(request.params).await,
            _ => Err(JsonRpcError::method_not_found()),
        };

        match result {
            Ok(result) => Response {
                id: request.id,
                result: Some(result),
                error: None,
            },
            Err(error) => Response {
                id: request.id,
                result: None,
                error: Some(error),
            },
        }
    }

    async fn handle_initialize(&self, _params: Option<Value>) -> Result<Value, JsonRpcError> {
        let capabilities = ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: Some(false) }),
            resources: Some(ResourcesCapability { 
                subscribe: Some(false), 
                list_changed: Some(false) 
            }),
            prompts: Some(PromptsCapability { list_changed: Some(false) }),
        };

        Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": capabilities,
            "serverInfo": {
                "name": "time-mcp-server",
                "version": "1.0.0"
            }
        }))
    }

    async fn handle_tools_list(&self) -> Result<Value, JsonRpcError> {
        let tools = self.time_handler.list_tools();
        Ok(json!({ "tools": tools }))
    }

    async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or(JsonRpcError::invalid_params())?;
        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or(JsonRpcError::invalid_params())?;
        let arguments = params.get("arguments")
            .cloned()
            .unwrap_or(Value::Null);

        match self.time_handler.handle_tool_call(name, arguments).await {
            Ok(result) => Ok(json!({ "content": [{ "type": "text", "text": result.to_string() }] })),
            Err(_) => Err(JsonRpcError::internal_error()),
        }
    }

    async fn handle_resources_list(&self) -> Result<Value, JsonRpcError> {
        let resources = self.time_handler.list_resources();
        Ok(json!({ "resources": resources }))
    }

    async fn handle_resources_read(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or(JsonRpcError::invalid_params())?;
        let uri = params.get("uri")
            .and_then(|v| v.as_str())
            .ok_or(JsonRpcError::invalid_params())?;

        match self.time_handler.read_resource(uri).await {
            Ok(content) => Ok(json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": content.to_string() }] })),
            Err(_) => Err(JsonRpcError::internal_error()),
        }
    }

    async fn handle_prompts_list(&self) -> Result<Value, JsonRpcError> {
        let prompts = self.time_handler.list_prompts();
        Ok(json!({ "prompts": prompts }))
    }

    async fn handle_prompts_get(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params.ok_or(JsonRpcError::invalid_params())?;
        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or(JsonRpcError::invalid_params())?;
        let arguments = params.get("arguments")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        match self.time_handler.get_prompt(name, Some(arguments)).await {
            Ok(prompt) => Ok(prompt),
            Err(_) => Err(JsonRpcError::internal_error()),
        }
    }
}