//! OAuth / API-key authentication storage using OS keyring.
//!
//! Tokens are stored in the OS keychain/keyring (macOS Keychain, Linux Secret Service,
//! Windows Credential Manager) with fallback to `.runie/auth.json` for CI/headless.

#[cfg(feature = "keyring")]
pub mod keyring;
pub mod storage;
pub mod store_trait;

pub use credential::CredentialResolver;
#[cfg(feature = "keyring")]
pub use keyring::{
    delete_keyring_entry, get_keyring, load_all_from_keyring, migrate_legacy_auth, set_and_verify_keyring, set_keyring,
    set_keyring_value,
};
pub use storage::{persist_provider_api_key, AuthStorage, AuthToken};
#[cfg(feature = "keyring")]
pub use store_trait::OsKeyringStore;
pub use store_trait::{KeyringStore, MockKeyringStore};

pub(crate) mod credential;
