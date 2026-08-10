macro_rules! declare_scrollback_domains {
    ($($domain:ident => ($($pattern:pat_param)|+)),+ $(,)?) => {
        impl ScrollbackMsg {
            pub const fn domain(&self) -> ScrollbackDomain {
                match self {
                    $($( $pattern )|+ => ScrollbackDomain::$domain,)+
                    _ => ScrollbackDomain::Navigation,
                }
            }
        }
    };
}

declare_scrollback_domains! {
    Lifecycle => (
        ScrollbackMsg::TurnStart | ScrollbackMsg::TurnEnd
            | ScrollbackMsg::AssistantStreamStart | ScrollbackMsg::AssistantStreamEnd
            | ScrollbackMsg::Clear | ScrollbackMsg::FinalizeAssistant { .. }
    ),
    Content => (
        ScrollbackMsg::Append(_) | ScrollbackMsg::AppendTurnSummary(_)
            | ScrollbackMsg::SetTheme(_) | ScrollbackMsg::AdvanceAnimation
            | ScrollbackMsg::RemoveKind(_) | ScrollbackMsg::NormalizeLiveCompletedAssistants
            | ScrollbackMsg::AddLiveAssistantTimestamp(_) | ScrollbackMsg::RemoveEmptyAfter(_)
            | ScrollbackMsg::NormalizeActivitySpacing | ScrollbackMsg::SetReasoningExpanded(_)
            | ScrollbackMsg::SetActivityExpanded(_) | ScrollbackMsg::ToggleActivityExpanded
            | ScrollbackMsg::SetPromptTimestamp(_) | ScrollbackMsg::SetFollowLatestUser(_)
    ),
    Tool => (
        ScrollbackMsg::SetToolName(_, _) | ScrollbackMsg::SetToolArgs(_, _)
            | ScrollbackMsg::RemoveToolArgs(_) | ScrollbackMsg::ActivityReset
            | ScrollbackMsg::ActivityToolStart(_) | ScrollbackMsg::ActivityToolEnd { .. }
            | ScrollbackMsg::SetToolMode(_, _) | ScrollbackMsg::ToggleToolMode(_)
            | ScrollbackMsg::MarkToolError(_) | ScrollbackMsg::ReplaceLine(_, _)
            | ScrollbackMsg::ReplaceLastByKind(_, _) | ScrollbackMsg::AppendToLastByKind(_, _)
            | ScrollbackMsg::ToolStart { .. } | ScrollbackMsg::ToolStartRunning { .. }
            | ScrollbackMsg::ToolUpdate { .. } | ScrollbackMsg::ToolEnd { .. }
    ),
    Workflow => (
        ScrollbackMsg::WorkflowStart { .. } | ScrollbackMsg::WorkflowProgress { .. }
            | ScrollbackMsg::WorkflowEnd { .. }
    ),
}
