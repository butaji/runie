//! Macros for declarative command handler registration.

#[macro_export]
macro_rules! handlers {
    ($registry:ident, $($name:literal => $handler:expr),* $(,)?) => {
        pub fn register_handlers($registry: &mut crate::commands::dsl::handlers::HandlerRegistry) {
            $( $registry.register($name, crate::commands::dsl::handlers::NamedHandler::Handler($handler)); )*
        }
    };
}

#[macro_export]
macro_rules! handlers_form {
    ($registry:ident, $($name:literal => ($title:expr, $fields:expr, $handler:expr)),* $(,)?) => {
        pub fn register_handlers($registry: &mut crate::commands::dsl::handlers::HandlerRegistry) {
            $( $registry.register(
                $name,
                crate::commands::dsl::handlers::NamedHandler::FormWithHandler { title: $title, fields: $fields, handler: $handler },
            ); )*
        }
    };
}
