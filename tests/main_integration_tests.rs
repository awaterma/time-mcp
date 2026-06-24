use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Ask the OS for a free port, then release it so the spawned server can bind
/// it. There is a small TOCTOU window between releasing and the child binding,
/// but it is far more robust than a hard-coded port that collides with leftover
/// processes or parallel test runs.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind a probe port")
        .local_addr()
        .expect("Failed to read probe port")
        .port()
}

#[tokio::test]
async fn test_main_http_transport() {
    let port = free_port();

    // Spawn the pre-built binary directly (Cargo builds it before integration
    // tests run) instead of `cargo run`. With `cargo run` a cold build spends
    // most of the connect timeout compiling, and `kill()` would only terminate
    // the cargo wrapper — orphaning the server and leaking its port.
    let mut child = Command::new(env!("CARGO_BIN_EXE_time-mcp-server"))
        .args(["--transport", "http", "--port", &port.to_string()])
        .spawn()
        .expect("Failed to start server");

    let url = format!("http://127.0.0.1:{}", port);
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(10);
    let mut connected = false;

    while start_time.elapsed() < timeout {
        if let Ok(res) = reqwest::get(&url).await {
            if res.status().is_success() {
                connected = true;
                break;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    if !connected {
        let _ = child.kill();
        panic!("Failed to connect to server within 10 seconds");
    }

    let client = reqwest::Client::new();
    let res = client
        .get(&url)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), reqwest::StatusCode::OK);

    child.kill().expect("Failed to kill server");
}
