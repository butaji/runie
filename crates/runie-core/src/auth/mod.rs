//! OAuth / API-key authentication storage using OS keyring.
//!
//! Tokens are stored in the OS keychain/keyring (macOS Keychain, Linux Secret Service,
//! Windows Credential Manager) with fallback to `.runie/auth.json` for CI/headless.

pub mod keyring;
pub mod storage;

pub use credential::CredentialResolver;
pub use keyring::{
    delete_keyring_entry, get_keyring, load_all_from_keyring, migrate_legacy_auth,
    set_and_verify_keyring, set_keyring, set_keyring_value,
};
pub use storage::{AuthStorage, AuthToken, Token};

mod credential;
