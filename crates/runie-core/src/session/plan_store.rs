//! Plan file artifact storage.
//!
//! Plans are markdown files stored under `<sessions_dir>/plans/`.
//! Each plan has a unique ID and is associated with a session.
//!
//! File layout: `<sessions_dir>/plans/<plan_id>.md`
//!
//! Plan mode is a blocking mode where write tools require explicit approval.
//! Plans are persisted across sessions and copied on fork.

use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

/// Plan content + metadata.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Unique plan identifier.
    pub id: PlanId,
    /// Plan markdown content.
    pub content: String,
    /// Session ID this plan is associated with.
    pub session_id: String,
    /// When the plan was created.
    pub created_at: DateTime<Utc>,
}

/// Plan identifier — a UUID v4.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlanId(pub uuid::Uuid);

impl PlanId {
    /// Generate a new unique plan ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Parse a PlanId from a UUID string.
    pub fn from_uuid_str(s: &str) -> Option<Self> {
        uuid::Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for PlanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// JSON metadata stored alongside each plan file.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PlanMeta {
    id: String,
    session_id: String,
    created_at: String,
}

/// Plan file store.
#[derive(Debug, Clone)]
pub struct PlanStore {
    plans_dir: PathBuf,
}

impl PlanStore {
    /// Create a new store at the given plans directory.
    pub fn new(plans_dir: PathBuf) -> Self {
        Self { plans_dir }
    }

    /// Plans directory.
    pub fn plans_dir(&self) -> &PathBuf {
        &self.plans_dir
    }

    /// Path to a plan's markdown file.
    pub fn plan_path(&self, plan_id: &PlanId) -> PathBuf {
        self.plans_dir.join(format!("{}.md", plan_id.0))
    }

    /// Path to a plan's metadata JSON file.
    fn meta_path(&self, plan_id: &PlanId) -> PathBuf {
        self.plans_dir.join(format!("{}.meta.json", plan_id.0))
    }

