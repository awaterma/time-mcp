use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_main_http_transport() {
    let mut child = Command::new("cargo")
        .args(&["run", "--", "--transport", "http", "--port", "8081"])
        .spawn()
        .expect("Failed to start server");

    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(10);
    let mut connected = false;

    while start_time.elapsed() < timeout {
        if let Ok(res) = reqwest::get("http://127.0.0.1:8081").await {
            if res.status().is_success() {
                connected = true;
                break;
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    if !connected {
        panic!("Failed to connect to server within 10 seconds");
    }

    let client = reqwest::Client::new();
    let res = client
        .get("http://127.0.0.1:8081")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(res.status(), reqwest::StatusCode::OK);

    child.kill().expect("Failed to kill server");
}
