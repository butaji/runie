//! Tests for the leader actor.

use crate::actors::leader::{Leader, LeaderAgentHandle, LeaderConfig, LeaderHandle, LeaderStatus};
use crate::Event as CoreEvent;
use crate::bus::EventBus;

/// Layer 2: Verify `Leader::new()` defaults to embedded mode (no TCP).
#[test]
fn leader_default_embedded_no_tcp() {
    let leader = Leader::new();
    assert!(leader.config.tcp_addr.is_none(), "Leader::new() must default to embedded mode");
    assert!(
        !leader.config.project_root.as_os_str().is_empty(),
        "project_root should be set from current directory"
    );
}

/// Layer 1: Compile-time check that `LeaderHandle` exposes all required actor ref fields.
#[test]
fn leader_handle_exposes_all_actor_refs() {
    fn _check_types() {
        fn _field<T>(_: &T) {}
        let handle: LeaderHandle = unimplemented!();
        _field(&handle.config);
        _field(&handle.provider);
        _field(&handle.io);
        _field(&handle.session);
        _field(&handle.permission);
        _field(&handle.turn);
        _field(&handle.input);
        _field(&handle.agent);
        _field(&handle.fff_indexer);
        // snapshot_rx is also exposed for render-path tests.
        _field(&handle.snapshot_rx);
    }
}

/// Layer 2: `Leader::new()` returns a config with correct defaults.
#[test]
fn leader_config_defaults() {
    let cfg = LeaderConfig::default();
    assert!(cfg.tcp_addr.is_none());
    // Data dir must be non-empty (from dirs crate).
    assert!(!cfg.data_dir.as_os_str().is_empty());
}

/// Layer 2: `with_tcp_addr` configures the TCP listener path.
#[test]
fn leader_config_with_tcp() {
    let cfg = LeaderConfig::default().with_tcp_addr("0.0.0.0:9001");
    assert_eq!(cfg.tcp_addr.as_deref(), Some("0.0.0.0:9001"));

    let leader = Leader::new().with_tcp_addr("0.0.0.0:9002");
    assert_eq!(leader.config.tcp_addr.as_deref(), Some("0.0.0.0:9002"));
}

/// Layer 1: Default `LeaderStatus` reflects a stopped leader.
#[test]
fn leader_status_default() {
    let status = LeaderStatus::default();
    assert!(!status.running);
    assert_eq!(status.actor_count, 0);
}

/// Layer 1: `LeaderCommand` variants implement `Debug`.
#[test]
fn leader_command_debug() {
    use crate::actors::leader::LeaderCommand;
    format!("{:?}", LeaderCommand::Status);
    format!("{:?}", LeaderCommand::Shutdown);
    format!("{:?}", LeaderCommand::ForceAbort);
}
