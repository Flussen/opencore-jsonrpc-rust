//! # OpenCore JSON-RPC Rust
//!
//! A simple and elegant library for creating JSON-RPC servers that communicate
//! with TypeScript frameworks via stdin/stdout.
//!
//! ## Features
//!
//! - Simple handler registration with type-safe function signatures
//! - Line-delimited JSON protocol for easy integration
//! - Automatic request/response serialization
//! - Clean error handling
//!
//! ## Quick Start
//!
//! ```no_run
//! use opencore_jsonrpc_rust::server::BinaryServer;
//! use serde_json::Value;
//!
//! fn add(params: Vec<Value>) -> Result<Value, String> {
//!     if params.len() != 2 {
//!         return Err("Expected 2 parameters".into());
//!     }
//!     let a = params[0].as_i64().ok_or("Invalid number")?;
//!     let b = params[1].as_i64().ok_or("Invalid number")?;
//!     Ok(Value::from(a + b))
//! }
//!
//! fn main() {
//!     let mut server = BinaryServer::new();
//!     server.register("add", add);
//!     server.run();
//! }
//! ```
//!
//! ## Protocol
//!
//! ### Request Format
//!
//! ```json
//! {
//!   "id": "unique-request-id",
//!   "action": "method-name",
//!   "params": [arg1, arg2, ...]
//! }
//! ```
//!
//! ### Response Format (Success)
//!
//! ```json
//! {
//!   "status": "ok",
//!   "id": "unique-request-id",
//!   "result": <value>
//! }
//! ```
//!
//! ### Response Format (Error)
//!
//! ```json
//! {
//!   "status": "error",
//!   "id": "unique-request-id",
//!   "error": "error message"
//! }
//! ```

pub mod protocol;
pub mod server;
