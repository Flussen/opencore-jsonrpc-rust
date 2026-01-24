//! JSON-RPC protocol types for request/response communication.
//!
//! This module defines the core protocol structures used for communication
//! between the TypeScript framework and Rust applications.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON-RPC request from the client.
///
/// # Fields
///
/// * `id` - Unique identifier for the request, used to match responses
/// * `action` - The name of the action/method to invoke
/// * `params` - Array of JSON values representing the parameters
///
/// # Example JSON
///
/// ```json
/// {
///   "id": "req-123",
///   "action": "sum",
///   "params": [5, 10]
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Request {
    pub id: String,
    pub action: String,
    pub params: Vec<Value>,
}

/// A JSON-RPC response to the client.
///
/// Responses are tagged with a `status` field that indicates success or failure.
///
/// # Variants
///
/// * `Ok` - Successful response with a result value
/// * `Error` - Error response with an error message
///
/// # Example JSON (Success)
///
/// ```json
/// {
///   "status": "ok",
///   "id": "req-123",
///   "result": 15
/// }
/// ```
///
/// # Example JSON (Error)
///
/// ```json
/// {
///   "status": "error",
///   "id": "req-123",
///   "error": "Invalid parameters"
/// }
/// ```
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(tag = "status")]
pub enum Response {
    /// Successful response containing the result
    #[serde(rename = "ok")]
    Ok {
        /// Request ID this response corresponds to
        id: String,
        /// The result value
        result: Value,
    },

    /// Error response containing an error message
    #[serde(rename = "error")]
    Error {
        /// Request ID this response corresponds to
        id: String,
        /// Human-readable error message
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_deserialization() {
        let json_str = r#"{"id":"req-123","action":"sum","params":[5,10]}"#;
        let req: Request = serde_json::from_str(json_str).unwrap();

        assert_eq!(req.id, "req-123");
        assert_eq!(req.action, "sum");
        assert_eq!(req.params.len(), 2);
        assert_eq!(req.params[0], json!(5));
        assert_eq!(req.params[1], json!(10));
    }

    #[test]
    fn test_request_with_empty_params() {
        let json_str = r#"{"id":"req-456","action":"ping","params":[]}"#;
        let req: Request = serde_json::from_str(json_str).unwrap();

        assert_eq!(req.id, "req-456");
        assert_eq!(req.action, "ping");
        assert_eq!(req.params.len(), 0);
    }

    #[test]
    fn test_response_ok_serialization() {
        let response = Response::Ok {
            id: "req-123".to_string(),
            result: json!(15),
        };

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["id"], "req-123");
        assert_eq!(parsed["result"], 15);
    }

    #[test]
    fn test_response_error_serialization() {
        let response = Response::Error {
            id: "req-456".to_string(),
            error: "Invalid parameters".to_string(),
        };

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["id"], "req-456");
        assert_eq!(parsed["error"], "Invalid parameters");
    }

    #[test]
    fn test_request_clone() {
        let req = Request {
            id: "req-789".to_string(),
            action: "test".to_string(),
            params: vec![json!(1), json!(2)],
        };

        let cloned = req.clone();
        assert_eq!(req, cloned);
    }

    #[test]
    fn test_response_clone() {
        let response = Response::Ok {
            id: "req-999".to_string(),
            result: json!("success"),
        };

        let cloned = response.clone();
        assert_eq!(response, cloned);
    }
}
