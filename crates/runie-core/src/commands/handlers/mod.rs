use super::{CommandRegistry};

pub fn register_all(registry: &mut CommandRegistry) {
    session::register(registry);
    model::register(registry);
    tool::register(registry);
    system::register(registry);
    help::register(registry);
}

mod session;
mod model;
mod tool;
mod system;
mod help;
