#!/bin/bash
# NOTE: This script is obsolete.
#
# Event taxonomy is now defined inline in `crates/runie-core/src/event/mod.rs`.
# The taxonomy.json file is kept as documentation only.
#
# To make changes:
# 1. Edit `crates/runie-core/src/event/mod.rs` directly.
# 2. Run `cargo test -p runie-core event::` to verify.
# 3. Update `taxonomy.json` to match if needed.

echo "This script is obsolete. Event taxonomy is now in crates/runie-core/src/event/mod.rs."
