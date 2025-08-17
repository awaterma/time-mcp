use crate::mcp::{Transport, messages::Message};
use anyhow::Result;
use tokio::net::TcpListener;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, Method, StatusCode};
use hyper_util::rt::TokioIo;
use http_body_util::{BodyExt, Full};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct HttpTransport {
    listener: TcpListener,
    message_queue: Arc<Mutex<Vec<Message>>>,
}

impl HttpTransport {
    pub fn new(listener: TcpListener) -> Self {
        Self { 
            listener,
            message_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn handle_request(
        req: Request<hyper::body::Incoming>,
        message_queue: Arc<Mutex<Vec<Message>>>,
    ) -> Result<Response<Full<bytes::Bytes>>, Infallible> {
        match (req.method(), req.uri().path()) {
            (&Method::POST, "/mcp/tools/call") => {
                let body_bytes = match req.collect().await {
                    Ok(collected) => collected.to_bytes(),
                    Err(_) => {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Full::new("Failed to read body".into()))
                            .unwrap());
                    }
                };

                let json_str = match String::from_utf8(body_bytes.to_vec()) {
                    Ok(s) => s,
                    Err(_) => {
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Full::new("Invalid UTF-8".into()))
                            .unwrap());
                    }
                };

                match serde_json::from_str::<Message>(&json_str) {
                    Ok(message) => {
                        message_queue.lock().await.push(message);
                        Ok(Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "application/json")
                            .body(Full::new(r#"{"status": "queued"}"#.into()))
                            .unwrap())
                    }
                    Err(_) => {
                        Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Full::new("Invalid JSON".into()))
                            .unwrap())
                    }
                }
            }
            (&Method::GET, "/mcp/capabilities") => {
                let capabilities = serde_json::json!({
                    "protocolVersion": "2025-03-26",
                    "capabilities": {
                        "tools": { "list_changed": false },
                        "resources": { "subscribe": false, "list_changed": false },
                        "prompts": { "list_changed": false }
                    },
                    "serverInfo": {
                        "name": "time-mcp-server",
                        "version": "1.0.0"
                    }
                });

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(Full::new(capabilities.to_string().into()))
                    .unwrap())
            }
            _ => {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Full::new("Not Found".into()))
                    .unwrap())
            }
        }
    }
}

impl Transport for HttpTransport {
    async fn send_message(&mut self, _message: &Message) -> Result<()> {
        Ok(())
    }

    async fn receive_message(&mut self) -> Result<Option<Message>> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let io = TokioIo::new(stream);
            let message_queue = self.message_queue.clone();

            tokio::task::spawn(async move {
                let service = service_fn(move |req| {
                    Self::handle_request(req, message_queue.clone())
                });

                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service)
                    .await
                {
                    tracing::error!("Error serving connection: {:?}", err);
                }
            });

            let mut queue = self.message_queue.lock().await;
            if let Some(message) = queue.pop() {
                return Ok(Some(message));
            }
        }
    }
}