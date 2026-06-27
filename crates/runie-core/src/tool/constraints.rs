//! Tool parameter constraint validation.
//!
//! This module provides a declarative DSL for defining constraints on tool parameters.
//! Constraints are validated at turn build time, before any provider call.

use serde_json::Value;

// ---------------------------------------------------------------------------
// Constraint types
// ---------------------------------------------------------------------------

/// A constraint on tool parameters.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Constraint {
    /// If field A is true, field B must also be true (or present).
    Implication { if_field: String, then_field: String },
    /// Only one of the listed fields may be set.
    Mutex { fields: Vec<String> },
    /// At least one of the listed fields must be set.
    RequireOne { fields: Vec<String> },
    /// Numeric field must be within range.
    Range { field: String, min: Option<f64>, max: Option<f64> },
    /// String field must match regex pattern.
    Pattern { field: String, pattern: String },
}

impl Constraint {
    /// Create an implication constraint: `if_field implies then_field`.
    pub fn implication(if_field: impl Into<String>, then_field: impl Into<String>) -> Self {
        Self::Implication { if_field: if_field.into(), then_field: then_field.into() }
    }

    /// Create a mutex constraint: only one field can be set.
    pub fn mutex(fields: impl Into<Vec<String>>) -> Self {
        Self::Mutex { fields: fields.into() }
    }

    /// Create a require-one constraint: at least one field must be set.
    pub fn require_one(fields: impl Into<Vec<String>>) -> Self {
        Self::RequireOne { fields: fields.into() }
    }

    /// Create a range constraint: field must be between min and max.
    pub fn range(field: impl Into<String>, min: Option<f64>, max: Option<f64>) -> Self {
        Self::Range { field: field.into(), min, max }
    }

    /// Create a pattern constraint: field must match regex.
    pub fn pattern(field: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self::Pattern { field: field.into(), pattern: pattern.into() }
    }
}

/// A violation of a tool constraint.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConstraintViolation {
    pub constraint: Constraint,
    pub message: String,
}

/// Result of validating constraints against tool arguments.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub violations: Vec<ConstraintViolation>,
}

impl ValidationResult {
    /// Create an empty (valid) result.
    pub fn valid() -> Self {
        Self { violations: Vec::new() }
    }

    /// Create a result with a single violation.
    pub fn violated(constraint: Constraint, message: impl Into<String>) -> Self {
        Self { violations: vec![ConstraintViolation { constraint, message: message.into() }] }
    }

    /// Check if validation passed.
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    /// Add a violation.
    pub fn add_violation(&mut self, constraint: Constraint, message: impl Into<String>) {
        self.violations.push(ConstraintViolation { constraint, message: message.into() });
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: Self) {
        self.violations.extend(other.violations);
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn get_field<'a>(args: &'a Value, field: &str) -> Option<&'a Value> {
    args.get(field)
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Null => false,
        _ => false,
    }
}

fn is_present(value: &Value) -> bool {
    !value.is_null()
}

// ---------------------------------------------------------------------------
// Individual constraint validators
// ---------------------------------------------------------------------------

fn validate_implication(if_field: &str, then_field: &str, args: &Value) -> Option<ConstraintViolation> {
    let if_value = get_field(args, if_field)?;
    let then_value = args.get(then_field);
    if is_truthy(if_value) && (then_value.is_none() || !is_present(then_value.unwrap())) {
        return Some(ConstraintViolation {
            constraint: Constraint::implication(if_field, then_field),
            message: format!("{} is set but {} is required", if_field, then_field),
        });
    }
    None
}

fn validate_mutex(fields: &[String], args: &Value) -> Option<ConstraintViolation> {
    let present: Vec<_> = fields.iter().filter(|f| {
        get_field(args, f).map(is_present).unwrap_or(false)
    }).collect();
    if present.len() > 1 {
        return Some(ConstraintViolation {
            constraint: Constraint::mutex(fields.to_vec()),
            message: format!("only one of [{}] may be set, but {} are present", fields.join(", "), present.len()),
        });
    }
    None
}

fn validate_require_one(fields: &[String], args: &Value) -> Option<ConstraintViolation> {
    let any_present = fields.iter().any(|f| {
        get_field(args, f).map(is_present).unwrap_or(false)
    });
    if !any_present {
        return Some(ConstraintViolation {
            constraint: Constraint::require_one(fields.to_vec()),
            message: format!("at least one of [{}] must be set", fields.join(", ")),
        });
    }
    None
}

fn validate_range(field: &str, min: Option<f64>, max: Option<f64>, args: &Value) -> Option<ConstraintViolation> {
    let value = get_field(args, field)?;
    let num = value.as_f64().or_else(|| value.as_i64().map(|n| n as f64))?;
    if let Some(min_val) = min {
        if num < min_val {
            return Some(ConstraintViolation {
                constraint: Constraint::range(field, min, max),
                message: format!("{} must be >= {}", field, min_val),
            });
        }
    }
    if let Some(max_val) = max {
        if num > max_val {
            return Some(ConstraintViolation {
                constraint: Constraint::range(field, min, max),
                message: format!("{} must be <= {}", field, max_val),
            });
        }
    }
    None
}

fn validate_pattern(field: &str, pattern: &str, args: &Value) -> Option<ConstraintViolation> {
    let value = get_field(args, field)?;
    let s = value.as_str()?;
    if let Ok(re) = regex::Regex::new(pattern) {
        if !re.is_match(s) {
            return Some(ConstraintViolation {
                constraint: Constraint::pattern(field, pattern),
                message: format!("{} must match pattern {}", field, pattern),
            });
        }
    }
    None
}

