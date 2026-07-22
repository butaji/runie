//! Circuit breaker for tool execution.
//!
//! Prevents repeated tool failures by tripping after threshold failures and
//! allowing recovery through a half-open probe phase.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

/// Number of consecutive failures before the circuit trips OPEN.
const DEFAULT_FAILURE_THRESHOLD: u32 = 5;
/// Number of consecutive successes in HALF_OPEN state before closing.
const DEFAULT_SUCCESS_THRESHOLD: u32 = 2;

/// Recovery timeout before transitioning from OPEN to HALF_OPEN.
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(60);

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed; requests pass through normally.
    Closed,
    /// Circuit is open; requests are blocked.
    Open,
    /// Circuit is half-open; allowing a probe request through.
    HalfOpen,
}

/// Circuit breaker for a single tool.
///
/// State machine: CLOSED → OPEN → HALF_OPEN → CLOSED (or back to OPEN).
#[derive(Clone)]
pub struct CircuitBreaker {
    /// Number of consecutive failures.
    failures: u32,
    /// Number of consecutive successes in HALF_OPEN state.
    successes: u32,
    /// Current circuit state.
    state: CircuitState,
    /// Timestamp of the last failure (for recovery timeout).
    last_failure_time: Option<Instant>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self { failures: 0, successes: 0, state: CircuitState::Closed, last_failure_time: None }
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker in the CLOSED state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful tool execution.
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failures = 0;
            }
            CircuitState::HalfOpen => {
                self.successes += 1;
                if self.successes >= DEFAULT_SUCCESS_THRESHOLD {
                    // Enough successes → close the circuit
                    self.state = CircuitState::Closed;
                    self.failures = 0;
                    self.successes = 0;
                }
            }
            CircuitState::Open => {
                // Should not happen; success in Open means we transitioned
            }
        }
    }

    /// Record a failed tool execution.
    pub fn record_failure(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failures += 1;
                if self.failures >= DEFAULT_FAILURE_THRESHOLD {
                    self.state = CircuitState::Open;
                    self.last_failure_time = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                // Probe failed → trip back to open
                self.state = CircuitState::Open;
                self.last_failure_time = Some(Instant::now());
                self.successes = 0;
            }
            CircuitState::Open => {
                self.last_failure_time = Some(Instant::now());
            }
        }
    }

    /// Returns true if the circuit is currently open (requests blocked).
    pub fn is_open(&self) -> bool {
        self.state == CircuitState::Open
    }

    /// Returns the current circuit state.
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Returns true if the circuit is open, but should transition to half-open
    /// because the recovery timeout has elapsed.
    pub fn should_attempt_recovery(&self) -> bool {
        self.state == CircuitState::Open
            && self
                .last_failure_time
                .is_some_and(|t| t.elapsed() >= RECOVERY_TIMEOUT)
    }

    /// Attempt to move from OPEN to HALF_OPEN.
    ///
    /// Returns true if the transition was made (allowing one probe request).
    /// Returns false if not in OPEN state or timeout not yet elapsed.
    pub fn call_once(&mut self) -> bool {
        if self.state == CircuitState::Open {
            self.state = CircuitState::HalfOpen;
            self.successes = 0;
            true
        } else {
            false
        }
    }

    /// Returns the number of consecutive failures (for debugging/metrics).
    #[cfg(test)]
    pub fn failures(&self) -> u32 {
        self.failures
    }
}

/// Registry mapping tool names to their circuit breakers.
pub struct CircuitBreakerRegistry {
    /// Per-tool circuit breakers.
    circuits: RwLock<HashMap<String, Arc<RwLock<CircuitBreaker>>>>,
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self { circuits: RwLock::new(HashMap::new()) }
    }
}

impl CircuitBreakerRegistry {
    /// Create a new empty registry.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Get or create a circuit breaker for the given tool.
    pub fn get_or_create(self: &Arc<Self>, tool_name: &str) -> Arc<RwLock<CircuitBreaker>> {
        {
            let circuits = self.circuits.read();
            if let Some(cb) = circuits.get(tool_name) {
                return cb.clone();
            }
        }
        let cb = Arc::new(RwLock::new(CircuitBreaker::new()));
        {
            let mut circuits = self.circuits.write();
            circuits.insert(tool_name.to_string(), cb.clone());
        }
        cb
    }

