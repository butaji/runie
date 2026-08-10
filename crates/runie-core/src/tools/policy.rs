//! Pure tool approval policy. The TUI or host supplies the interactive answer
//! through hooks; this module only classifies the safe default boundary.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApprovalMode {
    #[default]
    Ask,
    Auto,
    Yolo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Allow,
    Ask { reason: &'static str },
}

pub fn decide(mode: ApprovalMode, tool: &str) -> ApprovalDecision {
    if matches!(mode, ApprovalMode::Auto | ApprovalMode::Yolo)
        || matches!(tool, "read" | "grep" | "glob" | "list_dir" | "echo")
        || !matches!(tool, "write" | "edit" | "bash" | "shell" | "exec" | "run")
    {
        ApprovalDecision::Allow
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
}
