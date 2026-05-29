#!/usr/bin/env bash
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="$APP_DIR/app.yaml"
ARTIFACT="$APP_DIR/dist/component/codex_wasm_workbench.wasm"
RUNTIME_ARTIFACT="$APP_DIR/component.wasm"

log() {
  printf 'codex_wasm_workbench validation event=%s detail=%s\n' "$1" "$2"
}

if [[ "$APP_DIR" == *"/macaca/"* ]]; then
  echo "application package must not live under macaca/ code directory" >&2
  exit 1
fi
log "location_ok" "$APP_DIR"

test -f "$MANIFEST"
log "manifest_found" "$MANIFEST"

for service in \
  service.interaction \
  service.app_protocol \
  service.file \
  service.process \
  service.sandbox \
  service.approval \
  service.hook \
  service.config \
  service.code_intelligence \
  service.git \
  service.review \
  service.diagnostics \
  service.tool
do
  grep -q "service: $service" "$MANIFEST"
  log "service_declared" "$service"
done

grep -q "layer: L2Wasm" "$MANIFEST"
grep -q "runtime: web_bundle" "$MANIFEST"
grep -q "schema: genui.workbench.codex_wasm.v1" "$MANIFEST"
log "manifest_shape_ok" "workbench_and_ui_declared"

test -f "$ARTIFACT"
log "wasm_artifact_found" "$ARTIFACT"

test -f "$RUNTIME_ARTIFACT"
log "runtime_artifact_found" "$RUNTIME_ARTIFACT"

log "complete" "package_ready_for_macaca_admission_checks"
