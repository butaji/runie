mod core;
mod router;
mod script;
mod tui;

use anyhow::Result;

fn main() -> Result<()> {
    tui::run()
}
