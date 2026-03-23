//! Integration tests for the JSON-RPC library

use opencore_jsonrpc_rust::protocol::{Event, Request, Response};
use opencore_jsonrpc_rust::server::BinaryServer;
use serde_json::{json, Value};

fn multiply(params: Vec<Value>) -> Result<Value, String> {
    if params.len() != 2 {
        return Err("Expected 2 parameters".into());
    }
    let a = params[0].as_f64().ok_or("Invalid number")?;
    let b = params[1].as_f64().ok_or("Invalid number")?;
    Ok(Value::from(a * b))
}

fn echo(params: Vec<Value>) -> Result<Value, String> {
    params
        .first()
        .cloned()
        .ok_or_else(|| "No parameter provided".into())
}

fn concat(params: Vec<Value>) -> Result<Value, String> {
    let strings: Result<Vec<String>, String> = params
        .iter()
        .map(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "All parameters must be strings".to_string())
        })
        .collect();

    Ok(Value::from(strings?.join("")))
}

#[test]
fn test_full_request_response_cycle() {
    let mut server = BinaryServer::new();
    server.register("multiply", multiply);

    let request = Request {
        id: "test-1".to_string(),
        action: "multiply".to_string(),
        params: vec![json!(3.5), json!(2.0)],
    };

    let request_json = serde_json::to_string(&request).unwrap();
    let parsed_request: Request = serde_json::from_str(&request_json).unwrap();

    assert_eq!(parsed_request.id, "test-1");
    assert_eq!(parsed_request.action, "multiply");
}

#[test]
fn test_multiple_handlers() {
    let mut server = BinaryServer::new();
    server.register("multiply", multiply);
    server.register("echo", echo);
    server.register("concat", concat);

    let req1 = Request {
        id: "req-1".to_string(),
        action: "multiply".to_string(),
        params: vec![json!(4.0), json!(5.0)],
    };

    let req2 = Request {
        id: "req-2".to_string(),
        action: "echo".to_string(),
        params: vec![json!("hello")],
    };

    let req3 = Request {
        id: "req-3".to_string(),
        action: "concat".to_string(),
        params: vec![json!("Hello"), json!(" "), json!("World")],
    };

    let json1 = serde_json::to_string(&req1).unwrap();
    let json2 = serde_json::to_string(&req2).unwrap();
    let json3 = serde_json::to_string(&req3).unwrap();

    assert!(json1.contains("multiply"));
    assert!(json2.contains("echo"));
    assert!(json3.contains("concat"));
}

#[test]
fn test_response_serialization_roundtrip() {
    let response_ok = Response::Ok {
        id: "test-ok".to_string(),
        result: json!({"data": [1, 2, 3]}),
    };

    let json = serde_json::to_string(&response_ok).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["id"], "test-ok");
    assert_eq!(parsed["result"]["data"][0], 1);

    let response_error = Response::Error {
        id: "test-error".to_string(),
        error: "Something went wrong".to_string(),
    };

    let json = serde_json::to_string(&response_error).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["status"], "error");
    assert_eq!(parsed["id"], "test-error");
    assert_eq!(parsed["error"], "Something went wrong");
}

#[test]
fn test_complex_json_values() {
    let mut server = BinaryServer::new();
    server.register("echo", echo);

    let complex_value = json!({
        "nested": {
            "array": [1, 2, 3],
            "string": "test",
            "bool": true,
            "null": null
        }
    });

    let request = Request {
        id: "complex-1".to_string(),
        action: "echo".to_string(),
        params: vec![complex_value.clone()],
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: Request = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.params[0], complex_value);
}

#[test]
fn test_error_handling_in_handlers() {
    let mut server = BinaryServer::new();
    server.register("multiply", multiply);

    let request_wrong_count = Request {
        id: "err-1".to_string(),
        action: "multiply".to_string(),
        params: vec![json!(5.0)],
    };

    let request_wrong_type = Request {
        id: "err-2".to_string(),
        action: "multiply".to_string(),
        params: vec![json!("not a number"), json!(5.0)],
    };

    let json1 = serde_json::to_string(&request_wrong_count).unwrap();
    let json2 = serde_json::to_string(&request_wrong_type).unwrap();

    assert!(json1.contains("multiply"));
    assert!(json2.contains("multiply"));
}

#[test]
fn test_event_roundtrip() {
    let event = Event::new("worker.ready", Some(json!({"pid": 99})));

    let json = serde_json::to_string(&event).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["type"], "event");
    assert_eq!(parsed["event"], "worker.ready");
    assert_eq!(parsed["data"]["pid"], 99);
}
