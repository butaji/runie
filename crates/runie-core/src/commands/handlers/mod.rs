use super::CommandRegistry;

pub fn register_all(registry: &mut CommandRegistry) {
    session::register(registry);
    model::register(registry);
    tool::register(registry);
    system::register(registry);
    help::register(registry);
}

pub mod session;
pub mod model;
pub mod tool;
pub mod system;
pub mod help;
