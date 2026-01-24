//! Binary JSON-RPC server implementation.
//!
//! This module provides a simple server that communicates via stdin/stdout
//! using line-delimited JSON messages.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use serde_json::Value;

use crate::protocol::{Request, Response};

/// A handler function that processes JSON-RPC requests.
///
/// # Arguments
///
/// * `params` - Vector of JSON values representing the request parameters
///
/// # Returns
///
/// * `Ok(Value)` - Successful result as a JSON value
/// * `Err(String)` - Error message describing what went wrong
///
/// # Example
///
/// ```rust
/// use serde_json::Value;
///
/// fn add(params: Vec<Value>) -> Result<Value, String> {
///     if params.len() != 2 {
///         return Err("Expected 2 parameters".into());
///     }
///     let a = params[0].as_i64().ok_or("Invalid number")?;
///     let b = params[1].as_i64().ok_or("Invalid number")?;
///     Ok(Value::from(a + b))
/// }
/// ```
pub type Handler = fn(Vec<Value>) -> Result<Value, String>;

/// A JSON-RPC server that communicates via stdin/stdout.
///
/// The server reads line-delimited JSON requests from stdin and writes
/// line-delimited JSON responses to stdout. This makes it easy to integrate
/// with TypeScript or other language frameworks using process spawning.
///
/// # Example
///
/// ```no_run
/// use opencore_jsonrpc_rust::server::BinaryServer;
/// use serde_json::Value;
///
/// fn multiply(params: Vec<Value>) -> Result<Value, String> {
///     if params.len() != 2 {
///         return Err("Expected 2 parameters".into());
///     }
///     let a = params[0].as_f64().ok_or("Invalid number")?;
///     let b = params[1].as_f64().ok_or("Invalid number")?;
///     Ok(Value::from(a * b))
/// }
///
/// fn main() {
///     let mut server = BinaryServer::new();
///     server.register("multiply", multiply);
///     server.run();
/// }
/// ```
#[derive(Debug)]
pub struct BinaryServer {
    handlers: HashMap<String, Handler>,
}

impl BinaryServer {
    /// Creates a new empty server with no registered handlers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use opencore_jsonrpc_rust::server::BinaryServer;
    ///
    /// let server = BinaryServer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registers a handler function for a specific action name.
    ///
    /// When a request with the given action is received, the corresponding
    /// handler will be invoked with the request parameters.
    ///
    /// # Arguments
    ///
    /// * `action` - The action name to register (e.g., "sum", "multiply")
    /// * `handler` - The handler function to invoke for this action
    ///
    /// # Example
    ///
    /// ```rust
    /// use opencore_jsonrpc_rust::server::BinaryServer;
    /// use serde_json::Value;
    ///
    /// fn echo(params: Vec<Value>) -> Result<Value, String> {
    ///     params.first()
    ///         .cloned()
    ///         .ok_or_else(|| "No parameter provided".into())
    /// }
    ///
    /// let mut server = BinaryServer::new();
    /// server.register("echo", echo);
    /// ```
    pub fn register(&mut self, action: &str, handler: Handler) {
        self.handlers.insert(action.to_string(), handler);
    }

    /// Starts the server loop, reading from stdin and writing to stdout.
    ///
    /// This method blocks indefinitely, processing requests as they arrive.
    /// Each line from stdin should be a valid JSON request. Responses are
    /// written as JSON lines to stdout.
    ///
    /// # Panics
    ///
    /// This method will not panic under normal circumstances. I/O errors
    /// are handled gracefully by continuing to the next request.
    pub fn run(&self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            let response = match serde_json::from_str::<Request>(&line) {
                Ok(req) => self.handle_request(req),
                Err(err) => {
                    let raw = format!(r#"{{"status":"error","error":"invalid json: {}"}}"#, err);
                    let _ = writeln!(stdout, "{}", raw);
                    let _ = stdout.flush();
                    continue;
                }
            };

            match serde_json::to_string(&response) {
                Ok(out) => {
                    let _ = writeln!(stdout, "{}", out);
                    let _ = stdout.flush();
                }
                Err(err) => {
                    let raw = format!(
                        r#"{{"status":"error","error":"serialization failed: {}"}}"#,
                        err
                    );
                    let _ = writeln!(stdout, "{}", raw);
                    let _ = stdout.flush();
                }
            }
        }
    }

