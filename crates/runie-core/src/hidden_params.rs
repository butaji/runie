//! Hidden parameters attached to provider response events.
//!
//! Allows internal metadata (response cost, API base, original model name,
//! additional headers) to travel alongside `ProviderEvent`s without changing
//! the public surface of the event enum or its serialization.

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use crate::provider_event::ProviderEvent;

/// Cost breakdown for a single response (prompt + completion).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResponseCost {
    pub input_cost: f64,
    pub output_cost: f64,
}

/// Non-public struct holding metadata attached to a provider response.
/// Stored behind `Arc` so cloning is cheap.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HiddenParams {
    /// Actual cost incurred by this response.
    pub response_cost: Option<ResponseCost>,
    /// API base URL used for this request (may differ from configured default).
    pub api_base: Option<String>,
    /// Original model name before any normalization or aliasing.
    pub original_model: Option<String>,
    /// Extra HTTP headers injected during this request.
    pub additional_headers: HashMap<String, String>,
}

impl HiddenParams {
    /// Create a new `HiddenParams` builder.
    pub fn builder() -> HiddenParamsBuilder {
        HiddenParamsBuilder(HiddenParams::default())
    }
}

/// Builder for `HiddenParams`.
#[derive(Debug, Clone, Default)]
pub struct HiddenParamsBuilder(HiddenParams);

impl HiddenParamsBuilder {
    pub fn response_cost(mut self, cost: ResponseCost) -> Self {
        self.0.response_cost = Some(cost);
        self
    }

    pub fn api_base(mut self, base: impl Into<String>) -> Self {
        self.0.api_base = Some(base.into());
        self
    }

    pub fn original_model(mut self, model: impl Into<String>) -> Self {
        self.0.original_model = Some(model.into());
        self
    }

    pub fn insert_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.additional_headers.insert(key.into(), value.into());
        self
    }

    /// Consume the builder and return the final `HiddenParams`.
    pub fn build(self) -> HiddenParams {
        self.0
    }

    /// Consume the builder and return a shared `Arc<HiddenParams>`.
    pub fn build_arc(self) -> Arc<HiddenParams> {
        Arc::new(self.0)
    }
}

/// Trait for types that may carry hidden parameters from a provider response.
pub trait AsHiddenParams {
    /// Returns a reference to the hidden parameters, if any are set.
    fn hidden_params(&self) -> Option<&Arc<HiddenParams>>;

    /// Returns a mutable reference to the hidden parameters for internal use.
    /// Panics if no hidden parameters are set.
    fn hidden_params_mut(&mut self) -> Option<&mut Arc<HiddenParams>> {
        None
    }

    /// Returns `true` if this type has hidden parameters attached.
    fn has_hidden_params(&self) -> bool {
        self.hidden_params().is_some()
    }

    /// Get a clone of the inner `ProviderEvent` (loses hidden params).
    fn inner(&self) -> ProviderEvent
    where
        Self: Deref<Target = ProviderEvent>,
    {
        (**self).clone()
    }
}

/// A `ProviderEvent` enriched with hidden parameters.
///
/// This newtype wraps `ProviderEvent` and implements `AsHiddenParams`,
/// allowing metadata from the provider layer (cost, API base, etc.)
/// to be carried through the pipeline without modifying the public
/// event enum.
#[derive(Debug, Clone)]
pub struct ProviderEventWithHiddenParams {
    /// The underlying provider event.
    pub event: ProviderEvent,
    /// Hidden metadata attached to this event.
    hidden: Arc<HiddenParams>,
}

impl ProviderEventWithHiddenParams {
    /// Wrap a `ProviderEvent` with no hidden parameters.
    pub fn new(event: ProviderEvent) -> Self {
        Self { event, hidden: Arc::new(HiddenParams::default()) }
    }

    /// Wrap a `ProviderEvent` with the given hidden parameters.
    pub fn with_params(event: ProviderEvent, params: HiddenParams) -> Self {
        Self { event, hidden: Arc::new(params) }
    }

    /// Wrap a `ProviderEvent` with the given hidden parameters (arc form).
    pub fn with_params_arc(event: ProviderEvent, params: Arc<HiddenParams>) -> Self {
        Self { event, hidden: params }
    }

    /// Replace the hidden parameters with a new `Arc<HiddenParams>`.
    pub fn set_hidden_params(&mut self, params: Arc<HiddenParams>) {
        self.hidden = params;
    }

    /// Consume self and return the underlying event, discarding hidden params.
    pub fn into_inner(self) -> ProviderEvent {
        self.event
    }

    /// Map the inner event, preserving hidden params.
    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(ProviderEvent) -> ProviderEvent,
    {
        Self { event: f(self.event), hidden: self.hidden }
    }

    /// Returns a reference to the wrapped `ProviderEvent`.
    pub fn event(&self) -> &ProviderEvent {
        &self.event
    }
}

impl AsHiddenParams for ProviderEventWithHiddenParams {
    fn hidden_params(&self) -> Option<&Arc<HiddenParams>> {
        if self.hidden.response_cost.is_none()
            && self.hidden.api_base.is_none()
            && self.hidden.original_model.is_none()
            && self.hidden.additional_headers.is_empty()
        {
            None
        } else {
            Some(&self.hidden)
        }
    }

    fn hidden_params_mut(&mut self) -> Option<&mut Arc<HiddenParams>> {
        Some(&mut self.hidden)
    }

    fn has_hidden_params(&self) -> bool {
        self.hidden_params().is_some()
    }
}

