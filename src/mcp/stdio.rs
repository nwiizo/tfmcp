use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    pin::Pin,
    sync::Arc,
};
use tokio::sync::{mpsc, Mutex};

use tokio::io::AsyncBufReadExt;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Message {
    Request {
        #[serde(rename = "jsonrpc")]
        jsonrpc: String,

        #[serde(rename = "method")]
        method: String,

        #[serde(rename = "id")]
        id: u64,

        #[serde(rename = "params")]
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<serde_json::Value>,
    },
    Notification {
        #[serde(rename = "jsonrpc")]
        jsonrpc: String,

        #[serde(rename = "method")]
        method: String,

        #[serde(rename = "params")]
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<serde_json::Value>,
    },
    Response {
        #[serde(rename = "jsonrpc")]
        jsonrpc: String,

        #[serde(rename = "id")]
        id: u64,

        #[serde(rename = "result")]
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<serde_json::Value>,

        #[serde(rename = "error")]
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<serde_json::Value>,
    },
}

#[allow(dead_code)]
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, message: Message) -> Result<(), Error>;
    fn receive(&self) -> Pin<Box<dyn Stream<Item = Result<Message, Error>> + Send>>;
    async fn close(&self) -> Result<(), Error>;
}

pub struct StdioTransport {
    stdout: Arc<std::sync::Mutex<std::io::Stdout>>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<Result<Message, Error>>>>,
}

impl StdioTransport {
    pub fn new() -> (Self, mpsc::UnboundedSender<Result<Message, Error>>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let transport = Self {
            stdout: Arc::new(std::sync::Mutex::new(std::io::stdout())),
            receiver: Arc::new(Mutex::new(receiver)),
        };

        let stdin = tokio::io::stdin();
        let mut reader = tokio::io::BufReader::new(stdin);
        let sender_clone = sender.clone();

        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        eprintln!("[DEBUG] EOF reached on stdin");
                        break;
                    }
                    Ok(_) => {
                        // Trim whitespace to avoid parsing issues
                        let trimmed_line = line.trim();

                        // Skip empty lines
                        if trimmed_line.is_empty() {
                            continue;
                        }

                        // Debug log the received JSON
                        eprintln!("[DEBUG] Received JSON: {}", trimmed_line);

                        // Use the helper function for more robust parsing
                        let parsed = parse_json_message(trimmed_line);

                        // Log the send attempt
                        eprintln!("[DEBUG] Attempting to send parsed message to channel");
                        
                        if sender_clone.send(parsed).is_err() {
                            eprintln!("[ERROR] Failed to send parsed message to channel - receiver dropped");
                            break;
                        } else {
                            eprintln!("[DEBUG] Successfully sent message to channel");
                        }
                    }
                    Err(e) => {
                        eprintln!("[ERROR] Error reading from stdin: {}", e);
                        let _ = sender_clone.send(Err(Error::Io(format!("Error reading from stdin: {}", e))));
                        break;
                    }
                }
            }
        });

        (transport, sender)
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&self, message: Message) -> Result<(), Error> {
        let mut stdout = self
            .stdout
            .lock()
            .map_err(|_| Error::Other("Failed to lock stdout".into()))?;

        // Use to_string with proper error handling
        let json = match serde_json::to_string(&message) {
            Ok(s) => s,
            Err(e) => {
                return Err(Error::Serialization(format!(
                    "JSON serialization error: {}",
                    e
                )))
            }
        };

        // Debug log the JSON being sent (truncated if very long)
        let truncated_json = if json.len() > 500 {
            format!("{}... (truncated)", &json[0..500])
        } else {
            json.clone()
        };
        eprintln!("[DEBUG] Sending JSON: {}", truncated_json);

        // Write the JSON string followed by a newline and flush
        if let Err(e) = writeln!(stdout, "{}", json) {
            return Err(Error::Io(format!("Failed to write to stdout: {}", e)));
        }

        if let Err(e) = stdout.flush() {
            return Err(Error::Io(format!("Failed to flush stdout: {}", e)));
        }

        Ok(())
    }

    fn receive(&self) -> Pin<Box<dyn Stream<Item = Result<Message, Error>> + Send>> {
        let receiver = Arc::clone(&self.receiver);
        eprintln!("[DEBUG] Created new receiver for message stream");
        
        Box::pin(futures::stream::unfold(receiver, |receiver| async move {
            eprintln!("[DEBUG] Stream waiting for message...");
            
            let mut rx_guard = receiver.lock().await;
            
            match rx_guard.recv().await {
                Some(msg) => {
                    eprintln!("[DEBUG] Stream received message successfully");
                    // Release the lock before returning
                    drop(rx_guard);
                    Some((msg, receiver))
                },
                None => {
                    eprintln!("[DEBUG] Stream closed");
                    None
                }
            }
        }))
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

// Helper function to parse JSON messages with better error handling
fn parse_json_message(json_string: &str) -> Result<Message, Error> {
    // Basic validation for empty input
    if json_string.is_empty() {
        return Err(Error::Serialization("Empty JSON string".into()));
    }

    // Try to fix common JSON issues
    let mut processed_json = json_string.to_string();

    // Remove problematic whitespace characters
    processed_json = processed_json.replace(['\n', '\r', '\t'], " ");

    // Handle unescaped backslashes and quotes if needed
    if processed_json.contains("\\\\") || processed_json.contains("\\\"") {
        processed_json = processed_json.replace("\\\\", "\\").replace("\\\"", "\"");
    }

    // Attempt parsing with modified string
    let parse_result = serde_json::from_str::<Message>(&processed_json);

    match parse_result {
        Ok(msg) => Ok(msg),
        Err(e) => {
            eprintln!("[ERROR] JSON parse error: {}. Input: {}", e, processed_json);

            // Provide additional diagnostics
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&processed_json) {
                eprintln!("[DEBUG] JSON parsed as generic value: {:?}", value);
            } else {
                eprintln!("[ERROR] Could not parse JSON even as generic value");

                // Try to fix more aggressively
                if let Ok(msg) = serde_json::from_str::<Message>(
                    "{\"jsonrpc\":\"2.0\",\"method\":\"unknown\",\"id\":0}}",
                ) {
                    eprintln!("[DEBUG] Returning fallback message");
                    return Ok(msg);
                }
            }

            Err(Error::Serialization(format!("JSON parse error: {}", e)))
        }
    }
}