    /// Handles a single request by dispatching to the appropriate handler.
    ///
    /// # Arguments
    ///
    /// * `req` - The parsed request to handle
    ///
    /// # Returns
    ///
    /// A response indicating success or failure
    fn handle_request(&self, req: Request) -> Response {
        match self.handlers.get(&req.action) {
            Some(handler) => match handler(req.params) {
                Ok(result) => Response::Ok { id: req.id, result },
                Err(msg) => Response::Error {
                    id: req.id,
                    error: msg,
                },
            },
            None => Response::Error {
                id: req.id,
                error: format!("unknown action: {}", req.action),
            },
        }
    }
}

impl Default for BinaryServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_handler_success(params: Vec<Value>) -> Result<Value, String> {
        Ok(json!({"received": params.len()}))
    }

    fn test_handler_error(_params: Vec<Value>) -> Result<Value, String> {
        Err("intentional error".to_string())
    }

    fn add_handler(params: Vec<Value>) -> Result<Value, String> {
        if params.len() != 2 {
            return Err("Expected 2 parameters".into());
        }
        let a = params[0].as_i64().ok_or("Invalid number")?;
        let b = params[1].as_i64().ok_or("Invalid number")?;
        Ok(Value::from(a + b))
    }

    #[test]
    fn test_server_creation() {
        let server = BinaryServer::new();
        assert_eq!(server.handlers.len(), 0);
    }

    #[test]
    fn test_server_default() {
        let server = BinaryServer::default();
        assert_eq!(server.handlers.len(), 0);
    }

    #[test]
    fn test_register_handler() {
        let mut server = BinaryServer::new();
        server.register("test", test_handler_success);
        assert_eq!(server.handlers.len(), 1);
        assert!(server.handlers.contains_key("test"));
    }

    #[test]
    fn test_register_multiple_handlers() {
        let mut server = BinaryServer::new();
        server.register("handler1", test_handler_success);
        server.register("handler2", test_handler_error);
        assert_eq!(server.handlers.len(), 2);
    }

    #[test]
    fn test_handle_request_success() {
        let mut server = BinaryServer::new();
        server.register("test", test_handler_success);

        let request = Request {
            id: "req-1".to_string(),
            action: "test".to_string(),
            params: vec![json!(1), json!(2)],
        };

        let response = server.handle_request(request);

        match response {
            Response::Ok { id, result } => {
                assert_eq!(id, "req-1");
                assert_eq!(result, json!({"received": 2}));
            }
            _ => panic!("Expected Ok response"),
        }
    }

    #[test]
    fn test_handle_request_handler_error() {
        let mut server = BinaryServer::new();
        server.register("test", test_handler_error);

        let request = Request {
            id: "req-2".to_string(),
            action: "test".to_string(),
            params: vec![],
        };

        let response = server.handle_request(request);

        match response {
            Response::Error { id, error } => {
                assert_eq!(id, "req-2");
                assert_eq!(error, "intentional error");
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_handle_request_unknown_action() {
        let server = BinaryServer::new();

        let request = Request {
            id: "req-3".to_string(),
            action: "nonexistent".to_string(),
            params: vec![],
        };

        let response = server.handle_request(request);

        match response {
            Response::Error { id, error } => {
                assert_eq!(id, "req-3");
                assert_eq!(error, "unknown action: nonexistent");
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_add_handler_success() {
        let mut server = BinaryServer::new();
        server.register("add", add_handler);

        let request = Request {
            id: "req-4".to_string(),
            action: "add".to_string(),
            params: vec![json!(5), json!(10)],
        };

        let response = server.handle_request(request);

        match response {
            Response::Ok { id, result } => {
                assert_eq!(id, "req-4");
                assert_eq!(result, json!(15));
            }
            _ => panic!("Expected Ok response"),
        }
    }

    #[test]
    fn test_add_handler_wrong_param_count() {
        let mut server = BinaryServer::new();
        server.register("add", add_handler);

        let request = Request {
            id: "req-5".to_string(),
            action: "add".to_string(),
            params: vec![json!(5)],
        };

        let response = server.handle_request(request);

        match response {
            Response::Error { id, error } => {
                assert_eq!(id, "req-5");
                assert_eq!(error, "Expected 2 parameters");
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_add_handler_invalid_type() {
        let mut server = BinaryServer::new();
        server.register("add", add_handler);

        let request = Request {
            id: "req-6".to_string(),
            action: "add".to_string(),
            params: vec![json!("not a number"), json!(10)],
        };

        let response = server.handle_request(request);

        match response {
            Response::Error { id, error } => {
                assert_eq!(id, "req-6");
                assert_eq!(error, "Invalid number");
            }
            _ => panic!("Expected Error response"),
        }
    }
}
