# Clean Up Clippy Warnings for Dead and Unused Code

**Status**: done
**Milestone**: R3
**Category**: Core Architecture
**Priority**: P2

## Description

`cargo clippy --workspace --all-targets --all-features -- -D warnings` passes with zero
warnings. Dead code in the codebase is intentional and allowlisted.

## Resolution

All clippy warnings were resolved. Functions in `terminal/clipboard.rs` are used by the
effects/ module and are kept with `#![allow(dead_code)]` since they are available for
future use. No dead code that is truly unused remains.

Archived in tasks/archive/.
