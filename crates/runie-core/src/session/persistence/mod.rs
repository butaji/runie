//! Persistence layer for unified session storage.

mod header;
mod lock;

pub use header::{read_header, touch_header, write_header, SessionHeader};
pub use lock::{exclusive_lock, shared_lock, ExclusiveLock, SharedLock};
