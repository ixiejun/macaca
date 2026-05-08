#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# This guard is intentionally conservative for Phase 12. It checks for obvious
# new cross-layer coupling where CLI code imports Web internals beyond the
# existing server-start boundary. Later migration phases can tighten it once
# more direct Web/CLI semantic helpers are deprecated.
if rg -n "macaca_web::(routes|session|sse|chat_orchestrator|framework_runner|loop_manager)" \
  "$ROOT_DIR/macaca/crates/macaca-cli/src"; then
  echo "CLI must not depend on macaca-web internal semantic modules." >&2
  exit 1
fi

if rg -n "app_id\\s*==|workflow\\s*==|driver_name\\s*==|gateway\\s*==" \
  "$ROOT_DIR/macaca/crates/macaca-web/src/shell.rs" \
  "$ROOT_DIR/macaca/crates/macaca-sdk/src/system_facade.rs"; then
  echo "Thin shell facade files must not branch on application/provider names." >&2
  exit 1
fi
