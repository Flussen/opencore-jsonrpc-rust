use opencore_jsonrpc_rust::server::BinaryServer;
use serde_json::Value;

fn sum(params: Vec<Value>) -> Result<Value, String> {
    if params.len() != 2 {
        return Err("expected 2 parameters".into());
    }

    let a = params[0].as_i64().ok_or("invalid number")?;
    let b = params[1].as_i64().ok_or("invalid number")?;

    Ok(Value::from(a + b))
}

fn main() {
    let mut server = BinaryServer::new();
    server.register("sum", sum);
    server
        .emit_event(
            "worker.ready",
            serde_json::json!({ "pid": std::process::id() }),
        )
        .unwrap();
    server.run();
}
