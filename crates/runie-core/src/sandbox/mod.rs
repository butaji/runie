//! OS-level sandboxing module.
//!
//! Provides platform-specific sandboxing for bash tool execution:
//! - **macOS**: Uses `sandbox-exec` with configurable profiles (Seatbelt)
//! - **Linux**: Uses the `landlock` crate for filesystem sandboxing
//! - **Windows**: Basic fallback with restricted environment
//!
//! ## Sandbox Profiles
//!
//! - `Off`: No sandboxing (full access)
//! - `Workspace`: Read all, write to workspace only
//! - `Strict`: Explicit allowlist, no network
//! - `Devbox`: Wide write access, minimal restrictions
//! - `Custom`: Loaded from `sandbox.toml`

mod deny;
mod macos;
mod manager;
mod profiles;

pub use deny::{DenyList, ReadDenyList, WriteDenyList};
pub use manager::{SandboxManager, SandboxStatus};
pub use profiles::{resolve_profile, Profile, ProfileConfig, PROFILE_NAMES};

pub use crate::sandbox::macos::{
    build_mac_sandbox_profile, run_linux_sandboxed, run_mac_sandboxed, run_sandboxed,
    run_sandboxed_direct_async, run_sandboxed_shell_async, run_unsandboxed, sandbox_available,
};

/// Sandbox configuration.
pub use crate::sandbox::profiles::SandboxConfig;

pub use crate::sandbox::manager::SandboxStatus as SandboxAvailability;
