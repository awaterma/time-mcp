use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;
use anyhow::Result;

#[derive(Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub user_id: String,
    pub scopes: Vec<String>,
    pub expires_at: SystemTime,
}

impl TokenInfo {
    pub fn is_expired(&self) -> bool {
        self.expires_at <= SystemTime::now()
    }
}

#[derive(Deserialize)]
pub struct McpRequest {
    pub name: Option<String>,
    pub arguments: Option<Value>,
    pub uri: Option<String>,
}

#[derive(Debug)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

impl McpError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message)
    }
    
    pub fn method_not_found(message: impl Into<String>) -> Self {
        Self::new(-32601, message)
    }
    
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(-32603, message)
    }
}

impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        McpError::internal_error(err.to_string())
    }
}

pub type McpResult<T> = Result<T, McpError>;

#[derive(Serialize)]
pub struct McpResponse<T> {
    pub jsonrpc: String,
    pub id: Value,
    pub result: Option<T>,
    pub error: Option<McpError>,
}

impl<T: Serialize> McpResponse<T> {
    pub fn success(id: Value, result: T) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }
    
    pub fn error(id: Value, error: McpError) -> McpResponse<()> {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

impl Serialize for McpError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("McpError", 2)?;
        state.serialize_field("code", &self.code)?;
        state.serialize_field("message", &self.message)?;
        state.end()
    }
}