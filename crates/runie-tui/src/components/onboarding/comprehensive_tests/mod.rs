//! Comprehensive tests for the onboarding component.
//!
//! Split into submodules by onboarding step:
//! - welcome: Welcome step transitions and navigation
//! - provider: Provider selection and filtering
//! - model: Model fetching and selection
//! - key: API key input and validation
//! - complete: Completion step and settings save

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod welcome;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod provider;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod model;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod key;

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod complete;
