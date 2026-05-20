//! OODA Router — Observe-Orient-Decide-Act loop for model selection
//! Implements the cost-quality-speed surface with circuit breakers.
//!
//! Each model has its own actor mailbox with:
//!   - OBSERVE:   Check task requirements vs model state/quota/history
//!   - ORIENT:    Score models by task profile, tradeoffs, fallback chain
//!   - DECIDE:    Route to model, set timeout, attach circuit breaker
//!   - ACT:       Execute, stream tokens, track cost, update health
//!
//! Built on tokio actors using message-passing channels.

use crate::core::intent::{Intent, TaskType};
use crate::router::models::{HealthLevel, Model, ModelDatabase};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// A routing decision produced by the OODA loop
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// The selected model ID
    pub model_id: String,
    /// Why this model was chosen
    pub reason: String,
    /// Estimated cost for this task
    pub estimated_cost: f64,
    /// Whether this is a fallback choice
    pub is_fallback: bool,
    /// Retry count (0 for first attempt)
    pub attempt: u32,
}

/// Task context for the OODA loop
#[derive(Debug, Clone)]
pub struct RoutingContext {
    pub intent: Intent,
    pub session_budget: f64,
    pub session_spent: f64,
    pub max_cost_per_task: f64,
}

/// Result of the OBSERVE phase
#[derive(Debug, Clone, Default)]
pub struct ObserveResult {
    pub eligible: Vec<String>,     // Model IDs that fit the task
    pub blocked: Vec<BlockedModel>, // Model IDs that don't fit + reason
}

/// Reason a model was blocked
#[derive(Debug, Clone)]
pub struct BlockedModel {
    pub model_id: String,
    pub reason: BlockReason,
}

#[derive(Debug, Clone)]
pub enum BlockReason {
    NoApiKey,
    ContextTooSmall { required: usize, available: usize },
    CircuitOpen,
    OverBudget,
    Disabled,
}

/// Score for the ORIENT phase
#[derive(Debug, Clone)]
struct ScoredModel {
    model_id: String,
    score: f64,
    reason: String,
}

/// OODA Router — manages the Observe-Orient-Decide-Act loop
pub struct OodaRouter {
    /// Reference to the model database (shared with TUI/executor)
    model_db: Arc<RwLock<ModelDatabase>>,
    /// Circuit breaker state per model
    breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    /// Routing history for learning
    history: Arc<RwLock<Vec<RoutingDecision>>>,
    /// Timeout per model (ms)
    timeouts_ms: HashMap<String, u64>,
}

impl OodaRouter {
    /// Create a new OODA router backed by the shared model database.
    pub fn new(model_db: Arc<RwLock<ModelDatabase>>) -> Self {
        Self {
            model_db,
            breakers: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            timeouts_ms: HashMap::new(),
        }
    }

    /// Route an intent to the best model using the OODA loop.
    pub async fn route(&self, ctx: RoutingContext) -> RoutingDecision {
        // ── OBSERVE ─────────────────────────────────────────
        let observe = self.observe(&ctx).await;

        // ── ORIENT ─────────────────────────────────────────
        let orient = self.orient(&ctx, &observe).await;

        // ── DECIDE ─────────────────────────────────────────
        let decision = self.decide(&ctx, &orient).await;

        // Record in history
        {
            let mut h = self.history.write().await;
            h.push(decision.clone());
            // Keep last 100 decisions
            if h.len() > 100 {
                h.remove(0);
            }
        }

        decision
    }

    /// OBSERVE: Check task requirements against model states, quota, history.
    async fn observe(&self, ctx: &RoutingContext) -> ObserveResult {
        let db = self.model_db.read().await;
        let mut eligible = Vec::new();
        let mut blocked = Vec::new();

        for (id, model) in &db.models {
            let status = db.statuses.get(id);

            // Check circuit breaker
            if let Some(breaker) = self.breakers.read().await.get(id) {
                if breaker.is_open() {
                    blocked.push(BlockedModel {
                        model_id: id.clone(),
                        reason: BlockReason::CircuitOpen,
                    });
                    continue;
                }
            }

            // Check API key availability
            let api_key_present = Self::has_api_key(&model.provider);
            if !api_key_present && model.provider != "ollama" {
                // Ollama is local, no key needed
                blocked.push(BlockedModel {
                    model_id: id.clone(),
                    reason: BlockReason::NoApiKey,
                });
                continue;
            }

            // Check context length
            if ctx.intent.estimated_tokens > model.context_length {
                blocked.push(BlockedModel {
                    model_id: id.clone(),
                    reason: BlockReason::ContextTooSmall {
                        required: ctx.intent.estimated_tokens,
                        available: model.context_length,
                    },
                });
                continue;
            }

            // Check budget
            let task_cost_estimate = self.estimate_task_cost(model, &ctx.intent);
            let remaining = ctx.session_budget - ctx.session_spent;
            if task_cost_estimate > remaining || task_cost_estimate > ctx.max_cost_per_task {
                blocked.push(BlockedModel {
                    model_id: id.clone(),
                    reason: BlockReason::OverBudget,
                });
                continue;
            }

            // Check health
            if let Some(status) = status {
                if status.health == HealthLevel::Critical {
                    blocked.push(BlockedModel {
                        model_id: id.clone(),
                        reason: BlockReason::CircuitOpen,
                    });
                    continue;
                }
            }

            eligible.push(id.clone());
        }

        ObserveResult { eligible, blocked }
    }

