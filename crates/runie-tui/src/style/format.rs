//! Format string templates for consistent message rendering.

/// Collapsed thought bubble format: marker, content, duration, end marker
pub const THOUGHT_COLLAPSED_FMT: &str = "{} {} Thought for {:.1}s {}";

/// Expanded thought bubble format (same as collapsed, differs in content)
pub const THOUGHT_EXPANDED_FMT: &str = "{} {} Thought for {:.1}s {}";

/// System message prefix: 3 spaces + marker + space
pub const SYSTEM_PREFIX: &str = "   {} ";

/// Error prefix: 3 spaces + exclamation
pub const ERROR_PREFIX: &str = "   ! ";

/// User message chevron format: indent + chevron + space
pub const USER_CHEVRON: &str = "{} {} ";

/// Assistant message bullet format: 3 spaces + bullet + space
pub const ASSISTANT_BULLET_FMT: &str = "   {} ";

/// Tool running format: indent, name, status, duration
pub const TOOL_RUNNING_FMT: &str = "{} Run {} {} {:.1}s";

/// Tool complete format: indent, status, name, arrow, duration
pub const TOOL_COMPLETE_FMT: &str = "{} {} {} → {} {:.1}s";

/// Version badge format
pub const VERSION_BADGE_FMT: &str = "{} [stable] Beta";

/// Status separator between status items
pub const STATUS_SEPARATOR: &str = "  │  ";

/// Session tree node format
pub const TREE_NODE_FMT: &str = "{} {} ";

/// Permission request format
pub const PERMISSION_FMT: &str = "   {} {}";

/// Plan step pending format
pub const PLAN_STEP_PENDING_FMT: &str = "   ▸ {}";

/// Plan step complete format
pub const PLAN_STEP_COMPLETE_FMT: &str = "   ✓ {}";
