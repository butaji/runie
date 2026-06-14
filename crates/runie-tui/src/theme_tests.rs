//! Theme caching tests.

use std::sync::Arc;

use crate::theme::{current_theme, set_current_theme, test_lock};

#[test]
fn theme_cache_returns_same_instance() {
    let _lock = test_lock();
    set_current_theme("runie");
    let first = current_theme();
    set_current_theme("runie");
    let second = current_theme();
    assert!(Arc::ptr_eq(&first, &second));
}