impl Deref for ProviderEventWithHiddenParams {
    type Target = ProviderEvent;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

impl From<ProviderEvent> for ProviderEventWithHiddenParams {
    fn from(event: ProviderEvent) -> Self {
        Self::new(event)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_params_builder_default() {
        let params = HiddenParams::builder().build();
        assert!(params.response_cost.is_none());
        assert!(params.api_base.is_none());
        assert!(params.original_model.is_none());
        assert!(params.additional_headers.is_empty());
    }

    #[test]
    fn hidden_params_builder_full() {
        let params = HiddenParams::builder()
            .response_cost(ResponseCost { input_cost: 0.001, output_cost: 0.003 })
            .api_base("https://api.anthropic.com")
            .original_model("claude-3-5-sonnet-20241022")
            .insert_header("X-Custom", "value")
            .build();

        assert!(params.response_cost.is_some());
        assert_eq!(
            params.api_base.as_deref(),
            Some("https://api.anthropic.com")
        );
        assert_eq!(
            params.original_model.as_deref(),
            Some("claude-3-5-sonnet-20241022")
        );
        assert_eq!(
            params.additional_headers.get("X-Custom"),
            Some(&"value".into())
        );
    }

    #[test]
    fn hidden_params_arc_cheap_clone() {
        let params = Arc::new(HiddenParams::builder().api_base("http://localhost").build());
        let cloned = params.clone();
        assert_eq!(Arc::strong_count(&params), 2);
        drop(cloned);
        assert_eq!(Arc::strong_count(&params), 1);
    }

    #[test]
    fn provider_event_wrapper_basic() {
        let event = ProviderEvent::TextDelta("hello".into());
        let wrapped = ProviderEventWithHiddenParams::new(event.clone());
        assert!(!wrapped.has_hidden_params());
        assert!(wrapped.hidden_params().is_none());
        assert_eq!(wrapped.event, event);
    }

    #[test]
    fn provider_event_wrapper_with_params() {
        let event = ProviderEvent::Usage { input_tokens: 100, output_tokens: 50 };
        let params = HiddenParams::builder()
            .response_cost(ResponseCost { input_cost: 0.001, output_cost: 0.002 })
            .api_base("https://api.anthropic.com/v1")
            .original_model("claude-3-sonnet-4-20250514")
            .build();

        let wrapped = ProviderEventWithHiddenParams::with_params(event, params);

        assert!(wrapped.has_hidden_params());
        let hp = wrapped.hidden_params().unwrap();
        assert_eq!(hp.response_cost.as_ref().map(|c| c.input_cost), Some(0.001));
        assert_eq!(hp.api_base.as_deref(), Some("https://api.anthropic.com/v1"));
        assert_eq!(
            hp.original_model.as_deref(),
            Some("claude-3-sonnet-4-20250514")
        );
    }

    #[test]
    fn provider_event_wrapper_set_hidden_params() {
        let event = ProviderEvent::TextDelta("hi".into());
        let mut wrapped = ProviderEventWithHiddenParams::new(event);
        assert!(!wrapped.has_hidden_params());

        let new_params = HiddenParams::builder()
            .insert_header("X-Trace-Id", "abc123")
            .build_arc();
        wrapped.set_hidden_params(new_params);

        assert!(wrapped.has_hidden_params());
        let hp = wrapped.hidden_params().unwrap();
        assert_eq!(
            hp.additional_headers.get("X-Trace-Id"),
            Some(&"abc123".into())
        );
    }

    #[test]
    fn provider_event_wrapper_map_preserves_hidden_params() {
        let event = ProviderEvent::TextStart { id: "a".into() };
        let params = HiddenParams::builder().api_base("http://foo").build_arc();
        let wrapped = ProviderEventWithHiddenParams::with_params_arc(event, params);

        let mapped = wrapped.map(|e| match e {
            ProviderEvent::TextStart { id } => ProviderEvent::TextDelta(format!("id={}", id)),
            other => other,
        });

        assert!(matches!(mapped.event, ProviderEvent::TextDelta(_)));
        // hidden params should be preserved
        let hp = mapped.hidden_params().unwrap();
        assert_eq!(hp.api_base.as_deref(), Some("http://foo"));
    }

    #[test]
    fn provider_event_wrapper_into_inner() {
        let event = ProviderEvent::ThinkingDelta("reasoning".into());
        let wrapped = ProviderEventWithHiddenParams::with_params(event.clone(), HiddenParams::default());
        assert_eq!(wrapped.into_inner(), event);
    }

    #[test]
    fn provider_event_wrapper_deref() {
        let event = ProviderEvent::ToolCallStart { id: "call_1".into(), name: "bash".into() };
        let wrapped = ProviderEventWithHiddenParams::new(event.clone());
        // Deref to ProviderEvent
        assert_eq!(&*wrapped, &event);
    }

    #[test]
    fn provider_event_wrapper_from() {
        let event = ProviderEvent::Finish { reason: crate::provider_event::StopReason::Stop };
        let wrapped: ProviderEventWithHiddenParams = event.clone().into();
        assert_eq!(wrapped.event, event);
        assert!(!wrapped.has_hidden_params());
    }

    #[test]
    fn as_hidden_params_trait_has_hidden_params() {
        let event = ProviderEvent::TextEnd { id: "1".into() };
        let wrapped = ProviderEventWithHiddenParams::with_params(event, HiddenParams::default());
        assert!(
            !wrapped.has_hidden_params(),
            "empty HiddenParams should return false for has_hidden_params"
        );
    }
}