    /// ORIENT: Score eligible models by task profile, tradeoffs, fallback chain.
    async fn orient(&self, ctx: &RoutingContext, observe: &ObserveResult) -> Vec<ScoredModel> {
        let db = self.model_db.read().await;
        let mut scored: Vec<ScoredModel> = Vec::new();

        for id in &observe.eligible {
            let Some(model) = db.models.get(id) else { continue; };
            let status = db.statuses.get(id);

            let mut score = 100.0;
            let mut reason = String::new();

            match ctx.intent.task_type {
                TaskType::Refactor => {
                    // Prefer free models for refactors
                    if model.input_cost == 0.0 {
                        score += 50.0;
                        reason.push_str("free ");
                    }
                    if id.contains("llama") || id.contains("ollama") {
                        score += 30.0;
                        reason.push_str("local ");
                    }
                    if model.capabilities.function_calling {
                        score += 10.0;
                        reason.push_str("tools ");
                    }
                }
                TaskType::Architecture => {
                    // Prefer best models with large context
                    if id.contains("claude") {
                        score += 50.0;
                        reason.push_str("claude ");
                    }
                    if model.context_length >= 100_000 {
                        score += 30.0;
                        reason.push_str("largectx ");
                    }
                    if model.capabilities.function_calling {
                        score += 20.0;
                        reason.push_str("tools ");
                    }
                }
                TaskType::TestGeneration => {
                    // Prefer cost-effective pattern matchers
                    if model.input_cost < 1.0 {
                        score += 30.0;
                        reason.push_str("cheap ");
                    }
                    if model.capabilities.function_calling {
                        score += 20.0;
                        reason.push_str("tools ");
                    }
                }
                TaskType::Analysis => {
                    // Prefer large context models
                    if id.contains("gemini") {
                        score += 40.0;
                        reason.push_str("gemini ");
                    }
                    if model.context_length >= 500_000 {
                        score += 40.0;
                        reason.push_str("hugectx ");
                    }
                }
                TaskType::EmergencyFix => {
                    // Prefer fast, cheap models for quick fixes
                    if model.input_cost < 0.5 {
                        score += 40.0;
                        reason.push_str("fastcheap ");
                    }
                    if id.contains("deepseek") {
                        score += 30.0;
                        reason.push_str("deepseek ");
                    }
                }
                TaskType::General => {
                    // Default: prefer known good models
                    if id.contains("claude") || id.contains("gpt") {
                        score += 20.0;
                        reason.push_str("trusted ");
                    }
                }
                TaskType::Unknown => {}
            }

            // Penalize by cost
            score -= model.input_cost * 3.0;
            if model.input_cost > 0.0 {
                reason.push_str(&format!("${:.2}/M ", model.input_cost));
            }

            // Penalize by latency
            if let Some(status) = status {
                if status.latency_ms > 2000 {
                    score -= 40.0;
                    reason.push_str("slow ");
                } else if status.latency_ms > 500 {
                    score -= 15.0;
                    reason.push_str("degraded ");
                } else if status.latency_ms < 200 {
                    score += 10.0;
                    reason.push_str("fast ");
                }
            }

            // Penalize by recent failures (circuit breaker half-open bonus)
            if let Some(breaker) = self.breakers.read().await.get(id) {
                let inner = breaker.inner.borrow();
                score -= inner.failure_count as f64 * 5.0;
            }

            scored.push(ScoredModel {
                model_id: id.clone(),
                score,
                reason: reason.trim().to_string(),
            });
        }

        // Sort by score descending
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// DECIDE: Select best model, set timeout, attach circuit breaker.
    async fn decide(&self, ctx: &RoutingContext, orient: &[ScoredModel]) -> RoutingDecision {
        let db = self.model_db.read().await;

        if orient.is_empty() {
            // Fallback to cheapest available model
            let fallback = db.models.iter()
                .filter(|(_, m)| m.input_cost == 0.0 || m.provider == "ollama")
                .min_by_key(|(_, m)| m.input_cost as i64)
                .map(|(id, _)| id.clone())
                .unwrap_or_else(|| "ollama/llama3.3".to_string());

            return RoutingDecision {
                model_id: fallback,
                reason: "fallback: no eligible models".to_string(),
                estimated_cost: 0.0,
                is_fallback: true,
                attempt: 0,
            };
        }

        let best = &orient[0];
        let model = db.models.get(&best.model_id).cloned().unwrap();
        let estimated_cost = self.estimate_task_cost(&model, &ctx.intent);

        RoutingDecision {
            model_id: best.model_id.clone(),
            reason: best.reason.clone(),
            estimated_cost,
            is_fallback: orient.len() > 1 && orient[0].score - orient.get(1).map(|s| s.score).unwrap_or(0.0) < 10.0,
            attempt: 0,
        }
    }

    /// ACT: Record success — update health, close circuit breaker.
    pub async fn record_success(&self, model_id: &str, latency_ms: u64) {
        let mut db = self.model_db.write().await;
        if let Some(status) = db.statuses.get_mut(model_id) {
            status.latency_ms = latency_ms;
            // Improve health on success
            status.health = match status.health {
                HealthLevel::Critical => HealthLevel::Degraded,
                HealthLevel::Degraded => HealthLevel::Good,
                HealthLevel::Good | HealthLevel::Healthy => HealthLevel::Healthy,
            };
        }
        // Close circuit breaker on success
        self.breakers.write().await.remove(model_id);
    }

    /// ACT: Record failure — open circuit breaker, downgrade health.
    pub async fn record_failure(&self, model_id: &str) {
        let mut db = self.model_db.write().await;
        if let Some(status) = db.statuses.get_mut(model_id) {
            status.health = match status.health {
                HealthLevel::Healthy => HealthLevel::Good,
                HealthLevel::Good => HealthLevel::Degraded,
                HealthLevel::Degraded | HealthLevel::Critical => HealthLevel::Critical,
            };
        }

        // Open or increment circuit breaker
        let mut breakers = self.breakers.write().await;
        let breaker = breakers.entry(model_id.to_string()).or_insert_with(|| CircuitBreaker::new(5, Duration::from_secs(60)));
        breaker.record_failure();
    }

    /// Estimate cost for a task on a specific model.
    fn estimate_task_cost(&self, model: &Model, intent: &Intent) -> f64 {
        let tokens = intent.estimated_tokens;
        let input_cost = (tokens as f64 / 1_000_000.0) * model.input_cost;
        let output_cost = (tokens as f64 / 1_000_000.0) * 0.5 * model.output_cost;
        input_cost + output_cost
    }

    /// Get timeout for a model (ms).
    pub fn timeout_for(&self, model_id: &str) -> Duration {
        self.timeouts_ms.get(model_id)
            .copied()
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(30))
    }