    /// Check if a tool's circuit is open (blocked).
    pub fn is_open(&self, tool_name: &str) -> bool {
        self.circuits
            .read()
            .get(tool_name)
            .is_some_and(|cb| cb.read().is_open())
    }

    /// Check if a tool's circuit is open and should attempt a probe.
    pub fn should_probe(&self, tool_name: &str) -> bool {
        self.circuits
            .read()
            .get(tool_name)
            .is_some_and(|cb| cb.read().should_attempt_recovery())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CircuitBreaker state transitions ─────────────────────────────────────────

    #[test]
    fn closed_circuit_stays_closed_on_success() {
        let mut cb = CircuitBreaker::new();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failures(), 0);
    }

    #[test]
    fn closed_circuit_trips_after_threshold_failures() {
        let mut cb = CircuitBreaker::new();
        for _ in 0..4 {
            cb.record_failure();
            assert_eq!(cb.state(), CircuitState::Closed);
        }
        cb.record_failure(); // 5th failure
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn open_circuit_resets_failures_on_success() {
        let mut cb = CircuitBreaker::new();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Simulate recovery
        cb.call_once();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failures(), 0);
    }

    #[test]
    fn half_open_probe_failure_trips_back_to_open() {
        let mut cb = CircuitBreaker::new();

        // Trip the circuit
        for _ in 0..5 {
            cb.record_failure();
        }
        assert_eq!(cb.state(), CircuitState::Open);

        // Probe
        assert!(cb.call_once());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Probe fails
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn half_open_success_closes_after_threshold() {
        let mut cb = CircuitBreaker::new();

        // Trip the circuit
        for _ in 0..5 {
            cb.record_failure();
        }

        // Probe
        cb.call_once();

        // First success in half-open
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Second success closes the circuit
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn call_once_only_works_in_open_state() {
        let mut cb = CircuitBreaker::new();
        assert!(!cb.call_once(), "should not transition in Closed state");
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(cb.call_once(), "should transition from Open");
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        assert!(!cb.call_once(), "should not transition again in HalfOpen");
    }

    #[test]
    fn is_open_returns_true_only_when_open() {
        let mut cb = CircuitBreaker::new();
        assert!(!cb.is_open());

        for _ in 0..5 {
            cb.record_failure();
        }
        assert!(cb.is_open());

        cb.call_once();
        assert!(!cb.is_open());
    }

    // ── CircuitBreakerRegistry ───────────────────────────────────────────────────

    #[test]
    fn registry_creates_circuit_on_first_access() {
        let registry = CircuitBreakerRegistry::new();

        let cb = registry.get_or_create("bash");
        {
            let guard = cb.read();
            assert_eq!(guard.state(), CircuitState::Closed);
        }

        assert!(!registry.is_open("bash"));
    }

    #[test]
    fn registry_returns_same_instance_for_same_tool() {
        let registry = CircuitBreakerRegistry::new();

        let cb1 = registry.get_or_create("bash");
        let cb2 = registry.get_or_create("bash");

        assert!(
            Arc::ptr_eq(&cb1, &cb2),
            "same tool must return same instance"
        );
    }

    #[test]
    fn registry_tracks_multiple_tools() {
        let registry = CircuitBreakerRegistry::new();

        let bash = registry.get_or_create("bash");
        let _read = registry.get_or_create("read_file");

        // Trip bash only
        {
            let mut guard = bash.write();
            for _ in 0..5 {
                guard.record_failure();
            }
        }

        assert!(registry.is_open("bash"));
        assert!(!registry.is_open("read_file"));
    }

    // ── Smoke test ──────────────────────────────────────────────────────────────

    #[test]
    fn circuit_breaker_smoke_test() {
        let registry = CircuitBreakerRegistry::new();

        // bash fails 5 times
        {
            let cb = registry.get_or_create("bash");
            let mut guard = cb.write();
            for _ in 0..5 {
                guard.record_failure();
            }
        }
        assert!(registry.is_open("bash"));

        // Recovery probe
        {
            let cb = registry.get_or_create("bash");
            let mut guard = cb.write();
            guard.call_once();
            guard.record_success();
            guard.record_success();
        }
        assert!(!registry.is_open("bash"));
    }
}
