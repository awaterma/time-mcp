use crate::mcp::{Transport, messages::Message};
use anyhow::{Result, anyhow};
use std::io::{Stdin, Stdout};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as AsyncBufReader};
use tokio::io::{stdin, stdout};

pub struct StdioTransport {
    stdin: AsyncBufReader<tokio::io::Stdin>,
    stdout: tokio::io::Stdout,
}

impl StdioTransport {
    pub fn new(_stdin: Stdin, _stdout: Stdout) -> Self {
        Self {
            stdin: AsyncBufReader::new(stdin()),
            stdout: stdout(),
        }
    }
}

impl Transport for StdioTransport {
    async fn send_message(&mut self, message: &Message) -> Result<()> {
        let json = serde_json::to_string(message)?;
        self.stdout.write_all(json.as_bytes()).await?;
        self.stdout.write_all(b"\n").await?;
        self.stdout.flush().await?;
        Ok(())
    }

    async fn receive_message(&mut self) -> Result<Option<Message>> {
        let mut line = String::new();
        match self.stdin.read_line(&mut line).await? {
            0 => Ok(None),
            _ => {
                let message = serde_json::from_str(&line)
                    .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;
                Ok(Some(message))
            }
        }
    }
}