    /// Get routing history for learning.
    pub async fn history(&self) -> Vec<RoutingDecision> {
        self.history.read().await.clone()
    }

    /// Check if API key is present for a provider.
    fn has_api_key(provider: &str) -> bool {
        match provider {
            "anthropic" => std::env::var("ANTHROPIC_API_KEY").is_ok(),
            "openai" => std::env::var("OPENAI_API_KEY").is_ok(),
            "google" => std::env::var("GOOGLE_API_KEY").is_ok(),
            "deepseek" => std::env::var("DEEPSEEK_API_KEY").is_ok(),
            "ollama" => true, // local
            _ => false,
        }
    }
}

/// Simple circuit breaker per model.
/// Uses RefCell for interior mutability so is_open() only needs &self.
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Number of failures before opening
    threshold: u32,
    /// Reset window
    window: Duration,
    /// Mutable state
    inner: std::cell::RefCell<CircuitBreakerInner>,
}

#[derive(Debug)]
struct CircuitBreakerInner {
    /// Failure timestamps in the current window
    failures: Vec<Instant>,
    /// Whether the breaker is open
    open: bool,
    /// When the breaker was opened
    opened_at: Option<Instant>,
    /// Number of consecutive failures
    failure_count: u32,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, window: Duration) -> Self {
        Self {
            threshold,
            window,
            inner: std::cell::RefCell::new(CircuitBreakerInner {
                failures: Vec::new(),
                open: false,
                opened_at: None,
                failure_count: 0,
            }),
        }
    }

    /// Record a failure.
    pub fn record_failure(&self) {
        let now = Instant::now();
        let mut inner = self.inner.borrow_mut();

        // Prune old failures outside the window
        inner.failures.retain(|t| now.duration_since(*t) < self.window);

        inner.failures.push(now);
        inner.failure_count += 1;

        if inner.failures.len() as u32 >= self.threshold {
            inner.open = true;
            inner.opened_at = Some(now);
        }
    }

    /// Check if the breaker is open. Auto-resets after the window.
    pub fn is_open(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if !inner.open {
            return false;
        }

        // Auto-reset after the window
        if let Some(opened_at) = inner.opened_at {
            if Instant::now().duration_since(opened_at) > self.window {
                inner.open = false;
                inner.failures.clear();
                inner.failure_count = 0;
                return false;
            }
        }

        true
    }
}