/// Validate a single constraint against tool arguments.
pub fn validate_constraint(constraint: &Constraint, args: &Value) -> Option<ConstraintViolation> {
    match constraint {
        Constraint::Implication { if_field, then_field } => validate_implication(if_field, then_field, args),
        Constraint::Mutex { fields } => validate_mutex(fields, args),
        Constraint::RequireOne { fields } => validate_require_one(fields, args),
        Constraint::Range { field, min, max } => validate_range(field, *min, *max, args),
        Constraint::Pattern { field, pattern } => validate_pattern(field, pattern, args),
    }
}

/// Validate all constraints against tool arguments.
pub fn validate(args: &Value, constraints: &[Constraint]) -> ValidationResult {
    let mut result = ValidationResult::valid();
    for constraint in constraints {
        if let Some(violation) = validate_constraint(constraint, args) {
            result.violations.push(violation);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implication_violated_when_a_true_b_missing() {
        let constraint = Constraint::implication("auto_background", "enabled_background");
        let args = serde_json::json!({"auto_background": true});
        let result = validate_constraint(&constraint, &args);
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("enabled_background"));
    }

    #[test]
    fn implication_passes_when_a_true_b_present() {
        let constraint = Constraint::implication("auto_background", "enabled_background");
        let args = serde_json::json!({"auto_background": true, "enabled_background": true});
        assert!(validate_constraint(&constraint, &args).is_none());
    }

    #[test]
    fn implication_passes_when_a_false() {
        let constraint = Constraint::implication("auto_background", "enabled_background");
        let args = serde_json::json!({"auto_background": false});
        assert!(validate_constraint(&constraint, &args).is_none());
    }

    #[test]
    fn mutex_violated_when_multiple_present() {
        let constraint = Constraint::mutex(vec!["a".into(), "b".into(), "c".into()]);
        let args = serde_json::json!({"a": 1, "b": 2});
        let result = validate_constraint(&constraint, &args);
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("only one"));
    }

    #[test]
    fn mutex_passes_when_one_present() {
        let constraint = Constraint::mutex(vec!["a".into(), "b".into(), "c".into()]);
        let args = serde_json::json!({"a": 1});
        assert!(validate_constraint(&constraint, &args).is_none());
    }

    #[test]
    fn mutex_passes_when_none_present() {
        let constraint = Constraint::mutex(vec!["a".into(), "b".into(), "c".into()]);
        let args = serde_json::json!({});
        assert!(validate_constraint(&constraint, &args).is_none());
    }

    #[test]
    fn require_one_violated_when_none_present() {
        let constraint = Constraint::require_one(vec!["a".into(), "b".into()]);
        let args = serde_json::json!({"other": "value"});
        let result = validate_constraint(&constraint, &args);
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("at least one"));
    }

    #[test]
    fn require_one_passes_when_one_present() {
        let constraint = Constraint::require_one(vec!["a".into(), "b".into()]);
        let args = serde_json::json!({"a": 1});
        assert!(validate_constraint(&constraint, &args).is_none());
    }

    #[test]
    fn range_violated_when_below_min() {
        let constraint = Constraint::range("timeout", Some(1.0), Some(60.0));
        let args = serde_json::json!({"timeout": 0.5});
        let result = validate_constraint(&constraint, &args);
        assert!(result.is_some());
        assert!(result.unwrap().message.contains(">="));
    }

    #[test]
    fn range_violated_when_above_max() {
        let constraint = Constraint::range("timeout", Some(1.0), Some(60.0));
        let args = serde_json::json!({"timeout": 100.0});
        let result = validate_constraint(&constraint, &args);
        assert!(result.is_some());
        assert!(result.unwrap().message.contains("<="));
    }

    #[test]
    fn range_passes_when_within_bounds() {
        let constraint = Constraint::range("timeout", Some(1.0), Some(60.0));
        let args = serde_json::json!({"timeout": 30.0});
        assert!(validate_constraint(&constraint, &args).is_none());
    }

    #[test]
    fn range_passes_when_field_missing() {
        let constraint = Constraint::range("timeout", Some(1.0), Some(60.0));
        let args = serde_json::json!({});
        assert!(validate_constraint(&constraint, &args).is_none());
    }

    #[test]
    fn pattern_violated_when_no_match() {
        let constraint = Constraint::pattern("path", r"^/");
        let args = serde_json::json!({"path": "relative/path"});
        assert!(validate_constraint(&constraint, &args).is_some());
    }

    #[test]
    fn pattern_passes_when_matches() {
        let constraint = Constraint::pattern("path", r"^/");
        let args = serde_json::json!({"path": "/absolute/path"});
        assert!(validate_constraint(&constraint, &args).is_none());
    }

    #[test]
    fn validate_returns_valid_for_no_constraints() {
        let args = serde_json::json!({"path": "/test"});
        assert!(validate(&args, &[]).is_valid());
    }

    #[test]
    fn validate_collects_multiple_violations() {
        let constraints = vec![
            Constraint::implication("a", "b"),
            Constraint::require_one(vec!["c".into(), "d".into()]),
        ];
        let args = serde_json::json!({"a": true});
        let result = validate(&args, &constraints);
        assert!(!result.is_valid());
        assert_eq!(result.violations.len(), 2);
    }

    #[test]
    fn validation_result_merge() {
        let mut result = ValidationResult::valid();
        result.merge(ValidationResult::violated(Constraint::implication("a", "b"), "a implies b"));
        assert!(!result.is_valid());
        assert_eq!(result.violations.len(), 1);
    }
}
