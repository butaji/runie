#!/bin/bash
# Workspace structural linter.
# Enforces 500-line file limit, 40-line function limit, and approximate
# complexity <= 10 on all production .rs files.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "${script_dir}/check_structure.py"
