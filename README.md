# OpenCore JSON-RPC Rust

A simple and elegant library for creating JSON-RPC servers that communicate with TypeScript frameworks via stdin/stdout.

## Features

- **Simple API** - Register handlers with a clean, type-safe interface
- **Line-delimited JSON** - Easy integration with any language that can spawn processes
- **Automatic serialization** - Request/response handling is transparent
- **Robust error handling** - Clear error messages and graceful failure modes
- **Well-tested** - Comprehensive unit and integration tests
- **Fully documented** - Complete API documentation with examples

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
opencore-jsonrpc-rust = "0.1.0"
```

## Quick Start

```rust
use opencore_jsonrpc_rust::server::BinaryServer;
use serde_json::Value;

fn add(params: Vec<Value>) -> Result<Value, String> {
    if params.len() != 2 {
        return Err("Expected 2 parameters".into());
    }
    let a = params[0].as_i64().ok_or("Invalid number")?;
    let b = params[1].as_i64().ok_or("Invalid number")?;
    Ok(Value::from(a + b))
}

fn main() {
    let mut server = BinaryServer::new();
    server.register("add", add);
    server.run();
}
```

## Protocol

### Request Format

Requests are sent as line-delimited JSON to stdin:

```json
{"id": "req-123", "action": "add", "params": [5, 10]}
```

### Response Format

Responses are written as line-delimited JSON to stdout:

**Success:**
```json
{"status": "ok", "id": "req-123", "result": 15}
```

**Error:**
```json
{"status": "error", "id": "req-123", "error": "Invalid parameters"}
```

## Examples

### Basic Math Operations

```rust
use opencore_jsonrpc_rust::server::BinaryServer;
use serde_json::Value;

fn multiply(params: Vec<Value>) -> Result<Value, String> {
    if params.len() != 2 {
        return Err("Expected 2 parameters".into());
    }
    let a = params[0].as_f64().ok_or("Invalid number")?;
    let b = params[1].as_f64().ok_or("Invalid number")?;
    Ok(Value::from(a * b))
}

fn main() {
    let mut server = BinaryServer::new();
    server.register("multiply", multiply);
    server.run();
}
```

### String Operations

```rust
use opencore_jsonrpc_rust::server::BinaryServer;
use serde_json::Value;

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

fn main() {
    let mut server = BinaryServer::new();
    server.register("concat", concat);
    server.run();
}
```

### Complex Data Structures

```rust
use opencore_jsonrpc_rust::server::BinaryServer;
use serde_json::{json, Value};

fn process_user(params: Vec<Value>) -> Result<Value, String> {
    let user = params.first().ok_or("No user data provided")?;
    
    let name = user["name"].as_str().ok_or("Missing name")?;
    let age = user["age"].as_i64().ok_or("Missing age")?;
    
    Ok(json!({
        "message": format!("Hello, {}!", name),
        "is_adult": age >= 18
    }))
}

fn main() {
    let mut server = BinaryServer::new();
    server.register("process_user", process_user);
    server.run();
}
```

## TypeScript Integration

Here's how to use this library from TypeScript:

```typescript
import { spawn } from 'child_process';
import { randomUUID } from 'crypto';

class RustRPCClient {
  private process: any;
  private pending = new Map<string, any>();

  constructor(binaryPath: string) {
    this.process = spawn(binaryPath);
    
    this.process.stdout.on('data', (data: Buffer) => {
      const lines = data.toString().split('\n').filter(l => l.trim());
      for (const line of lines) {
        const response = JSON.parse(line);
        const resolver = this.pending.get(response.id);
        if (resolver) {
          this.pending.delete(response.id);
          if (response.status === 'ok') {
            resolver.resolve(response.result);
          } else {
            resolver.reject(new Error(response.error));
          }
        }
      }
    });
  }

  async call(action: string, params: any[]): Promise<any> {
    const id = randomUUID();
    const request = { id, action, params };
    
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.process.stdin.write(JSON.stringify(request) + '\n');
    });
  }

  close() {
    this.process.kill();
  }
}

// Usage
const client = new RustRPCClient('./target/release/my-server');
const result = await client.call('add', [5, 10]);
console.log(result); // 15
```

## Error Handling

Handlers should return `Result<Value, String>`:

- `Ok(value)` - Success with a JSON value
- `Err(message)` - Error with a descriptive message

The server automatically handles:
- Invalid JSON in requests
- Unknown actions
- Serialization errors
- I/O errors

## Testing

Run the test suite:

```bash
cargo test
```

Run with output:

```bash
cargo test -- --nocapture
```

## Documentation

Generate and view the full API documentation:

```bash
cargo doc --open
```

## License

MIT

## Contributing

Contributions are welcome! Please ensure:

1. All tests pass (`cargo test`)
2. Code is formatted (`cargo fmt`)
3. No clippy warnings (`cargo clippy`)
4. Documentation is updated
