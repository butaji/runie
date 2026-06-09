# Session Tree (/fork, /clone, /tree)

**Status**: todo
**Milestone**: R3
**Category**: Sessions

## Description

Session branching — fork from any message, clone current position, navigate tree.

## Architecture

```rust
pub struct SessionTree {
    pub root: TreeNode,
    pub current_branch: Vec<usize>,  // path from root to current
}

pub struct TreeNode {
    pub message: ChatMessage,
    pub children: Vec<TreeNode>,
}

pub enum DialogState {
    // ... other ...
    SessionTree {
        filter: SessionTreeFilter,
        selected: Vec<usize>,  // selected node path
    },
}

#[derive(Clone, Copy)]
pub enum SessionTreeFilter {
    All,
    NoTools,
    UserOnly,
    LabeledOnly,
}
```

### Events

```rust
Event::ForkSession { message_index: usize },
Event::CloneSession,
Event::ToggleSessionTree,
Event::SessionTreeFilterCycle,
```

## Acceptance Criteria

- [ ] `/fork` creates new branch from selected user message
- [ ] `/clone` duplicates current session at current position
- [ ] `/tree` opens tree navigation dialog
- [ ] Tree shows all branches with fold/unfold
- [ ] Arrow keys navigate, Enter selects branch
- [ ] Filters: all, no-tools, user-only, labeled-only
- [ ] Tree persisted in session JSON

## Tests

### Layer 1
- [ ] `fork_creates_branch` — new child node added
- [ ] `clone_duplicates_position` — messages up to index copied
- [ ] `tree_filter_excludes_tools` — no-tools filter hides tool messages

### Layer 2
- [ ] `slash_fork_emits_event` — /fork emits ForkSession
