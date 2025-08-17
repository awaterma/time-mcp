use time_mcp_server::time_tools::TimeToolsHandler;
use time_mcp_server::mcp::{ToolHandler, ResourceHandler, PromptHandler};
use serde_json::json;

#[tokio::test]
async fn test_get_current_time() {
    let handler = TimeToolsHandler::new();
    let result = handler.handle_tool_call("get_current_time", json!({})).await;
    assert!(result.is_ok());
    
    let result_value = result.unwrap();
    assert!(result_value.get("timestamp").is_some());
    assert!(result_value.get("timezone").is_some());
}

#[tokio::test]
async fn test_convert_timezone() {
    let handler = TimeToolsHandler::new();
    let args = json!({
        "timestamp": "2025-08-17T10:30:00Z",
        "from_timezone": "UTC",
        "to_timezone": "America/New_York"
    });
    
    let result = handler.handle_tool_call("convert_timezone", args).await;
    assert!(result.is_ok());
    
    let result_value = result.unwrap();
    assert!(result_value.get("original").is_some());
    assert!(result_value.get("converted").is_some());
}

#[tokio::test]
async fn test_list_timezones() {
    let handler = TimeToolsHandler::new();
    let result = handler.handle_tool_call("list_timezones", json!({})).await;
    assert!(result.is_ok());
    
    let result_value = result.unwrap();
    assert!(result_value.get("timezones").is_some());
    assert!(result_value.get("count").is_some());
}

#[tokio::test]
async fn test_timezone_database_resource() {
    let handler = TimeToolsHandler::new();
    let result = handler.read_resource("timezone_database").await;
    assert!(result.is_ok());
    
    let result_value = result.unwrap();
    assert!(result_value.get("timezones").is_some());
    assert!(result_value.get("total_count").is_some());
}

#[tokio::test]
async fn test_time_query_prompt() {
    let handler = TimeToolsHandler::new();
    let mut args = std::collections::HashMap::new();
    args.insert("user_query".to_string(), json!("What time is it in Tokyo?"));
    
    let result = handler.get_prompt("time_query_assistant", Some(args)).await;
    assert!(result.is_ok());
    
    let result_value = result.unwrap();
    assert!(result_value.get("description").is_some());
    assert!(result_value.get("messages").is_some());
}