//! Plan persistence helpers — bridges `PlanStore` with session save/load/fork.
//!
//! This module provides functions to:
//! - Save the active plan when entering plan mode
//! - Load the associated plan when loading a session
//! - Fork the plan when forking a session
//!
//! Plans are stored separately from sessions under `<sessions_dir>/plans/`.

use crate::session::plan_store::{Plan, PlanId, PlanStore};
use std::path::{Path, PathBuf};

/// Plans directory relative to sessions directory.
const PLANS_SUBDIR: &str = "plans";

/// Plans directory from a sessions directory.
pub fn plans_dir(sessions_dir: &Path) -> PathBuf {
    sessions_dir.join(PLANS_SUBDIR)
}

/// Get the default plans directory.
pub fn default_plans_dir() -> Option<PathBuf> {
    crate::session::store::SessionStore::default_store().map(|store| plans_dir(store.dir()))
}

/// Save a plan to the plan store and return its ID.
pub fn save_plan(
    plans_dir: &Path,
    session_id: &str,
    content: &str,
) -> std::io::Result<Option<String>> {
    let store = PlanStore::new(plans_dir.to_path_buf());
    let id = PlanId::new();
    let plan = Plan {
        id: id.clone(),
        content: content.to_owned(),
        session_id: session_id.to_owned(),
        created_at: chrono::Utc::now(),
    };
    store.save(&plan)?;
    Ok(Some(id.0.to_string()))
}

/// Load a plan by ID.
pub fn load_plan(plans_dir: &Path, plan_id: &str) -> std::io::Result<Option<Plan>> {
    let store = PlanStore::new(plans_dir.to_path_buf());
    let id = PlanId::from_uuid_str(plan_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid plan ID"))?;
    store.load(&id)
}

/// Fork a plan by ID and return the new plan ID.
pub fn fork_plan(plans_dir: &Path, plan_id: &str) -> std::io::Result<Option<String>> {
    let store = PlanStore::new(plans_dir.to_path_buf());
    let id = PlanId::from_uuid_str(plan_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid plan ID"))?;
    let new_id = store.fork(&id)?;
    Ok(new_id.map(|id| id.0.to_string()))
}

/// Delete a plan by ID.
pub fn delete_plan(plans_dir: &Path, plan_id: &str) -> std::io::Result<()> {
    let store = PlanStore::new(plans_dir.to_path_buf());
    let id = PlanId::from_uuid_str(plan_id)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid plan ID"))?;
    store.delete(&id)
}

/// Check if the plans directory exists and create it if needed.
pub fn ensure_plans_dir(plans_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(plans_dir)
}

/// Plans directory path for a given sessions directory.
pub fn resolve_plans_dir(sessions_dir: Option<&Path>) -> Option<PathBuf> {
    sessions_dir.map(plans_dir)
}

/// Load plan content from a plan ID.
pub fn load_plan_content(plans_dir: &Path, plan_id: &str) -> Option<String> {
    load_plan(plans_dir, plan_id)
        .ok()
        .flatten()
        .map(|p| p.content)
}

/// Plans store builder for tests.
#[cfg(test)]
pub fn test_store() -> (PlanStore, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let store = PlanStore::new(tmp.path().join("plans"));
    (store, tmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let plans_dir = tmp.path().join("plans");
        let plan_id = save_plan(&plans_dir, "session-1", "# Test Plan\n\nDo it.").unwrap();
        assert!(plan_id.is_some());
        let content = load_plan_content(&plans_dir, plan_id.as_ref().unwrap()).unwrap();
        assert!(content.contains("Test Plan"));
    }

    #[test]
    fn test_fork_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let plans_dir = tmp.path().join("plans");
        let original_id = save_plan(&plans_dir, "session-1", "# Original")
            .unwrap()
            .unwrap();
        let new_id = fork_plan(&plans_dir, &original_id).unwrap().unwrap();
        assert_ne!(original_id, new_id);
        // Both should have the same content
        let orig_content = load_plan_content(&plans_dir, &original_id).unwrap();
        let new_content = load_plan_content(&plans_dir, &new_id).unwrap();
        assert_eq!(orig_content, new_content);
    }

    #[test]
    fn test_delete_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let plans_dir = tmp.path().join("plans");
        let plan_id = save_plan(&plans_dir, "session-1", "delete me")
            .unwrap()
            .unwrap();
        delete_plan(&plans_dir, &plan_id).unwrap();
        assert!(load_plan_content(&plans_dir, &plan_id).is_none());
    }

    #[test]
    fn load_nonexistent_plan() {
        let tmp = tempfile::tempdir().unwrap();
        let plans_dir = tmp.path().join("plans");
        assert!(load_plan_content(&plans_dir, "nonexistent-uuid").is_none());
    }
}
