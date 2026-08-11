//! Pure tool approval policy. The TUI or host supplies the interactive answer
//! through hooks; this module only classifies the safe default boundary.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    Deny,
    #[default]
    Ask,
    Auto,
    Yolo,
}

macro_rules! approval_mode_wire_names {
    ($(($variant:ident, $wire:literal)),+ $(,)?) => {
        impl ApprovalMode {
            pub const fn wire_name(self) -> &'static str {
                match self { $(Self::$variant => $wire,)+ }
            }

            pub fn from_wire_name(name: &str) -> Option<Self> {
                match name { $($wire => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}

approval_mode_wire_names! {
    (Deny, "deny"),
    (Ask, "ask"),
    (Auto, "auto"),
    (Yolo, "yolo"),
}

impl From<crate::command_actor::ApprovalMode> for ApprovalMode {
    fn from(mode: crate::command_actor::ApprovalMode) -> Self {
        match mode {
            crate::command_actor::ApprovalMode::Ask => Self::Ask,
            crate::command_actor::ApprovalMode::Deny => Self::Deny,
            crate::command_actor::ApprovalMode::Auto => Self::Auto,
            crate::command_actor::ApprovalMode::Always => Self::Yolo,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApprovalModeEvent {
    Set(ApprovalMode),
}

pub fn reduce_approval_mode(mode: &mut ApprovalMode, event: ApprovalModeEvent) {
    let ApprovalModeEvent::Set(next) = event;
    *mode = next;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolClass {
    ReadOnly,
    Mutating,
    Unknown,
}

macro_rules! tool_class_table {
    ($( $class:ident => [$($name:literal),* $(,)?] ),+ $(,)?) => {
        fn classify_tool(tool: &str) -> ToolClass {
            match tool {
                $( $( $name => ToolClass::$class, )* )+
                _ => ToolClass::Unknown,
            }
        }
    };
}

tool_class_table! {
    ReadOnly => ["read", "grep", "glob", "list_dir", "echo"],
    Mutating => ["write", "edit", "bash", "shell", "exec", "run", "git_commit", "git_push", "git_revert"],
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
        let _ = self.apply(ApprovalModeEvent::Set(mode));
    }

    pub fn apply(&self, event: ApprovalModeEvent) -> bool {
        let mut mode = self.current();
        reduce_approval_mode(&mut mode, event);
        self.tx.send(mode).is_ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApprovalDecision {
    Allow,
    Deny { reason: String },
    Ask { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApprovalTrace {
    pub tool: String,
    pub mode: ApprovalMode,
    pub decision: ApprovalDecision,
}

impl ApprovalTrace {
    pub fn terminal_line(&self) -> String {
        let decision = match &self.decision {
            ApprovalDecision::Allow => "allow".to_owned(),
            ApprovalDecision::Deny { reason } => format!("deny ({reason})"),
            ApprovalDecision::Ask { reason } => format!("ask ({reason})"),
        };
        format!("approval {}: {} [{:?}]", self.tool, decision, self.mode)
    }
}

const MAX_APPROVAL_TRACES: usize = 128;

pub fn record_approval_trace(traces: &mut Vec<ApprovalTrace>, trace: ApprovalTrace) {
    traces.push(trace);
    if traces.len() > MAX_APPROVAL_TRACES {
        traces.remove(0);
    }
}

pub fn decide(mode: ApprovalMode, tool: &str) -> ApprovalDecision {
    decide_registered(mode, tool, false)
}

pub fn decide_registered(mode: ApprovalMode, tool: &str, registered: bool) -> ApprovalDecision {
    let class = classify_tool(tool);
    if matches!(mode, ApprovalMode::Auto | ApprovalMode::Yolo)
        || class == ToolClass::ReadOnly
        || (registered && mode != ApprovalMode::Deny)
    {
        ApprovalDecision::Allow
    } else if mode == ApprovalMode::Deny {
        ApprovalDecision::Deny {
            reason: "Tool execution is disabled by the current approval mode".into(),
        }
    } else {
        ApprovalDecision::Ask {
            reason: if class == ToolClass::Mutating {
                "This tool can change files or execute a process"
            } else if registered {
                "This registered tool can change files or execute a process"
            } else {
                "This tool is not in the trusted tool policy"
            }
            .into(),
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
    fn tool_classification_is_one_declarative_state_table() {
        assert_eq!(classify_tool("read"), ToolClass::ReadOnly);
        assert_eq!(classify_tool("git_push"), ToolClass::Mutating);
        assert_eq!(classify_tool("plugin__demo__inspect"), ToolClass::Unknown);
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
    fn unknown_tools_require_explicit_approval() {
        assert!(matches!(
            decide(ApprovalMode::Ask, "plugin__demo__inspect"),
            ApprovalDecision::Ask { reason } if reason.contains("trusted")
        ));
    }

    #[test]
    fn registered_tools_keep_their_explicit_mutation_classification() {
        assert_eq!(
            decide_registered(ApprovalMode::Ask, "plugin__demo__inspect", true),
            ApprovalDecision::Allow
        );
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

    #[test]
    fn approval_mode_events_replay_as_pure_state_data() {
        let mut mode = ApprovalMode::Ask;
        for event in [
            ApprovalModeEvent::Set(ApprovalMode::Auto),
            ApprovalModeEvent::Set(ApprovalMode::Yolo),
        ] {
            reduce_approval_mode(&mut mode, event);
        }
        assert_eq!(mode, ApprovalMode::Yolo);
    }

    #[test]
    fn approval_modes_round_trip_through_wire_names() {
        for mode in [
            ApprovalMode::Deny,
            ApprovalMode::Ask,
            ApprovalMode::Auto,
            ApprovalMode::Yolo,
        ] {
            assert_eq!(ApprovalMode::from_wire_name(mode.wire_name()), Some(mode));
        }
        assert_eq!(ApprovalMode::from_wire_name("unknown"), None);
    }

    #[test]
    fn command_approval_modes_lower_into_tool_policy_once() {
        assert_eq!(
            ApprovalMode::from(crate::command_actor::ApprovalMode::Always),
            ApprovalMode::Yolo
        );
        assert_eq!(
            ApprovalMode::from(crate::command_actor::ApprovalMode::Ask),
            ApprovalMode::Ask
        );
    }

    #[test]
    fn approval_traces_are_bounded_replayable_data() {
        let mut traces = Vec::new();
        for index in 0..=MAX_APPROVAL_TRACES {
            record_approval_trace(
                &mut traces,
                ApprovalTrace {
                    tool: format!("tool-{index}"),
                    mode: ApprovalMode::Ask,
                    decision: ApprovalDecision::Allow,
                },
            );
        }
        assert_eq!(traces.len(), MAX_APPROVAL_TRACES);
        assert_eq!(traces[0].tool, "tool-1");
    }

    #[test]
    fn approval_trace_projects_decision_and_reason_for_terminal_hosts() {
        let trace = ApprovalTrace {
            tool: "write".into(),
            mode: ApprovalMode::Ask,
            decision: ApprovalDecision::Ask {
                reason: "changes files".into(),
            },
        };
        assert_eq!(
            trace.terminal_line(),
            "approval write: ask (changes files) [Ask]"
        );
    }
}
