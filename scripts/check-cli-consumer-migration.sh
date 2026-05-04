#!/usr/bin/env bash
# Verify upper consumers do not call deprecated macaca-cli Rust helpers.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

fail() {
  printf 'ERROR: %s\n' "$1" >&2
  exit 1
}

if ! command -v rg >/dev/null 2>&1; then
  fail "ripgrep (rg) is required"
fi

deprecated_call_pattern='macaca_cli::(run_kernel|list_agents|show_status|create_kernel)'
deprecated_import_pattern='use[[:space:]]+macaca_cli::.*(run_kernel|list_agents|show_status|create_kernel)'

if rg -n "$deprecated_call_pattern" macaca/crates macaca/tests scripts \
  -g '*.rs' -g '*.sh' -g '!**/target/**'; then
  fail "deprecated macaca_cli helper call found"
fi

if rg -n "$deprecated_import_pattern" macaca/crates macaca/tests \
  -g '*.rs' -g '!**/target/**'; then
  fail "deprecated macaca_cli helper import found"
fi

unexpected_manifest_refs="$(
  rg -n 'macaca-cli[[:space:]]*=' macaca -g 'Cargo.toml' \
    | awk -F: '$1 != "macaca/Cargo.toml" && $1 != "macaca/crates/macaca-cli/Cargo.toml" { print }'
)"

if [[ -n "$unexpected_manifest_refs" ]]; then
  printf '%s\n' "$unexpected_manifest_refs" >&2
  fail "unexpected Cargo dependency on macaca-cli found"
fi

printf 'macaca-cli consumer migration guard passed\n'
