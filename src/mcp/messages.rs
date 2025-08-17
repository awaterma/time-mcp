use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "jsonrpc")]
pub enum Message {
    #[serde(rename = "2.0")]
    JsonRpc(JsonRpcMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: Value,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: Value,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        }
    }

    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: None,
        }
    }

    pub fn method_not_found() -> Self {
        Self {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        }
    }

    pub fn invalid_params() -> Self {
        Self {
            code: -32602,
            message: "Invalid params".to_string(),
            data: None,
        }
    }

    pub fn internal_error() -> Self {
        Self {
            code: -32603,
            message: "Internal error".to_string(),
            data: None,
        }
    }

    pub fn invalid_timezone() -> Self {
        Self {
            code: -32000,
            message: "Invalid timezone".to_string(),
            data: None,
        }
    }

    pub fn invalid_timestamp() -> Self {
        Self {
            code: -32001,
            message: "Invalid timestamp format".to_string(),
            data: None,
        }
    }

    pub fn timezone_conversion_error() -> Self {
        Self {
            code: -32002,
            message: "Timezone conversion error".to_string(),
            data: None,
        }
    }
}