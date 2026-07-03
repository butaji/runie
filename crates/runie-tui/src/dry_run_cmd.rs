//! Dry-run / preview flag handling for the TUI binary.

use runie_core::{run_dry_run, Config, DryRunReport};

/// If `dry` is true, load config and return a preview report without starting
/// the terminal UI.
///
/// This is a non-production synchronous fallback: it validates configuration
/// without making API calls and without spawning actors. In the interactive
/// path, provider construction is handled exclusively by `ProviderActor`.
pub fn run_dry_run_if_requested(dry: bool) -> Option<DryRunReport> {
    if !dry {
        return None;
    }
    let config = Config::load(None);
    Some(run_dry_run(&config))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_flag_triggers_preview_mode() {
        let report = run_dry_run_if_requested(true);
        assert!(report.is_some());
    }

    #[test]
    fn no_dry_run_flag_returns_none() {
        let report = run_dry_run_if_requested(false);
        assert!(report.is_none());
    }
}
