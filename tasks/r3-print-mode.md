# Print Mode

**Status**: done
**Milestone**: R3
**Category**: Modes

## Description

Non-interactive CLI output. Run a single prompt and print the response.

## Architecture

```rust
// Separate binary: runie-print
// Usage: runie-print "refactor this function"

#[tokio::main]
async fn main() {
    let prompt = std::env::args().nth(1).expect("prompt required");
    let config = Config::load();
    let provider = create_provider(&config).await;
    
    let messages = vec![
        Message::system(config.system_prompt()),
        Message::user(prompt),
    ];
    
    provider.generate(messages, |chunk| {
        print!("{}", chunk.content);
        io::stdout().flush().unwrap();
    }).await.unwrap();
    println!();
}
```

## Acceptance Criteria

- [x] `runie-print "prompt"` runs single turn
- [x] Streams output to stdout
- [x] No TUI, no ratatui dependency
- [x] Respects config.toml provider/model
- [x] Tool execution supported (with confirmation)
- [x] Exit code 0 on success, 1 on error

## Tests

### Layer 1
- [x] `print_mode_streams_output` — stdout receives chunks
- [x] `print_mode_respects_config` — provider from config.toml
