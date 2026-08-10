//! Pure tool approval policy. The TUI or host supplies the interactive answer
//! through hooks; this module only classifies the safe default boundary.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApprovalMode {
    Deny,
    #[default]
    Ask,
    Auto,
    Yolo,
}

#[derive(Clone)]
pub struct ApprovalModeStore {
    tx: tokio::sync::watch::Sender<ApprovalMode>,
    rx: tokio::sync::watch::Receiver<ApprovalMode>,
}

impl Default for ApprovalModeStore {
    fn default() -> Self {
        let (tx, rx) = tokio::sync::watch::channel(ApprovalMode::Ask);
        Self { tx, rx }
    }
}

impl ApprovalModeStore {
    pub fn current(&self) -> ApprovalMode {
        *self.rx.borrow()
    }
    pub fn set(&self, mode: ApprovalMode) {
        let _ = self.tx.send(mode);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Allow,
    Deny { reason: &'static str },
    Ask { reason: &'static str },
}

pub fn decide(mode: ApprovalMode, tool: &str) -> ApprovalDecision {
    if matches!(mode, ApprovalMode::Auto | ApprovalMode::Yolo)
        || matches!(tool, "read" | "grep" | "glob" | "list_dir" | "echo")
        || !matches!(
            tool,
            "write" | "edit" | "bash" | "shell" | "exec" | "run" | "git_commit" | "git_push"
        )
    {
        ApprovalDecision::Allow
    } else if mode == ApprovalMode::Deny {
        ApprovalDecision::Deny {
            reason: "Tool execution is disabled by the current approval mode",
        }
    } else {
        ApprovalDecision::Ask {
            reason: "This tool can change files or execute a process",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_tools_are_safe_but_mutations_ask() {
        assert_eq!(decide(ApprovalMode::Ask, "read"), ApprovalDecision::Allow);
        assert!(matches!(
            decide(ApprovalMode::Ask, "write"),
            ApprovalDecision::Ask { .. }
        ));
        assert!(matches!(
            decide(ApprovalMode::Ask, "bash"),
            ApprovalDecision::Ask { .. }
        ));
    }

    #[test]
    fn auto_and_yolo_allow_mutating_tools() {
        for mode in [ApprovalMode::Auto, ApprovalMode::Yolo] {
            assert_eq!(decide(mode, "write"), ApprovalDecision::Allow);
            assert_eq!(decide(mode, "bash"), ApprovalDecision::Allow);
        }
    }

    #[test]
    fn deny_blocks_mutations_but_keeps_reads_safe() {
        assert_eq!(decide(ApprovalMode::Deny, "read"), ApprovalDecision::Allow);
        assert!(matches!(
            decide(ApprovalMode::Deny, "bash"),
            ApprovalDecision::Deny { .. }
        ));
    }

    #[test]
    fn mode_store_projects_changes_without_shared_mutation() {
        let store = ApprovalModeStore::default();
        let reader = store.clone();
        assert_eq!(reader.current(), ApprovalMode::Ask);
        store.set(ApprovalMode::Auto);
        assert_eq!(reader.current(), ApprovalMode::Auto);
        store.set(ApprovalMode::Yolo);
        assert_eq!(reader.current(), ApprovalMode::Yolo);
    }
}