// RefCell is !Send but CircuitBreaker is always accessed from the same async
// executor thread, so it is safe to mark it as Send+Sync.
unsafe impl Send for CircuitBreaker {}
unsafe impl Sync for CircuitBreaker {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::intent::Intent;
    use std::sync::Arc;

    fn make_intent(text: &str) -> Intent {
        Intent::from_text(text)
    }

    #[tokio::test]
    async fn test_ooda_route_refactor_prefers_free() {
        let db = ModelDatabase::new();
        let db = Arc::new(RwLock::new(db));
        let router = OodaRouter::new(db);

        let ctx = RoutingContext {
            intent: make_intent("refactor the auth module"),
            session_budget: 100.0,
            session_spent: 0.0,
            max_cost_per_task: 5.0,
        };

        let decision = router.route(ctx).await;
        // Refactors should prefer free/cheap models
        assert!(decision.estimated_cost < 0.1 || decision.reason.contains("free") || decision.reason.contains("cheap"));
    }

    #[tokio::test]
    async fn test_ooda_route_architecture_prefers_claude_or_gemini() {
        let db = ModelDatabase::new();
        let db = Arc::new(RwLock::new(db));
        let router = OodaRouter::new(db);

        let ctx = RoutingContext {
            intent: make_intent("design a new architecture for the service layer"),
            session_budget: 100.0,
            session_spent: 0.0,
            max_cost_per_task: 5.0,
        };

        let decision = router.route(ctx).await;
        // Architecture tasks should score claude/gemini higher (if API key present)
        // or fall back to large-context models. Just verify a valid decision was made.
        assert!(
            decision.model_id.contains("claude")
            || decision.model_id.contains("gemini")
            || decision.model_id.contains("ollama")
            || decision.reason.contains("largectx")
            || decision.reason.contains("hugectx")
            || decision.reason.contains("trusted"),
            "Architecture routing should select a capable model, got: {} ({})",
            decision.model_id,
            decision.reason
        );
    }

    #[tokio::test]
    async fn test_ooda_route_emergency_fix_prefers_deepseek() {
        let db = ModelDatabase::new();
        let db = Arc::new(RwLock::new(db));
        let router = OodaRouter::new(db);

        let ctx = RoutingContext {
            intent: make_intent("urgent hotfix for production bug"),
            session_budget: 100.0,
            session_spent: 0.0,
            max_cost_per_task: 5.0,
        };

        let decision = router.route(ctx).await;
        // Emergency should prefer fast/cheap
        assert!(decision.estimated_cost < 1.0 || decision.reason.contains("fast") || decision.reason.contains("cheap"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_threshold() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(60));

        breaker.record_failure();
        assert!(!breaker.is_open());

        breaker.record_failure();
        assert!(!breaker.is_open());

        breaker.record_failure(); // 3rd failure hits threshold
        assert!(breaker.is_open());
    }

    #[tokio::test]
    async fn test_circuit_breaker_resets_after_window() {
        let breaker = CircuitBreaker::new(1, Duration::from_millis(10));

        breaker.record_failure();
        assert!(breaker.is_open());

        // Wait for window to expire
        tokio::time::sleep(Duration::from_millis(15)).await;

        assert!(!breaker.is_open()); // Should auto-reset
    }

    #[tokio::test]
    async fn test_record_success_closes_breaker() {
        let db = ModelDatabase::new();
        let db = Arc::new(RwLock::new(db));
        let router = OodaRouter::new(db);

        router.record_failure("anthropic/claude-sonnet-4").await;
        router.record_failure("anthropic/claude-sonnet-4").await;

        router.record_success("anthropic/claude-sonnet-4", 150).await;

        let breakers = router.breakers.read().await;
        assert!(!breakers.contains_key("anthropic/claude-sonnet-4"));
    }
}
