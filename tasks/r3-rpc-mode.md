# RPC / Server Mode

**Status**: todo
**Milestone**: R3
**Category**: Modes

## Description

Expose runie as an RPC server for IDE integration and programmatic access.

## Architecture

```rust
// Separate binary: runie-server
// Protocol: JSON-RPC 2.0 over stdin/stdout or TCP

pub enum RpcMethod {
    Initialize,
    Chat { messages: Vec<Message> },
    Complete { prompt: String },
    ListModels,
    ListSessions,
    LoadSession { name: String },
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    println!("{}", listener.local_addr().unwrap().port());
    
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
    }
}
```

## Acceptance Criteria

- [ ] `runie-server` starts RPC server
- [ ] JSON-RPC 2.0 protocol
- [ ] Methods: initialize, chat, complete, listModels, listSessions
- [ ] TCP or stdio transport
- [ ] SDK clients for common languages (TypeScript, Python)
- [ ] Graceful shutdown

## Tests

### Layer 1
- [ ] `rpc_parses_request` — valid JSON-RPC parsed
- [ ] `rpc_returns_response` — response is valid JSON-RPC
- [ ] `rpc_list_models` — returns model catalog
