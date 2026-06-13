use super::CommandRegistry;

pub fn register_all(registry: &mut CommandRegistry) {
    session::register(registry);
    model::register(registry);
    tool::register(registry);
    system::register(registry);
    subagent::register(registry);
    agents::register(registry);
    help::register(registry);
}

pub mod agents;
pub mod help;
pub mod model;
pub mod session;
pub mod subagent;
pub mod system;
pub mod tool;
