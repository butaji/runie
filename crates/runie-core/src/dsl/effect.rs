//! Effect types for the declarative DSL.
//!
//! Effects are fire-and-forget IO requests sent to actors.
//! They do **not** block the flow.

use crate::dsl::runtime::Runtime;

/// A pure side-effect (clipboard, IO request, etc.) executed at runtime.
///
/// Effects are boxed closures that are executed when the flow runs.
/// They are fire-and-forget — the flow continues immediately after.
pub type Effect = Box<dyn Fn(&mut dyn Runtime) + Send>;

/// Create an effect from a closure.
pub fn effect<F>(f: F) -> Effect
where
    F: Fn(&mut dyn Runtime) + Send + 'static,
{
    Box::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::runtime::TestRuntime;

    #[test]
    fn effect_runs_in_test_runtime() {
        let mut rt = TestRuntime::new();
        let flag = std::sync::Arc::new(std::sync::Mutex::new(false));
        let flag_clone = flag.clone();
        let eff = effect(move |_| *flag_clone.lock().unwrap() = true);
        eff(&mut rt);
        assert!(*flag.lock().unwrap());
    }
}
