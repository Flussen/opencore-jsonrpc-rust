//! Binary JSON-RPC server implementation.
//!
//! This module provides a simple server that communicates via stdin/stdout
//! using line-delimited JSON messages, including fire-and-forget events.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;

use crate::protocol::{Event, Request, Response};

type SharedReader = Arc<Mutex<Box<dyn BufRead + Send>>>;
type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;

/// A handler function that processes JSON-RPC requests.
pub type Handler = fn(Vec<Value>) -> Result<Value, String>;

/// A cloneable writer for emitting unsolicited OpenCore binary events.
#[derive(Clone)]
pub struct EventEmitter {
    output: SharedWriter,
}

impl EventEmitter {
    /// Emits an event with a JSON-serializable payload.
    pub fn emit<T: Serialize>(&self, event: &str, data: T) -> io::Result<()> {
        let value = serde_json::to_value(data).map_err(serialization_error)?;
        self.emit_value(event, Some(value))
    }

    /// Emits an event without a payload.
    pub fn emit_empty(&self, event: &str) -> io::Result<()> {
        self.emit_value(event, None)
    }

    fn emit_value(&self, event: &str, data: Option<Value>) -> io::Result<()> {
        self.write_json_line(&Event::new(event, data))
    }

    fn write_json_line<T: Serialize>(&self, value: &T) -> io::Result<()> {
        let payload = serde_json::to_vec(value).map_err(serialization_error)?;
        let mut writer = self
            .output
            .lock()
            .map_err(|_| io::Error::other("output writer lock poisoned"))?;

        writer.write_all(&payload)?;
        writer.write_all(b"\n")?;
        writer.flush()
    }
}

/// A JSON-RPC server that communicates via stdin/stdout.
pub struct BinaryServer {
    handlers: HashMap<String, Handler>,
    input: SharedReader,
    emitter: EventEmitter,
}

impl BinaryServer {
    /// Creates a new empty server with stdin/stdout transport.
    pub fn new() -> Self {
        Self::with_io(BufReader::new(io::stdin()), io::stdout())
    }

    /// Creates a server with custom input and output streams.
    pub fn with_io<R, W>(input: R, output: W) -> Self
    where
        R: BufRead + Send + 'static,
        W: Write + Send + 'static,
    {
        let output: SharedWriter = Arc::new(Mutex::new(Box::new(output)));

        Self {
            handlers: HashMap::new(),
            input: Arc::new(Mutex::new(Box::new(input))),
            emitter: EventEmitter { output },
        }
    }

    /// Returns a cloneable event emitter tied to the server stdout stream.
    pub fn emitter(&self) -> EventEmitter {
        self.emitter.clone()
    }

    /// Registers a handler function for a specific action name.
    pub fn register(&mut self, action: &str, handler: Handler) {
        self.handlers.insert(action.to_string(), handler);
    }

    /// Emits an event with a JSON-serializable payload.
    pub fn emit_event<T: Serialize>(&self, event: &str, data: T) -> io::Result<()> {
        self.emitter.emit(event, data)
    }

    /// Emits an event without a payload.
    pub fn emit_event_empty(&self, event: &str) -> io::Result<()> {
        self.emitter.emit_empty(event)
    }

    /// Starts the server loop, reading from stdin and writing to stdout.
    pub fn run(&self) {
        let mut line = String::new();

        loop {
            line.clear();

            let bytes_read = {
                let mut reader = match self.input.lock() {
                    Ok(reader) => reader,
                    Err(_) => return,
                };

                match reader.read_line(&mut line) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                }
            };

            if bytes_read == 0 {
                break;
            }

            let trimmed = line.trim_end_matches(['\n', '\r']);
            if trimmed.is_empty() {
                continue;
            }

            let response = match serde_json::from_str::<Request>(trimmed) {
                Ok(req) => self.handle_request(req),
                Err(err) => {
                    let raw = format!(r#"{{"status":"error","error":"invalid json: {}"}}"#, err);
                    let _ = self.write_raw_line(&raw);
                    continue;
                }
            };

            if self.emitter.write_json_line(&response).is_err() {
                let raw = r#"{"status":"error","error":"serialization failed"}"#;
                let _ = self.write_raw_line(raw);
            }
        }
    }

