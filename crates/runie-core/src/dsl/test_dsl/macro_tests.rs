//! Layer 1 tests for the `runie-macros` DSL macros.

use runie_macros::{define_command, define_event, define_hook, define_policy, define_tool};

define_event!(TurnStarted {
    turn_id: u64,
    agent_id: String,
});

define_tool!(ReadFile, "read_file", "Read file contents", {
    path: String,
});

define_command!(list_files, "list files");

define_hook!(PreToolUse, "before_tool");

define_policy!(AllowRead, tool: "read_file", action: Allow);

#[cfg(test)]
mod tests {
    use crate::hooks::{HookDecision, HookHandler};
    use crate::permissions::{PermissionAction, PermissionRule};
    use crate::tool::runtime::ToolRuntime;
    use crate::tool::{ToolContext, ToolStatus};

    #[test]
    fn define_event_generates_correct_enum() {
        let event = super::TurnStarted {
            turn_id: 42,
            agent_id: "agent-1".into(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["turn_id"], 42);
        assert_eq!(json["agent_id"], "agent-1");
        assert_eq!(super::TurnStarted::event_name(), "TurnStarted");
    }

    #[tokio::test]
    async fn define_tool_generates_correct_impl() {
        let tool = super::ReadFile {
            path: "/tmp/foo".into(),
        };
        let ctx = ToolContext::default();
        let out = tool.run(&ctx).await.unwrap();
        assert_eq!(out.tool_name, "read_file");
        assert_eq!(out.content, "Read file contents");
        assert_eq!(out.status, ToolStatus::Success);
    }

    #[test]
    fn define_command_generates_handler() {
        let cmd = super::list_files();
        assert_eq!(cmd.name, "list_files");
        assert_eq!(cmd.desc, "list files");
    }

    #[test]
    fn define_hook_generates_closure() {
        let decision = super::PreToolUse.handle(&serde_json::json!({ "tool": "write_file" }));
        assert_eq!(decision, HookDecision::Allow);
    }

    #[test]
    fn define_policy_generates_policy() {
        let rule: PermissionRule = super::allow_read();
        assert!(rule.matches("read_file", None));
        assert!(!rule.matches("write_file", None));
        assert_eq!(rule.action, PermissionAction::Allow);
    }
}
