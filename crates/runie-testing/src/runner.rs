//! High-level test runner for event-driven agent tests.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use runie_agent::{run_agent_turn, AgentCommand, PermissionGate};
use runie_core::config::Config;
use runie_core::event::Event;
use runie_core::permissions::{AutoAllowSink, PermissionManager};
use runie_provider::DynProvider;
use tempfile::TempDir;

use crate::fixtures::{load_default_config_for_test, temp_home};

/// Identifier for a submitted user turn.
pub type TestSubmissionId = String;

/// A test harness that drives an agent turn and collects emitted events.
pub struct TestRunner {
    pub test_home: TempDir,
    pub config: Config,
    events: Arc<Mutex<Vec<Event>>>,
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRunner {
    /// Create a new runner with an isolated temp home and default config.
    pub fn new() -> Self {
        let test_home = temp_home();
        let config = load_default_config_for_test(&test_home);
        Self {
            test_home,
            config,
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Immutable access to the test config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Run a single agent turn with the given input and provider.
    ///
    /// Returns the generated submission id.
    pub async fn submit(
        &self,
        input: &str,
        provider: &DynProvider,
    ) -> anyhow::Result<TestSubmissionId> {
        let id = format!("sub.{}", nanoid());
        let cmd = AgentCommand {
            content: input.to_string(),
            id: id.clone(),
            provider: self.config().provider.clone().unwrap_or("mock".into()),
            model: self.config().default_model().unwrap_or("echo").to_string(),
            thinking_level: runie_core::model::ThinkingLevel::Off,
            read_only: false,
            skills_context: String::new(),
            system_prompt: String::new(),
            truncation: runie_agent::truncate::TruncationPolicy::default(),
        };

        let events = self.events.clone();
        let emit = Arc::new(Mutex::new(move |evt: Event| {
            events.lock().unwrap().push(evt)
        }));
        let gate = PermissionGate::new(PermissionManager::default(), Arc::new(AutoAllowSink));
        run_agent_turn(provider, &cmd, emit, 5, gate).await?;
        Ok(id)
    }

    /// Wait up to `timeout` for an event matching `predicate`.
    pub async fn expect_event_with_timeout<F>(
        &self,
        timeout: Duration,
        predicate: F,
    ) -> Option<Event>
    where
        F: Fn(&Event) -> bool,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            {
                let guard = self.events.lock().unwrap();
                if let Some(evt) = guard.iter().find(|e| predicate(e)) {
                    return Some(evt.clone());
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Wait a default duration for an event matching `predicate`.
    pub async fn expect_event<F>(&self, predicate: F) -> Option<Event>
    where
        F: Fn(&Event) -> bool,
    {
        self.expect_event_with_timeout(Duration::from_secs(2), predicate)
            .await
    }

    /// Return all collected events.
    pub fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }
}

fn nanoid() -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    let seed = format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    seed.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::mock_provider;

    #[tokio::test]
    async fn test_runner_submit_and_expect_event() {
        let runner = TestRunner::new();
        let provider = mock_provider();
        let id = runner.submit("hello", &provider).await.unwrap();
        let evt = runner
            .expect_event(|e| matches!(e, Event::Done { id: i } if i == &id))
            .await;
        assert!(evt.is_some(), "expected Done event for {id}");
    }

    #[tokio::test]
    async fn mock_provider_handles_stream() {
        let runner = TestRunner::new();
        let provider = mock_provider();
        runner.submit("hello", &provider).await.unwrap();
        let evts = runner.events();
        assert!(!evts.is_empty(), "agent turn should emit events");
    }
}