    /// Ensure the plans directory exists.
    fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.plans_dir)
    }

    /// Save a plan to disk.
    ///
    /// Caller should run this in `spawn_blocking` for async contexts.
    pub fn save(&self, plan: &Plan) -> std::io::Result<()> {
        self.ensure_dir()?;
        let path = self.plan_path(&plan.id);
        fs::write(&path, &plan.content)?;
        let meta = PlanMeta {
            id: plan.id.0.to_string(),
            session_id: plan.session_id.clone(),
            created_at: plan.created_at.to_rfc3339(),
        };
        let meta_path = self.meta_path(&plan.id);
        fs::write(
            &meta_path,
            serde_json::to_string_pretty(&meta).unwrap_or_default(),
        )?;
        Ok(())
    }

    /// Load a plan from disk.
    ///
    /// Returns `None` if the plan does not exist.
    /// Caller should run this in `spawn_blocking` for async contexts.
    pub fn load(&self, plan_id: &PlanId) -> std::io::Result<Option<Plan>> {
        let path = self.plan_path(plan_id);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let meta: PlanMeta = if self.meta_path(plan_id).exists() {
            let meta_json = fs::read_to_string(self.meta_path(plan_id))?;
            serde_json::from_str(&meta_json).unwrap_or(PlanMeta {
                id: plan_id.0.to_string(),
                session_id: String::new(),
                created_at: Utc::now().to_rfc3339(),
            })
        } else {
            PlanMeta { id: plan_id.0.to_string(), session_id: String::new(), created_at: Utc::now().to_rfc3339() }
        };
        let created_at = DateTime::parse_from_rfc3339(&meta.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(Some(Plan {
            id: PlanId(uuid::Uuid::parse_str(&meta.id).unwrap_or_default()),
            content,
            session_id: meta.session_id,
            created_at,
        }))
    }

    /// Delete a plan from disk.
    ///
    /// Caller should run this in `spawn_blocking` for async contexts.
    pub fn delete(&self, plan_id: &PlanId) -> std::io::Result<()> {
        let path = self.plan_path(plan_id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        let meta_path = self.meta_path(plan_id);
        if meta_path.exists() {
            fs::remove_file(meta_path)?;
        }
        Ok(())
    }

    /// List all plans in the store.
    ///
    /// Returns plan IDs sorted by creation time (newest first).
    /// Caller should run this in `spawn_blocking` for async contexts.
    pub fn list(&self) -> std::io::Result<Vec<PlanId>> {
        self.ensure_dir()?;
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.plans_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".md") {
                if let Some(id_str) = name.strip_suffix(".md") {
                    if let Ok(uuid) = uuid::Uuid::parse_str(id_str) {
                        ids.push(PlanId(uuid));
                    }
                }
            }
        }
        ids.sort_by_key(|id| std::cmp::Reverse(id.0)); // newest first
        Ok(ids)
    }

    /// Copy a plan to a new plan file (used on session fork).
    ///
    /// Returns the new plan's ID.
    /// Caller should run this in `spawn_blocking` for async contexts.
    pub fn fork(&self, plan_id: &PlanId) -> std::io::Result<Option<PlanId>> {
        let Some(plan) = self.load(plan_id)? else {
            return Ok(None);
        };
        let new_id = PlanId::new();
        let new_plan =
            Plan { id: new_id.clone(), content: plan.content, session_id: plan.session_id, created_at: Utc::now() };
        self.save(&new_plan)?;
        Ok(Some(new_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn plan_store_round_trip() {
        let tmp = TempDir::new().unwrap();
        let store = PlanStore::new(tmp.path().join("plans"));
        let plan = Plan {
            id: PlanId::new(),
            content: "# My Plan\n\nDo the thing.".to_owned(),
            session_id: "test-session".to_owned(),
            created_at: Utc::now(),
        };
        store.save(&plan).unwrap();
        let loaded = store.load(&plan.id).unwrap().unwrap();
        assert_eq!(loaded.content, plan.content);
        assert_eq!(loaded.session_id, "test-session");
    }

    #[test]
    fn plan_store_list() {
        let tmp = TempDir::new().unwrap();
        let store = PlanStore::new(tmp.path().join("plans"));
        let p1 = Plan {
            id: PlanId::new(),
            content: "plan 1".to_owned(),
            session_id: "s1".to_owned(),
            created_at: Utc::now(),
        };
        let p2 = Plan {
            id: PlanId::new(),
            content: "plan 2".to_owned(),
            session_id: "s1".to_owned(),
            created_at: Utc::now(),
        };
        store.save(&p1).unwrap();
        store.save(&p2).unwrap();
        let ids = store.list().unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn plan_store_fork() {
        let tmp = TempDir::new().unwrap();
        let store = PlanStore::new(tmp.path().join("plans"));
        let original = Plan {
            id: PlanId::new(),
            content: "# Forked Plan".to_owned(),
            session_id: "orig".to_owned(),
            created_at: Utc::now(),
        };
        store.save(&original).unwrap();
        let new_id = store.fork(&original.id).unwrap().unwrap();
        assert_ne!(new_id.0, original.id.0);
        let loaded = store.load(&new_id).unwrap().unwrap();
        assert_eq!(loaded.content, "# Forked Plan");
    }

    #[test]
    fn plan_store_delete() {
        let tmp = TempDir::new().unwrap();
        let store = PlanStore::new(tmp.path().join("plans"));
        let plan = Plan {
            id: PlanId::new(),
            content: "delete me".to_owned(),
            session_id: "s1".to_owned(),
            created_at: Utc::now(),
        };
        store.save(&plan).unwrap();
        store.delete(&plan.id).unwrap();
        assert!(store.load(&plan.id).unwrap().is_none());
    }

    #[test]
    fn plan_store_nonexistent_returns_none() {
        let tmp = TempDir::new().unwrap();
        let store = PlanStore::new(tmp.path().join("plans"));
        assert!(store.load(&PlanId::new()).unwrap().is_none());
    }
}
