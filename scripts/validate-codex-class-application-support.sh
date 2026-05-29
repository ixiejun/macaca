#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MACACA_DIR="$ROOT_DIR/macaca"
CHANGE_ID="complete-codex-class-application-support"

# This script is a validation facade for the Codex-class application support
# proposal. It intentionally owns no product behavior: each step delegates to
# existing OpenSpec, Rust, shell, and frontend gates so audit evidence points at
# the service or boundary that owns the behavior under test.
log() {
  printf 'codex_class_validation event=%s detail=%s\n' "$1" "$2"
}

run_step() {
  local name="$1"
  shift
  log "step_start" "$name"
  "$@"
  log "step_complete" "$name"
}

run_cargo() {
  local name="$1"
  shift
  run_step "$name" cargo test --manifest-path "$MACACA_DIR/Cargo.toml" "$@"
}

run_step "openspec:${CHANGE_ID}" openspec validate "$CHANGE_ID" --strict

run_cargo "workbench_contracts:proto" -p macaca-proto workbench
run_cargo "workbench_contracts:sdk" -p macaca-sdk workbench
run_cargo "workbench_contracts:manifest" -p macaca-app --test workbench_manifest

# Runtime-host owns the built-in provider adapters for the workbench service
# families. Running this package with service-family filters proves provider
# behavior without forcing unrelated crates into every gate.
for service_filter in \
  interaction_service \
  app_protocol_service \
  file_service \
  process_service \
  sandbox_service \
  approval_service \
  hook_service \
  config_service \
  plugin_marketplace_service \
  skill_operator_lifecycle \
  code_intelligence_service \
  git_service \
  review_service \
  diagnostics_realtime_remote_service
do
  run_cargo "workbench_service_families:${service_filter}" -p macaca-runtime-host "$service_filter"
done
run_cargo "workbench_service_families:mcp_runtime_regression" -p macaca-runtime-host --test mcp_runtime_regression

run_cargo "boundary:route_c_dependency" -p macaca-integration-tests --test route_c_dependency_boundaries
run_cargo "boundary:serviceization_escape_hatches" -p macaca-integration-tests --test serviceization_escape_hatches
run_cargo "boundary:codex_class_scope_control" -p macaca-integration-tests --test codex_class_scope_control
run_cargo "boundary:tool_service" -p macaca-integration-tests --test tool_service_boundaries
run_cargo "boundary:self_evolving_skill" -p macaca-integration-tests --test self_evolving_skill_boundaries

run_cargo "audit_replay:codex_class_proof" -p macaca-integration-tests --test codex_class_application_neutral_proof
run_cargo "audit_replay:skill_governance" -p macaca-skill governance_audit_replay
run_cargo "audit_replay:approval" -p macaca-runtime-host approval_service
run_cargo "audit_replay:plugin_marketplace" -p macaca-runtime-host plugin_marketplace_service
run_cargo "audit_replay:git" -p macaca-runtime-host git_service
run_cargo "audit_replay:review" -p macaca-runtime-host review_service
run_cargo "audit_replay:diagnostics" -p macaca-runtime-host diagnostics_realtime_remote_service

run_cargo "api_regression:route_c_baseline" -p macaca-integration-tests --test route_c_baseline
run_cargo "application_regression:app_declarative" -p macaca-integration-tests --test app_declarative
run_cargo "application_regression:application_platform_contracts" -p macaca-integration-tests --test application_platform_contracts
run_cargo "application_regression:wasm_runtime_provider" -p macaca-runtime-host wasm_runtime_provider
run_cargo "industrial_regression:tool_system" -p macaca-integration-tests --test industrial_tool_system

if [[ "${RUN_LIVE_API_PROOF:-0}" == "1" ]]; then
  if [[ -z "${MACACA_LIVE_API_PROOF_CMD:-}" ]]; then
    echo "RUN_LIVE_API_PROOF=1 requires MACACA_LIVE_API_PROOF_CMD." >&2
    exit 1
  fi
  # The live proof is intentionally injected as an operator command so the gate
  # can use the current deployment topology without hardcoding provider, model,
  # application, or environment names into this repository script.
  run_step "live_api_proof" bash -lc "$MACACA_LIVE_API_PROOF_CMD"
else
  log "live_api_proof_skipped" "set RUN_LIVE_API_PROOF=1 and MACACA_LIVE_API_PROOF_CMD to run deployment-specific API proof"
fi

if ! git -C "$ROOT_DIR" diff --quiet -- frontend; then
  run_step "frontend:lint" npm --prefix "$ROOT_DIR/frontend" run lint
  run_step "frontend:build" npm --prefix "$ROOT_DIR/frontend" run build
else
  log "frontend_skipped" "no frontend diff detected"
fi

log "validation_complete" "$CHANGE_ID"
