//! Klafs Sauna Control Library
//!
//! This crate provides a Rust client for interacting with the Klafs sauna control API.
//! It handles authentication, session management, and provides typed access to sauna
//! status and control functions.
//!
//! # Features
//!
//! - Async API using tokio
//! - Automatic cookie/session management
//! - Strongly typed request/response models
//! - Comprehensive error handling
//! - HTTP traffic debugging
//!
//! # Example
//!
//! ```no_run
//! use klafs_api::{KlafsClient, SaunaMode};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client and authenticate
//!     let client = KlafsClient::new();
//!     client.login("user@example.com", "password").await?;
//!
//!     // Get sauna status
//!     let status = client.get_status("your-sauna-uuid").await?;
//!
//!     println!("Connected: {}", status.is_connected);
//!     println!("Powered On: {}", status.is_powered_on);
//!     println!("Current Temperature: {}°C", status.current_temperature);
//!     println!("Target Temperature: {}°C", status.target_temperature());
//!
//!     if let Some(mode) = status.current_mode() {
//!         println!("Mode: {}", mode);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Debug Logging
//!
//! Enable HTTP traffic logging for debugging:
//!
//! ```no_run
//! use klafs_api::{KlafsClient, ClientConfig, DebugConfig};
//! use std::path::PathBuf;
//!
//! let config = ClientConfig {
//!     debug: DebugConfig::enabled().with_log_file(PathBuf::from("klafs-debug.log")),
//!     ..Default::default()
//! };
//!
//! let client = KlafsClient::with_config(config);
//! // All HTTP traffic will be logged to klafs-debug.log
//! ```
//!
//! # Security Warning
//!
//! **Klafs locks accounts after 3 failed login attempts!**
//!
//! Be careful with automated login attempts and ensure credentials are correct
//! before attempting to authenticate.

mod client;
pub mod debug;
mod error;
mod models;

pub use client::{ClientConfig, KlafsClient, DEFAULT_BASE_URL};
pub use debug::DebugConfig;
pub use error::{KlafsError, Result};
pub use models::{SaunaInfo, SaunaMode, SaunaStatus, StatusCode};
