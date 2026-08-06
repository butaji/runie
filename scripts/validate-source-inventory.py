#!/usr/bin/env python3
"""Validate the checked-in counts for the authoritative Pi/Grok source scan."""

from collections import Counter
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent
CONTRACT = ROOT / "tasks/source-inventory-contract.json"
INVENTORY = ROOT / "scripts/source-inventory.sh"


def main() -> int:
    expected = json.loads(CONTRACT.read_text())
    output = subprocess.run(
        [str(INVENTORY)],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()

    counts = Counter(
        line.split("\t", 1)[0]
        for line in output
        if "\t" in line and line.split("\t", 1)[0] in expected
    )
    actual = {label: count for label, count in counts.items()}
    if actual != expected:
        raise ValueError(f"source inventory drift: expected {expected}, got {actual}")

    print("source-inventory: " + ", ".join(f"{label}={expected[label]}" for label in expected))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, subprocess.CalledProcessError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        print(f"source-inventory: {error}", file=sys.stderr)
        raise SystemExit(1)
