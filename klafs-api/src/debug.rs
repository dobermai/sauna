//! Debug logging for HTTP traffic
//!
//! This module provides utilities for capturing and logging HTTP request/response
//! traffic for debugging purposes.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, trace};

/// Configuration for debug logging
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Whether debug logging is enabled
    pub enabled: bool,
    /// Optional file path to write debug logs
    pub log_file: Option<PathBuf>,
    /// Whether to log request/response bodies
    pub log_bodies: bool,
    /// Whether to redact sensitive information (passwords, tokens)
    pub redact_sensitive: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_file: None,
            log_bodies: true,
            redact_sensitive: true,
        }
    }
}

impl DebugConfig {
    /// Create a new debug config with logging enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Enable logging to a file
    pub fn with_log_file(mut self, path: PathBuf) -> Self {
        self.log_file = Some(path);
        self
    }

    /// Disable body logging
    pub fn without_bodies(mut self) -> Self {
        self.log_bodies = false;
        self
    }

    /// Disable sensitive data redaction (use with caution!)
    pub fn without_redaction(mut self) -> Self {
        self.redact_sensitive = false;
        self
    }
}

/// HTTP traffic logger
#[derive(Debug)]
pub struct HttpDebugger {
    config: DebugConfig,
    enabled: AtomicBool,
    log_buffer: Arc<RwLock<Vec<TrafficEntry>>>,
}

/// A single HTTP traffic entry
#[derive(Debug, Clone)]
pub struct TrafficEntry {
    /// Unique request ID
    pub request_id: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request method
    pub method: String,
    /// Request URL
    pub url: String,
    /// Request headers (redacted if configured)
    pub request_headers: Vec<(String, String)>,
    /// Request body (redacted if configured)
    pub request_body: Option<String>,
    /// Response status code
    pub response_status: Option<u16>,
    /// Response headers
    pub response_headers: Vec<(String, String)>,
    /// Response body
    pub response_body: Option<String>,
    /// Request duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Any error that occurred
    pub error: Option<String>,
}

impl TrafficEntry {
    /// Create a new traffic entry for a request
    pub fn new(method: &str, url: &str) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            method: method.to_string(),
            url: url.to_string(),
            request_headers: Vec::new(),
            request_body: None,
            response_status: None,
            response_headers: Vec::new(),
            response_body: None,
            duration_ms: None,
            error: None,
        }
    }

    /// Format as a debug string
    pub fn format(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "\n{}\n[{}] {} {}\nRequest ID: {}\n",
            "=".repeat(60),
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f UTC"),
            self.method,
            self.url,
            self.request_id
        ));

        if !self.request_headers.is_empty() {
            output.push_str("\n--- Request Headers ---\n");
            for (key, value) in &self.request_headers {
                output.push_str(&format!("{}: {}\n", key, value));
            }
        }

        if let Some(ref body) = self.request_body {
            output.push_str("\n--- Request Body ---\n");
            output.push_str(body);
            output.push('\n');
        }

        if let Some(status) = self.response_status {
            output.push_str(&format!("\n--- Response Status: {} ---\n", status));
        }

        if !self.response_headers.is_empty() {
            output.push_str("\n--- Response Headers ---\n");
            for (key, value) in &self.response_headers {
                output.push_str(&format!("{}: {}\n", key, value));
            }
        }

        if let Some(ref body) = self.response_body {
            output.push_str("\n--- Response Body ---\n");
            // Try to pretty-print JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                    output.push_str(&pretty);
                } else {
                    output.push_str(body);
                }
            } else {
                // Truncate very long HTML responses
                if body.len() > 2000 {
                    output.push_str(&body[..2000]);
                    output.push_str(&format!("\n... [truncated, {} bytes total]", body.len()));
                } else {
                    output.push_str(body);
                }
            }
            output.push('\n');
        }

        if let Some(duration) = self.duration_ms {
            output.push_str(&format!("\nDuration: {}ms\n", duration));
        }

        if let Some(ref error) = self.error {
            output.push_str(&format!("\n!!! Error: {} !!!\n", error));
        }

        output.push_str(&"=".repeat(60));
        output.push('\n');

        output
    }
}

impl Default for HttpDebugger {
    fn default() -> Self {
        Self::new(DebugConfig::default())
    }
}

