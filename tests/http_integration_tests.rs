use anyhow::Result;
use std::net::TcpListener;
use time_mcp_server::config::{ServerConfig, TransportType};
use time_mcp_server::handlers::http::HttpHandler;
use tokio::time::{sleep, Duration};

async fn start_http_server(config: ServerConfig) -> Result<()> {
    let handler = HttpHandler::new(config.clone());
    if let TransportType::Http { host, port } = config.transport {
        handler.run(&host, port).await
    } else {
        Err(anyhow::anyhow!("Invalid transport type for HTTP server"))
    }
}

fn get_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test]
async fn test_http_server_starts_and_responds() {
    let port = get_available_port();
    let config = ServerConfig {
        transport: TransportType::Http {
            host: "127.0.0.1".to_string(),
            port,
        },
        host: "127.0.0.1".to_string(),
        port,
        auth_enabled: false,
    };

    // Run the server in a separate thread
    tokio::spawn(async move {
        start_http_server(config).await.unwrap();
    });

    // Give the server a moment to start up
    sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://127.0.0.1:{}", port))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn test_http_get_capabilities() {
    let port = get_available_port();
    let config = ServerConfig {
        transport: TransportType::Http {
            host: "127.0.0.1".to_string(),
            port,
        },
        host: "127.0.0.1".to_string(),
        port,
        auth_enabled: false,
    };

    tokio::spawn(async move {
        start_http_server(config).await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://127.0.0.1:{}/mcp/capabilities", port))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = res.json().await.expect("Failed to parse json");
    assert_eq!(body["protocolVersion"], "1.0");
}

#[tokio::test]
async fn test_http_call_tool() {
    let port = get_available_port();
    let config = ServerConfig {
        transport: TransportType::Http {
            host: "127.0.0.1".to_string(),
            port,
        },
        host: "127.0.0.1".to_string(),
        port,
        auth_enabled: false,
    };

    tokio::spawn(async move {
        start_http_server(config).await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://127.0.0.1:{}/mcp/tools/call", port))
        .json(&serde_json::json!({
            "name": "get_current_time",
            "arguments": {
                "timezone": "UTC"
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = res.json().await.expect("Failed to parse json");
    assert!(body["content"][0]["text"].is_string());
}

#[tokio::test]
async fn test_http_read_resource() {
    let port = get_available_port();
    let config = ServerConfig {
        transport: TransportType::Http {
            host: "127.0.0.1".to_string(),
            port,
        },
        host: "127.0.0.1".to_string(),
        port,
        auth_enabled: false,
    };

    tokio::spawn(async move {
        start_http_server(config).await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://127.0.0.1:{}/mcp/resources/read", port))
        .json(&serde_json::json!({
            "uri": "timezone_database"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = res.json().await.expect("Failed to parse json");
    assert_eq!(body["contents"][0]["uri"], "timezone_database");
    assert!(body["contents"][0]["text"].is_string());
}

#[tokio::test]
async fn test_http_get_prompt() {
    let port = get_available_port();
    let config = ServerConfig {
        transport: TransportType::Http {
            host: "127.0.0.1".to_string(),
            port,
        },
        host: "127.0.0.1".to_string(),
        port,
        auth_enabled: false,
    };

    tokio::spawn(async move {
        start_http_server(config).await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://127.0.0.1:{}/mcp/prompts/get", port))
        .json(&serde_json::json!({
            "name": "time_query_assistant"
        }))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), reqwest::StatusCode::OK);
    let body: serde_json::Value = res.json().await.expect("Failed to parse json");
    assert_eq!(body["description"], "Assistant for time-related queries");
}

#[tokio::test]
async fn test_http_auth_required() {
    let port = get_available_port();
    let config = ServerConfig {
        transport: TransportType::Http {
            host: "127.0.0.1".to_string(),
            port,
        },
        host: "127.0.0.1".to_string(),
        port,
        auth_enabled: true,
    };

    tokio::spawn(async move {
        start_http_server(config).await.unwrap();
    });

    sleep(Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("http://127.0.0.1:{}/mcp/capabilities", port))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
}
