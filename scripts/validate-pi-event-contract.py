#!/usr/bin/env python3
"""Check Runie's generated Pi event boundary against Pi's AgentEvent union."""

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
PI_TYPES = Path("/Users/admin/Code/agents/pi/packages/agent/src/types.ts")
RUST_CONTRACT = ROOT / "crates/runie-core/src/pi_event.rs"


def pi_events(source: str) -> list[str]:
    body = source.split("export type AgentEvent =", 1)[1].split("};", 1)[0]
    return re.findall(r'type:\s*"([a-z_]+)"', body)


def runie_events(source: str) -> list[str]:
    body = source.split("pi_event_contract! {", 1)[1].split("}\n\n#[cfg(test)]", 1)[0]
    units = re.search(r"unit\s*\{([^}]*)\}", body, re.S)
    if units is None:
        raise ValueError("missing unit declaration")
    names = [name.strip() for name in units.group(1).split(",") if name.strip()]
    names.extend(re.findall(r"^\s{8}([A-Z][A-Za-z0-9_]*)\s*\{", body, re.M))
    return [re.sub(r"(?<!^)([A-Z])", r"_\1", name).lower() for name in names]


def main() -> int:
    if not PI_TYPES.is_file():
        print(f"pi-event-contract: upstream source unavailable, skipped: {PI_TYPES}")
        return 0
    expected = pi_events(PI_TYPES.read_text())
    actual = runie_events(RUST_CONTRACT.read_text())
    if sorted(expected) != sorted(actual):
        raise ValueError(f"event union drift: expected {expected}, got {actual}")
    print(f"pi-event-contract: {len(actual)} Pi event names match upstream AgentEvent")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, IndexError, ValueError) as error:
        print(f"pi-event-contract: {error}")
        raise SystemExit(1)
