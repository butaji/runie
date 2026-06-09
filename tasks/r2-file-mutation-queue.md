# File Mutation Queue

**Status**: done
**Milestone**: R2
**Category**: Tools

## Description

Serialize file edits to avoid conflicts. Queue mutations and apply sequentially with validation.

## Architecture

```rust
pub struct FileMutationQueue {
    pending: VecDeque<Mutation>,
}

pub enum Mutation {
    Write { path: PathBuf, content: String },
    Edit { path: PathBuf, old: String, new: String },
}

impl FileMutationQueue {
    pub fn enqueue(&mut self, mutation: Mutation);
    pub fn flush(&mut self) -> Vec<MutationResult>;
}

pub struct MutationResult {
    pub path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}
```

## Acceptance Criteria

- [ ] Mutations queued instead of immediate execution
- [ ] Queue flushed sequentially
- [ ] Each mutation validated before application
- [ ] Failed mutation stops queue
- [ ] Results reported to agent

## Tests

### Layer 1
- [ ] `queue_executes_in_order` — FIFO
- [ ] `failed_mutation_stops` — error halts processing
- [ ] `validation_catches_invalid` — unique match check
