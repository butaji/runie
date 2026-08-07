#!/usr/bin/env python3
"""Guard the actor/model/render boundary for the feed reducer."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ACTOR = ROOT / "crates/runie-tui/src/scrollback_actor.rs"
RENDERER = ROOT / "crates/runie-tui/src/event_renderer.rs"


def main() -> int:
    source = ACTOR.read_text(encoding="utf-8")
    renderer = RENDERER.read_text(encoding="utf-8")
    forbidden = {
        "Scrollback::new(": "actor must construct FeedState, not the widget",
        "state.apply(": "actor must reduce through FeedState::reduce",
        "state.apply_batch(": "actor must reduce through FeedState::reduce",
        "crate::event_renderer": "actor must not depend on renderer-only projections",
        "ratatui::": "actor must not depend on terminal rendering types",
        "crossterm::": "actor must not depend on terminal input types",
    }
    failures = [f"{needle}: {reason}" for needle, reason in forbidden.items() if needle in source]
    renderer_forbidden = {
        "pub fn apply_event(": "renderer delivery must cross an actor mailbox",
        "Projection::Legacy": "renderer must not construct a mutex-owned projection",
        "Legacy(Arc<Mutex": "renderer must not retain a legacy mutex projection type",
    }
    failures.extend(
        f"event_renderer.rs: {needle}: {reason}"
        for needle, reason in renderer_forbidden.items()
        if needle in renderer
    )
    if failures:
        for failure in failures:
            print(f"feed-actor-boundary: {failure}")
        return 1
    if "FeedState" not in source or "state.reduce(message)" not in source:
        print("feed-actor-boundary: actor is missing the FeedState reduction seam")
        return 1
    print("feed-actor-boundary: ScrollbackActor reduces FeedState and only rehydrates the renderer")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
