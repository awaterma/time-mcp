use crate::models::{TokenInfo, McpError, McpResult};
use axum::http::{HeaderMap, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AuthManager {
    enabled: bool,
    tokens: Arc<RwLock<HashMap<String, TokenInfo>>>,
}

impl AuthManager {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn authenticate(&self, headers: &HeaderMap) -> McpResult<()> {
        if !self.enabled {
            return Ok(());
        }
        
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| McpError::new(401, "Authorization header required"))?;
        
        if !auth_header.starts_with("Bearer ") {
            return Err(McpError::new(401, "Invalid authorization format"));
        }
        
        let token = &auth_header[7..];
        let tokens = self.tokens.read().await;
        
        match tokens.get(token) {
            Some(token_info) if !token_info.is_expired() => Ok(()),
            Some(_) => Err(McpError::new(401, "Token expired")),
            None => Err(McpError::new(401, "Invalid token")),
        }
    }
    
    pub async fn add_token(&self, token: String, info: TokenInfo) {
        let mut tokens = self.tokens.write().await;
        tokens.insert(token, info);
    }
    
    pub async fn remove_expired_tokens(&self) {
        let mut tokens = self.tokens.write().await;
        tokens.retain(|_, info| !info.is_expired());
    }
}

impl From<McpError> for StatusCode {
    fn from(error: McpError) -> Self {
        match error.code {
            401 => StatusCode::UNAUTHORIZED,
            400 | -32602 => StatusCode::BAD_REQUEST,
            404 | -32601 => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}