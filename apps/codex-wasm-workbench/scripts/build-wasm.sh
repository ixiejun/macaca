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

# The portable Component Model adapter reads declarative metadata appended to
# the WASM artifact.  This application-owned command chain keeps Codex
# collaboration semantics inside the Workbench package: the OS only sees
# provider-neutral service calls and agent delegation commands, while the
# coordinator model decides task complexity and downstream collaboration depth.
cat >> "$OUT_DIR/codex_wasm_workbench.wasm" <<'EOF'
macaca:component-model:v1
export=app:start
wit=macaca:application/runtime@1
host-command={"import":"task_create_goal","payload":{"description":"CODEX-WASM-WORKBENCH task: ${chat.input}"},"trace":null,"metadata":{"capability":"task.manage","workbench.workflow_step":"task_board_admission","workbench.collaboration_contract":"macaca_task_service_goal"}}
host-command={"import":"service_call","payload":{"workspace_root":"."},"trace":null,"metadata":{"service.id":"service.git","service.operation":"git.status","capability":"service.call"}}
host-command={"import":"agent_delegate","payload":{"target_agent":"coordinator","prompt":"Coordinate this application-owned coding task. Use model judgment to classify complexity as simple, standard, or deep from the actual request and workspace context; do not use keyword lists, programming-language shortcuts, application-name branches, or business-domain heuristics. Return a concise collaboration plan with handoffs for planner, coder, and reviewer. User task: ${chat.input}","context":{"user_input":"${chat.input}","workspace_ref":"${chat.workspace_ref}","application_execution_run_id":"${chat.run_id}","requested_capabilities":"${chat.requested_capabilities}","execution_source":"wasm_application_execution","task_goal":"${host.results.0.output}","workspace_status":"${host.results.1.output}","collaboration_step":"coordinator_complexity_assessment"}},"trace":null,"metadata":{"capability":"agent.delegate","wait_timeout_ms":"300000","policy.timeout_ms":"300000","priority":"9","application_execution.run_id":"${chat.run_id}","application_execution.provider_id":"provider.macaca_hosted","application_execution.provider_kind":"MacacaHosted","workbench.agent_role":"coordinator","workbench.workflow_step":"understand","workbench.collaboration_contract":"model_decided_complexity"}}
host-command={"import":"agent_delegate","payload":{"target_agent":"planner","prompt":"Plan this coding task from the coordinator's model-decided collaboration plan. Consume the coordinator output; keep the plan lightweight for simple tasks and add explicit validation/review checkpoints for standard or deep tasks. Do not introduce hardcoded complexity rules. User task: ${chat.input}","context":{"user_input":"${chat.input}","workspace_ref":"${chat.workspace_ref}","application_execution_run_id":"${chat.run_id}","requested_capabilities":"${chat.requested_capabilities}","execution_source":"wasm_application_execution","task_goal":"${host.results.0.output}","workspace_status":"${host.results.1.output}","coordinator_handoff":"${host.results.2.output}","collaboration_step":"planner_task_plan"}},"trace":null,"metadata":{"capability":"agent.delegate","wait_timeout_ms":"300000","policy.timeout_ms":"300000","priority":"8","application_execution.run_id":"${chat.run_id}","application_execution.provider_id":"provider.macaca_hosted","application_execution.provider_kind":"MacacaHosted","workbench.agent_role":"planner","workbench.workflow_step":"plan","workbench.collaboration_contract":"consume_coordinator_handoff"}}
host-command={"import":"agent_delegate","payload":{"target_agent":"coder","prompt":"Implement the application-owned coding task using the coordinator and planner handoffs. When the user asks for files or an application artifact, call file_write for each required file and place it under delegated_context.workspace.shared_path. Do not use shell for initial file creation, and do not only describe the code when a filesystem artifact is requested. Return files written, tool calls, validation status, and residual risks for reviewer consumption. User task: ${chat.input}","context":{"user_input":"${chat.input}","workspace_ref":"${chat.workspace_ref}","application_execution_run_id":"${chat.run_id}","requested_capabilities":"${chat.requested_capabilities}","execution_source":"wasm_application_execution","task_goal":"${host.results.0.output}","workspace_status":"${host.results.1.output}","coordinator_handoff":"${host.results.2.output}","planner_handoff":"${host.results.3.output}","collaboration_step":"coder_implementation"}},"trace":null,"metadata":{"capability":"agent.delegate","wait_timeout_ms":"300000","policy.timeout_ms":"300000","priority":"7","application_execution.run_id":"${chat.run_id}","application_execution.provider_id":"provider.macaca_hosted","application_execution.provider_kind":"MacacaHosted","workbench.agent_role":"coder","workbench.workflow_step":"implement","workbench.collaboration_contract":"consume_plan_and_write_artifacts"}}
host-command={"import":"agent_delegate","payload":{"target_agent":"reviewer","prompt":"Review the implemented task using the coordinator complexity assessment, planner handoff, and coder handoff. Match review depth to the model-decided complexity. Never invent validation evidence; clearly separate verified facts from residual risk. Return a user-facing review summary that is readable without raw JSON. User task: ${chat.input}","context":{"user_input":"${chat.input}","workspace_ref":"${chat.workspace_ref}","application_execution_run_id":"${chat.run_id}","requested_capabilities":"${chat.requested_capabilities}","execution_source":"wasm_application_execution","task_goal":"${host.results.0.output}","workspace_status":"${host.results.1.output}","coordinator_handoff":"${host.results.2.output}","planner_handoff":"${host.results.3.output}","coder_handoff":"${host.results.4.output}","collaboration_step":"reviewer_structured_review"}},"trace":null,"metadata":{"capability":"agent.delegate","wait_timeout_ms":"300000","policy.timeout_ms":"300000","priority":"6","application_execution.run_id":"${chat.run_id}","application_execution.provider_id":"provider.macaca_hosted","application_execution.provider_kind":"MacacaHosted","workbench.agent_role":"reviewer","workbench.workflow_step":"review","workbench.collaboration_contract":"consume_full_handoff_chain"}}
EOF

cp "$OUT_DIR/codex_wasm_workbench.wasm" "$RUNTIME_ARTIFACT"

echo "codex_wasm_workbench build event=complete artifact=$OUT_DIR/codex_wasm_workbench.wasm"
echo "codex_wasm_workbench build event=runtime_artifact_ready artifact=$RUNTIME_ARTIFACT"
