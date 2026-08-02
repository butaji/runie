//! Macros for declarative command handler registration.

/// `handlers!` generates a scoped `pub fn register_handlers` that accepts both
/// named fn references and inline closures without boxing — zero-cost abstraction.
/// Hand-writing `register_handlers` per module (Option A) was considered but would
/// add ~15–20 lines of boilerplate per call site without meaningful benefit.
/// Keep as Option B: the macro is live, not harmful, and the tradeoff between
/// verbosity and zero-cost closure acceptance favors keeping it.
#[macro_export]
macro_rules! handlers {
    ($registry:ident, $($name:literal => $handler:expr),* $(,)?) => {
        pub fn register_handlers($registry: &mut crate::commands::dsl::handlers::HandlerRegistry) {
            $( $registry.register($name, crate::commands::dsl::handlers::NamedHandler::Handler($handler)); )*
        }
    };
}
