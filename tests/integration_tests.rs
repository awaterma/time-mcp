use serde_json::json;
use std::process::{Command, Stdio};
use std::io::Write;

#[tokio::test]
async fn test_stdio_get_current_time() {
    let mut child = Command::new("./target/release/time-mcp-server")
        .arg("--transport=stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    let stdin = child.stdin.as_mut().unwrap();
    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_current_time",
            "arguments": {}
        },
        "id": 1
    });
    
    writeln!(stdin, "{}", request).unwrap();
    stdin.flush().unwrap();
    
    let output = child.wait_with_output().unwrap();
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"]["content"][0]["text"].is_string());
}

#[tokio::test]
async fn test_stdio_all_tools() {
    let tools = [
        ("get_current_time", json!({"timezone": "UTC"})),
        ("convert_timezone", json!({
            "timestamp": "2025-08-17T10:30:00Z",
            "from_timezone": "UTC", 
            "to_timezone": "America/New_York"
        })),
        ("calculate_duration", json!({
            "start_time": "2025-08-17T10:00:00Z",
            "end_time": "2025-08-17T11:00:00Z",
            "units": "hours"
        })),
        ("format_time", json!({
            "timestamp": "2025-08-17T10:30:00Z",
            "format": "iso8601"
        })),
        ("get_timezone_info", json!({"timezone": "America/New_York"})),
        ("list_timezones", json!({"region": "America"}))
    ];

    for (tool_name, args) in tools {
        let mut child = Command::new("./target/release/time-mcp-server")
            .arg("--transport=stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start server");

        let stdin = child.stdin.as_mut().unwrap();
        let request = json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": args
            },
            "id": 1
        });
        
        writeln!(stdin, "{}", request).unwrap();
        stdin.flush().unwrap();
        
        let output = child.wait_with_output().unwrap();
        let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        
        assert_eq!(response["jsonrpc"], "2.0", "Tool {} failed", tool_name);
        assert_eq!(response["id"], 1);
        assert!(response["result"]["content"][0]["text"].is_string(), "Tool {} response invalid", tool_name);
    }
}