impl HttpDebugger {
    /// Create a new HTTP debugger with the given configuration
    pub fn new(config: DebugConfig) -> Self {
        let enabled = config.enabled;
        Self {
            config,
            enabled: AtomicBool::new(enabled),
            log_buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if debugging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable debugging at runtime
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disable debugging at runtime
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Log a request (before sending)
    pub async fn log_request(
        &self,
        method: &str,
        url: &str,
        headers: &reqwest::header::HeaderMap,
        body: Option<&str>,
    ) -> String {
        let mut entry = TrafficEntry::new(method, url);

        // Capture headers
        for (key, value) in headers.iter() {
            let value_str = value.to_str().unwrap_or("<binary>").to_string();
            let value_str = if self.config.redact_sensitive {
                self.redact_header(key.as_str(), &value_str)
            } else {
                value_str
            };
            entry.request_headers.push((key.to_string(), value_str));
        }

        // Capture body
        if self.config.log_bodies {
            if let Some(body) = body {
                entry.request_body = Some(if self.config.redact_sensitive {
                    self.redact_body(body)
                } else {
                    body.to_string()
                });
            }
        }

        let request_id = entry.request_id.clone();

        if self.is_enabled() {
            debug!(
                request_id = %entry.request_id,
                method = %entry.method,
                url = %entry.url,
                "HTTP Request"
            );
            trace!("Request details:\n{}", entry.format());
        }

        // Store in buffer
        self.log_buffer.write().await.push(entry);

        request_id
    }

    /// Log a response
    pub async fn log_response(
        &self,
        request_id: &str,
        status: u16,
        headers: &reqwest::header::HeaderMap,
        body: Option<&str>,
        duration_ms: u64,
    ) {
        let mut buffer = self.log_buffer.write().await;

        if let Some(entry) = buffer.iter_mut().find(|e| e.request_id == request_id) {
            entry.response_status = Some(status);
            entry.duration_ms = Some(duration_ms);

            // Capture response headers (with redaction if enabled)
            for (key, value) in headers.iter() {
                let value_str = value.to_str().unwrap_or("<binary>").to_string();
                let value_str = if self.config.redact_sensitive {
                    self.redact_header(key.as_str(), &value_str)
                } else {
                    value_str
                };
                entry.response_headers.push((key.to_string(), value_str));
            }

            // Capture response body (with redaction if enabled)
            if self.config.log_bodies {
                if let Some(body) = body {
                    entry.response_body = Some(if self.config.redact_sensitive {
                        self.redact_body(body)
                    } else {
                        body.to_string()
                    });
                }
            }

            if self.is_enabled() {
                debug!(
                    request_id = %request_id,
                    status = status,
                    duration_ms = duration_ms,
                    "HTTP Response"
                );
                trace!("Response details:\n{}", entry.format());
            }

            // Write to file if configured
            if let Some(ref path) = self.config.log_file {
                if let Err(e) = self.write_to_file(path, &entry.format()).await {
                    tracing::warn!("Failed to write debug log to file: {}", e);
                }
            }
        }
    }

    /// Log an error
    pub async fn log_error(&self, request_id: &str, error: &str) {
        let mut buffer = self.log_buffer.write().await;

        if let Some(entry) = buffer.iter_mut().find(|e| e.request_id == request_id) {
            entry.error = Some(error.to_string());

            if self.is_enabled() {
                debug!(
                    request_id = %request_id,
                    error = %error,
                    "HTTP Error"
                );
            }
        }
    }

    /// Get all captured traffic entries
    pub async fn get_traffic(&self) -> Vec<TrafficEntry> {
        self.log_buffer.read().await.clone()
    }

    /// Clear the traffic buffer
    pub async fn clear(&self) {
        self.log_buffer.write().await.clear();
    }

    /// Export all traffic to a string
    pub async fn export(&self) -> String {
        let buffer = self.log_buffer.read().await;
        buffer.iter().map(|e| e.format()).collect::<Vec<_>>().join("\n")
    }

    /// Redact sensitive header values
    fn redact_header(&self, key: &str, value: &str) -> String {
        let key_lower = key.to_lowercase();
        if key_lower.contains("authorization")
            || key_lower.contains("cookie")
            || key_lower.contains("token")
            || key_lower.contains("auth")
        {
            if value.len() > 10 {
                format!("{}...REDACTED", &value[..5])
            } else {
                "REDACTED".to_string()
            }
        } else {
            value.to_string()
        }
    }

    /// Redact sensitive body content
    fn redact_body(&self, body: &str) -> String {
        // Try to parse as form data or JSON and redact password fields
        if body.contains("Password=") {
            // Form-encoded
            body.split('&')
                .map(|pair| {
                    if pair.to_lowercase().starts_with("password=") {
                        "Password=REDACTED"
                    } else {
                        pair
                    }
                })
                .collect::<Vec<_>>()
                .join("&")
        } else if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(body) {
            // JSON
            Self::redact_json_passwords(&mut json);
            serde_json::to_string(&json).unwrap_or_else(|_| body.to_string())
        } else {
            body.to_string()
        }
    }

    /// Recursively redact password fields in JSON
    fn redact_json_passwords(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map.iter_mut() {
                    let key_lower = key.to_lowercase();
                    if key_lower.contains("password")
                        || key_lower.contains("secret")
                        || key_lower.contains("token")
                        || key_lower == "pin"
                    {
                        *val = serde_json::Value::String("REDACTED".to_string());
                    } else {
                        Self::redact_json_passwords(val);
                    }
                }
            }
            serde_json::Value::Array(arr) => {
                for item in arr.iter_mut() {
                    Self::redact_json_passwords(item);
                }
            }
            _ => {}
        }
    }

    /// Write to log file
    async fn write_to_file(&self, path: &PathBuf, content: &str) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        file.write_all(content.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }
}

/// Helper for timing operations
pub struct Timer {
    start: Instant,
}

impl Timer {
    /// Start a new timer
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_body_form() {
        let debugger = HttpDebugger::new(DebugConfig::enabled());
        let body = "UserName=test@example.com&Password=secret123&Remember=true";
        let redacted = debugger.redact_body(body);
        assert!(redacted.contains("Password=REDACTED"));
        assert!(redacted.contains("UserName=test@example.com"));
    }

    #[test]
    fn test_redact_body_json() {
        let debugger = HttpDebugger::new(DebugConfig::enabled());
        let body = r#"{"username":"test","password":"secret","pin":"1234"}"#;
        let redacted = debugger.redact_body(body);
        assert!(redacted.contains("REDACTED"));
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("1234"));
    }

    #[test]
    fn test_redact_header() {
        let debugger = HttpDebugger::new(DebugConfig::enabled());

        let cookie = debugger.redact_header("Cookie", ".ASPXAUTH=verylongtokenvalue");
        assert!(cookie.contains("REDACTED"));

        let content_type = debugger.redact_header("Content-Type", "application/json");
        assert_eq!(content_type, "application/json");
    }
}
