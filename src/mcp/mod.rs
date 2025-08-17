use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use anyhow::Result;

pub mod messages;
pub mod server;

pub use server::McpServer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<PromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: Option<ToolsCapability>,
    pub resources: Option<ResourcesCapability>,
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    pub subscribe: Option<bool>,
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    pub list_changed: Option<bool>,
}

pub trait Transport {
    async fn send_message(&mut self, message: &messages::Message) -> Result<()>;
    async fn receive_message(&mut self) -> Result<Option<messages::Message>>;
}

pub trait ToolHandler {
    async fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<Value>;
    fn list_tools(&self) -> Vec<Tool>;
}

pub trait ResourceHandler {
    async fn read_resource(&self, uri: &str) -> Result<Value>;
    fn list_resources(&self) -> Vec<Resource>;
}

pub trait PromptHandler {
    async fn get_prompt(&self, name: &str, arguments: Option<HashMap<String, Value>>) -> Result<Value>;
    fn list_prompts(&self) -> Vec<Prompt>;
}