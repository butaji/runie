#!/usr/bin/env python3
"""Validate the source-backed parity/tui component manifest."""

import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "parity/tui/index.json"
FIXTURES = ROOT / "crates/runie-tui/tests/e2e"


def main() -> int:
    manifest = json.loads(MANIFEST.read_text())
    components = manifest.get("components")
    if not isinstance(components, list) or not components:
        raise ValueError("components must be a non-empty array")

    reference_root = Path(manifest["reference_root"])
    ids = set()
    for component in components:
        component_id = component["id"]
        if component_id in ids:
            raise ValueError(f"duplicate component id: {component_id}")
        ids.add(component_id)

        doc = ROOT / "parity/tui" / component["doc"]
        if not doc.is_file():
            raise ValueError(f"{component_id}: missing doc {doc}")
        for source in component["sources"]:
            path = reference_root / source
            if reference_root.exists() and not path.is_file() and not path.is_dir():
                raise ValueError(f"{component_id}: missing source {path}")
        for fixture in component["fixtures"]:
            path = FIXTURES / fixture
            if not path.is_file():
                raise ValueError(f"{component_id}: missing fixture {path}")

    print(f"parity-index: {len(components)} components validated")
    if not reference_root.exists():
        print(f"parity-index: reference root unavailable, source checks skipped: {reference_root}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        print(f"parity-index: {error}", file=sys.stderr)
        raise SystemExit(1)
