//! Helper functions for command ranking and post copy extraction.

use crate::state::CommandUsage;
use crate::view::elements::Element;

/// Compute a ranking score boost from usage count and recency.
/// Score is scaled so it doesn't dominate the fuzzy score.
pub(crate) fn compute_ranking_score(
    _query: &str,
    _cmd: &crate::commands::CommandDef,
    usage: Option<&CommandUsage>,
) -> i32 {
    let usage_boost = usage.map(|u| u.count as i32).unwrap_or(0);
    // Recency: commands used in the last 5 minutes get a bonus that decays.
    // Stored as Unix timestamp; compare against current time.
    let now = crate::update::now();
    let recency_boost = usage
        .map(|u| {
            let age = now - u.last_used;
            if age < 300.0 {
                // Exponential decay: max bonus at t=0, zero at t=300s
                ((300.0 - age) / 300.0 * 10.0) as i32
            } else {
                0
            }
        })
        .unwrap_or(0);

    // Usage boosts are small (1–10), recency boosts also small.
    // This is added to fuzzy score * 100, so it only breaks ties.
    usage_boost + recency_boost
}

/// Extract plain text from an Element for `y` copy.
pub(crate) fn element_text(elem: &Element) -> Option<String> {
    match elem {
        Element::UserMessage { content, .. } => Some(content.clone()),
        Element::AgentMessage { content, .. } => Some(content.clone()),
        Element::ThoughtSummary { content, .. } => Some(content.clone()),
        Element::ThoughtMarker { content, .. } => Some(content.clone()),
        Element::ToolRunning { name, args, .. } => {
            if args.is_empty() {
                Some(name.clone())
            } else {
                Some(format!("{} {}", name, args))
            }
        }
        Element::ToolDone {
            name, args, output, ..
        } => {
            let head = if args.is_empty() {
                name.clone()
            } else {
                format!("{} {}", name, args)
            };
            if output.is_empty() {
                Some(head)
            } else {
                Some(format!(
                    "{} {}\n{}",
                    head,
                    output,
                    if output.ends_with('\n') { "" } else { "\n" }
                ))
            }
        }
        _ => None,
    }
}

/// Extract short metadata string from an Element for `Y` (copy metadata).
pub(crate) fn element_metadata(elem: &Element) -> Option<String> {
    match elem {
        Element::UserMessage { timestamp, .. } => Some(format!("user {:.0}s", timestamp)),
        Element::AgentMessage {
            provider,
            timestamp,
            ..
        } => Some(format!("{} {:.0}s", provider, timestamp)),
        Element::Thinking { timestamp, .. } => Some(format!("thinking {:.0}s", timestamp)),
        Element::ThoughtSummary {
            duration_secs,
            timestamp,
            ..
        } => Some(thought_metadata(*timestamp, *duration_secs)),
        Element::ToolRunning {
            name, timestamp, ..
        } => Some(format!("{} running at {:.0}s", name, timestamp)),
        Element::ToolDone {
            name,
            duration_secs,
            timestamp,
            ..
        } => Some(tool_done_metadata(name, *duration_secs, *timestamp)),
        Element::ToolSummary {
            name,
            duration_secs,
            timestamp,
            ..
        } => Some(tool_summary_metadata(name, *duration_secs, *timestamp)),
        Element::TurnComplete {
            duration_secs,
            timestamp,
            ..
        } => Some(format!("turn {:.1}s at {:.0}s", duration_secs, timestamp)),
        _ => None,
    }
}

fn thought_metadata(timestamp: f64, duration_secs: f64) -> String {
    format!("thought {:.0}s → {:.1}s", timestamp, duration_secs)
}

fn tool_done_metadata(name: &str, duration_secs: f64, timestamp: f64) -> String {
    format!(
        "{} done in {:.1}s at {:.0}s",
        name, duration_secs, timestamp
    )
}

fn tool_summary_metadata(name: &str, duration_secs: f64, timestamp: f64) -> String {
    format!("{} {:.1}s at {:.0}s", name, duration_secs, timestamp)
}
