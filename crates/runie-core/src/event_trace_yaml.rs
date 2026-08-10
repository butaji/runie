//! YAML-backed event traces for pure reducer and replay tests.

use crate::EventMemo;
use serde::de::DeserializeOwned;

/// Replay a YAML sequence of events through the same memo used by runtime
/// actors. Keeping parsing outside the reducer preserves an inspectable,
/// event-only test contract.
pub fn replay_yaml<S, E>(
    yaml: &str,
    initial: S,
    reduce: impl Fn(&mut S, &E),
) -> Result<EventMemo<E, S>, serde_yaml::Error>
where
    E: DeserializeOwned,
{
    let events = serde_yaml::from_str::<Vec<E>>(yaml)?;
    Ok(EventMemo::replay(initial, events, reduce))
}

#[cfg(test)]
mod tests {
    use super::replay_yaml;

    #[test]
    fn yaml_trace_replays_in_declared_order() {
        let trace = replay_yaml::<i32, i32>("- 2\n- 3\n", 10, |state, event| {
            *state += event;
        })
        .expect("valid event trace");
        assert_eq!(trace.state(), &15);
        assert_eq!(trace.events(), &[2, 3]);
    }

    #[test]
    fn malformed_yaml_is_reported_without_running_the_reducer() {
        let trace = replay_yaml::<i32, i32>("not: a sequence", 10, |state, event| {
            *state += event;
        });
        assert!(trace.is_err());
    }
}
