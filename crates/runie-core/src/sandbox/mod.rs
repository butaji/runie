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

// All sandbox modules are macOS-only. Compiled out entirely on other platforms.
#[cfg(target_os = "macos")]
mod deny;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
mod manager;
#[cfg(target_os = "macos")]
mod profiles;

#[cfg(target_os = "macos")]
pub use deny::{DenyList, ReadDenyList, WriteDenyList};
#[cfg(target_os = "macos")]
pub use manager::{SandboxManager, SandboxStatus};
#[cfg(target_os = "macos")]
pub use profiles::{resolve_profile, Profile, ProfileConfig, PROFILE_NAMES};

#[cfg(target_os = "macos")]
pub use crate::sandbox::macos::{
    build_mac_sandbox_profile, run_linux_sandboxed, run_mac_sandboxed, run_sandboxed,
    run_sandboxed_direct_async, run_sandboxed_shell_async, run_unsandboxed, sandbox_available,
};

/// Sandbox configuration.
#[cfg(target_os = "macos")]
pub use crate::sandbox::profiles::SandboxConfig;

#[cfg(target_os = "macos")]
pub use crate::sandbox::manager::SandboxStatus as SandboxAvailability;
