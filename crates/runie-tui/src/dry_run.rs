//! Dry-run / preview flag handling for the TUI binary.

use runie_core::{run_dry_run, Config, DryRunReport};

/// If the argument list requests a dry run, load config and return a preview
/// report without starting the terminal UI.
pub fn run_from_args(args: &[String]) -> Option<DryRunReport> {
    let dry = args.iter().any(|a| a == "--dry-run" || a == "--preview");
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
        let args = vec!["runie".to_string(), "--dry-run".to_string()];
        let report = run_from_args(&args);
        assert!(report.is_some());
    }

    #[test]
    fn no_dry_run_flag_returns_none() {
        let args = vec!["runie".to_string(), "hello".to_string()];
        let report = run_from_args(&args);
        assert!(report.is_none());
    }
}
