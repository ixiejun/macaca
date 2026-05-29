#!/usr/bin/env bash
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPONENT_DIR="$APP_DIR/component"
OUT_DIR="$APP_DIR/dist/component"
RUNTIME_ARTIFACT="$APP_DIR/component.wasm"

echo "codex_wasm_workbench build event=start app_dir=$APP_DIR"

if [[ "$APP_DIR" == *"/macaca/"* ]]; then
  echo "codex_wasm_workbench build event=error reason=app_must_not_live_under_macaca_code_dir" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
cargo build \
  --manifest-path "$COMPONENT_DIR/Cargo.toml" \
  --target wasm32-unknown-unknown \
  --release

cp "$COMPONENT_DIR/target/wasm32-unknown-unknown/release/codex_wasm_workbench_component.wasm" \
  "$OUT_DIR/codex_wasm_workbench.wasm"

cat >> "$OUT_DIR/codex_wasm_workbench.wasm" <<'EOF'
macaca:component-model:v1
export=app:start
wit=macaca:application/runtime@1
host-command={"import":"service_call","payload":{"workspace_root":"."},"trace":null,"metadata":{"service.id":"service.git","service.operation":"git.status","capability":"service.call"}}
EOF

cp "$OUT_DIR/codex_wasm_workbench.wasm" "$RUNTIME_ARTIFACT"

echo "codex_wasm_workbench build event=complete artifact=$OUT_DIR/codex_wasm_workbench.wasm"
echo "codex_wasm_workbench build event=runtime_artifact_ready artifact=$RUNTIME_ARTIFACT"
