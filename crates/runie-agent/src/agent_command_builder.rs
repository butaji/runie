//! Test builder for [`crate::AgentCommand`].
//!
//! Provides a fluent API to construct `AgentCommand` instances for tests with
//! sensible defaults for all fields.
//!
//! # Example
//!
//! ```ignore
//! use crate::tests::agent_cmd;
//!
//! let cmd = agent_cmd("hello").build();
//! let cmd = agent_cmd("list files").id("req.1").model("fast").build();
//! ```

/// Default provider used in the builder.
const DEFAULT_PROVIDER: &str = "mock";
/// Default model used in the builder.
const DEFAULT_MODEL: &str = "echo";

/// Builder for [`crate::AgentCommand`] with test-friendly defaults.
///
/// All fields have sensible defaults; only `content` is required.
/// The builder can be used via the free function [`agent_cmd`] or directly:
/// ```ignore
/// let cmd = AgentCommandBuilder::new("hello").build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct AgentCommandBuilder {
    content: String,
    id: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    thinking_level: Option<runie_core::model::ThinkingLevel>,
    read_only: Option<bool>,
    skills_context: Option<String>,
    system_prompt: Option<String>,
    truncation: Option<crate::truncate::TruncationPolicy>,
    cancellation_token: Option<tokio_util::sync::CancellationToken>,
}

impl AgentCommandBuilder {
    /// Create a new builder with the required `content` field.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            ..Default::default()
        }
    }

    /// Set the request id. Defaults to `"req.0"`.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the provider key. Defaults to `"mock"`.
    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set the model. Defaults to `"echo"`.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the thinking level. Defaults to `Off`.
    pub fn thinking_level(mut self, level: runie_core::model::ThinkingLevel) -> Self {
        self.thinking_level = Some(level);
        self
    }

    /// Set the read-only flag. Defaults to `false`.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = Some(read_only);
        self
    }

    /// Set the skills context. Defaults to empty.
    pub fn skills_context(mut self, context: impl Into<String>) -> Self {
        self.skills_context = Some(context.into());
        self
    }

    /// Set the system prompt. Defaults to empty.
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the truncation policy. Defaults to the default policy.
    pub fn truncation(mut self, policy: crate::truncate::TruncationPolicy) -> Self {
        self.truncation = Some(policy);
        self
    }

    /// Set the cancellation token. Defaults to a new `CancellationToken`.
    pub fn cancellation_token(mut self, token: tokio_util::sync::CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// Build the final [`crate::AgentCommand`].
    #[allow(clippy::unwrap_used)]
    pub fn build(self) -> crate::AgentCommand {
        crate::AgentCommand {
            content: self.content,
            id: self.id.unwrap_or_else(|| "req.0".to_owned()),
            provider: self.provider.unwrap_or_else(|| DEFAULT_PROVIDER.to_owned()),
            model: self.model.unwrap_or_else(|| DEFAULT_MODEL.to_owned()),
            thinking_level: self
                .thinking_level
                .unwrap_or(runie_core::model::ThinkingLevel::Off),
            read_only: self.read_only.unwrap_or(false),
            skills_context: self.skills_context.unwrap_or_default(),
            system_prompt: self.system_prompt.unwrap_or_default(),
            truncation: self.truncation.unwrap_or_default(),
            cancellation_token: self.cancellation_token.unwrap_or_default(),
        }
    }
}

/// Construct an [`crate::AgentCommand`] with test defaults.
///
/// Short-hand for `AgentCommandBuilder::new(content).build()`.
///
/// # Example
///
/// ```ignore
/// let cmd = agent_cmd("hello");
/// let cmd = agent_cmd("list files").id("req.1").model("fast").build();
/// ```
pub fn agent_cmd(content: impl Into<String>) -> AgentCommandBuilder {
    AgentCommandBuilder::new(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_with_defaults() {
        let cmd = agent_cmd("hello").build();
        assert_eq!(cmd.content, "hello");
        assert_eq!(cmd.id, "req.0");
        assert_eq!(cmd.provider, "mock");
        assert_eq!(cmd.model, "echo");
        assert!(!cmd.read_only);
    }

    #[test]
    fn builder_with_custom_fields() {
        let cmd = agent_cmd("hello")
            .id("custom.1")
            .provider("openai")
            .model("gpt-4")
            .thinking_level(runie_core::model::ThinkingLevel::High)
            .read_only(true)
            .build();
        assert_eq!(cmd.content, "hello");
        assert_eq!(cmd.id, "custom.1");
        assert_eq!(cmd.provider, "openai");
        assert_eq!(cmd.model, "gpt-4");
        assert_eq!(cmd.thinking_level, runie_core::model::ThinkingLevel::High);
        assert!(cmd.read_only);
    }

    #[test]
    fn builder_chains_correctly() {
        let cmd = agent_cmd("test")
            .id("x")
            .id("y") // later call wins
            .build();
        assert_eq!(cmd.id, "y");
    }
}