    /// Handles a single request by dispatching to the appropriate handler.
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

    fn write_raw_line(&self, raw: &str) -> io::Result<()> {
        let mut writer = self
            .emitter
            .output
            .lock()
            .map_err(|_| io::Error::other("output writer lock poisoned"))?;
        writer.write_all(raw.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()
    }
}

impl Default for BinaryServer {
    fn default() -> Self {
        Self::new()
    }
}

fn serialization_error(err: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    #[derive(Clone, Default)]
    struct SharedBuffer {
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    impl SharedBuffer {
        fn into_string(&self) -> String {
            String::from_utf8(self.bytes.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.bytes.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

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
        let server = BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());
        assert_eq!(server.handlers.len(), 0);
    }

    #[test]
    fn test_server_default() {
        let server = BinaryServer::default();
        assert_eq!(server.handlers.len(), 0);
    }

    #[test]
    fn test_register_handler() {
        let mut server =
            BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());
        server.register("test", test_handler_success);
        assert_eq!(server.handlers.len(), 1);
        assert!(server.handlers.contains_key("test"));
    }

    #[test]
    fn test_register_multiple_handlers() {
        let mut server =
            BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());
        server.register("handler1", test_handler_success);
        server.register("handler2", test_handler_error);
        assert_eq!(server.handlers.len(), 2);
    }

    #[test]
    fn test_handle_request_success() {
        let mut server =
            BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());
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
        let mut server =
            BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());
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
        let server = BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());

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
        let mut server =
            BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());
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
        let mut server =
            BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());
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
        let mut server =
            BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), SharedBuffer::default());
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

    #[test]
    fn test_emit_event() {
        let output = SharedBuffer::default();
        let server = BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), output.clone());

        server
            .emit_event("worker.ready", json!({"pid": 42}))
            .unwrap();

        let line = output.into_string();
        let parsed: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed["type"], "event");
        assert_eq!(parsed["event"], "worker.ready");
        assert_eq!(parsed["data"]["pid"], 42);
    }

    #[test]
    fn test_emit_event_empty() {
        let output = SharedBuffer::default();
        let server = BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), output.clone());

        server.emit_event_empty("heartbeat").unwrap();

        let line = output.into_string();
        let parsed: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed["type"], "event");
        assert_eq!(parsed["event"], "heartbeat");
        assert!(parsed.get("data").is_none());
    }

    #[test]
    fn test_emitter_clone_writes_same_stream() {
        let output = SharedBuffer::default();
        let server = BinaryServer::with_io(Cursor::new(Vec::<u8>::new()), output.clone());
        let emitter = server.emitter();

        emitter.emit("worker.ready", json!({"pid": 7})).unwrap();
        server.emit_event_empty("heartbeat").unwrap();

        let content = output.into_string();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        let first: Value = serde_json::from_str(lines[0]).unwrap();
        let second: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(first["event"], "worker.ready");
        assert_eq!(second["event"], "heartbeat");
    }

    #[test]
    fn test_run_writes_event_and_response() {
        let output = SharedBuffer::default();
        let input =
            Cursor::new(b"{\"id\":\"req-1\",\"action\":\"sum\",\"params\":[2,3]}\n".to_vec());
        let mut server = BinaryServer::with_io(input, output.clone());

        server.register("sum", |params| {
            let a = params[0].as_i64().ok_or("Invalid number")?;
            let b = params[1].as_i64().ok_or("Invalid number")?;
            Ok(Value::from(a + b))
        });

        server.emit_event_empty("startup").unwrap();
        server.run();

        let content = output.into_string();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        let event: Value = serde_json::from_str(lines[0]).unwrap();
        let response: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(event["type"], "event");
        assert_eq!(event["event"], "startup");
        assert_eq!(response["status"], "ok");
        assert_eq!(response["result"], 5);
    }
}
