#!/usr/bin/env bash
set -euo pipefail

# Runs one daily Macaca self-evolution monitoring batch.
#
# The script is intentionally a shell adapter, not an OS semantic owner. It
# sends generic tasks through public Web APIs, captures bounded evidence, asks
# service-owned Skill operations to process proposals, and writes a sanitized
# daily report. The Skill service still owns proposal quality, materialization,
# curation, rollback, telemetry, and audit semantics.

API_URL="http://127.0.0.1:3001"
APP_ID=""
AGENT_NAME=""
DAY_NUMBER=""
TASKS_PER_DAY="6"
DEFAULT_STALE_DAYS="30"
FORCED_STALE_DAYS="0"
OUT_DIR="macaca/docs/self-evolution-monitoring"
MODEL=""
ENGINE="framework"
WAIT_SECONDS="30"

usage() {
  cat <<'USAGE'
Usage:
  scripts/run-self-evolution-daily-monitor.sh --app-id <APP_ID> --agent <AGENT_NAME> [options]

Options:
  --api <URL>                 Macaca Web API URL. Default: http://127.0.0.1:3001
  --app-id <APP_ID>           Application id used for the generic monitoring batch.
  --agent <AGENT_NAME>        Agent name used for skill visibility and operations.
  --day <N>                   Monitoring day number. Default: inferred from existing reports.
  --tasks-per-day <N>         Number of task families to run, 1-6. Default: 6.
  --default-stale-days <N>    Production curation stale threshold. Default: 30.
  --forced-stale-days <N>     Diagnostic stale threshold. Default: 0.
  --out-dir <PATH>            Monitoring report root. Default: macaca/docs/self-evolution-monitoring
  --model <MODEL>             Optional chat model name. Omitted by default.
  --engine <ENGINE>           Chat engine name passed through Web. Default: framework
  --wait-seconds <N>          Observer flush wait after task batch. Default: 30
  -h, --help                  Show this help.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --api) API_URL="$2"; shift 2 ;;
    --app-id) APP_ID="$2"; shift 2 ;;
    --agent) AGENT_NAME="$2"; shift 2 ;;
    --day) DAY_NUMBER="$2"; shift 2 ;;
    --tasks-per-day) TASKS_PER_DAY="$2"; shift 2 ;;
    --default-stale-days) DEFAULT_STALE_DAYS="$2"; shift 2 ;;
    --forced-stale-days) FORCED_STALE_DAYS="$2"; shift 2 ;;
    --out-dir) OUT_DIR="$2"; shift 2 ;;
    --model) MODEL="$2"; shift 2 ;;
    --engine) ENGINE="$2"; shift 2 ;;
    --wait-seconds) WAIT_SECONDS="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$APP_ID" || -z "$AGENT_NAME" ]]; then
  echo "ERROR: --app-id and --agent are required." >&2
  usage
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required." >&2
  exit 2
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "ERROR: curl is required." >&2
  exit 2
fi

if [[ -z "$DAY_NUMBER" ]]; then
  existing_count="$(find "$OUT_DIR/daily" -maxdepth 1 -type f -name '*.md' 2>/dev/null | wc -l | tr -d ' ')"
  DAY_NUMBER="$((existing_count + 1))"
fi

if [[ "$TASKS_PER_DAY" -lt 1 || "$TASKS_PER_DAY" -gt 6 ]]; then
  echo "ERROR: --tasks-per-day must be between 1 and 6." >&2
  exit 2
fi

RUN_DATE="$(date +%F)"
RUN_STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
DAY_LABEL="$(printf 'day-%02d' "$DAY_NUMBER")"
DAILY_ROOT="$OUT_DIR/daily"
EVIDENCE_DIR="$OUT_DIR/evidence/$RUN_DATE-$DAY_LABEL"
REPORT_PATH="$DAILY_ROOT/$RUN_DATE-$DAY_LABEL.md"

mkdir -p "$DAILY_ROOT" "$EVIDENCE_DIR"

# Writes a compact operations summary. Full snapshots are still preserved in
# evidence files, but the report only embeds aggregate fields.
summarize_operations() {
  local input_file="$1"
  jq '{
    proposals: .count.proposals,
    processing_records: .count.processing_records,
    processing_duplicate_groups: .count.processing_duplicate_groups,
    processing_ready: .count.processing_ready,
    processing_waiting_proposals: .count.processing_waiting_proposals,
    materialization_operator_runs: .count.materialization_operator_runs,
    materialization_operator_applied: .count.materialization_operator_applied,
    materialization_operator_denied: .count.materialization_operator_denied,
    curation_recommendations: .count.curation_recommendations,
    governance_records: .count.governance_records
  }' "$input_file"
}

capture_snapshot() {
  local label="$1"
  curl -sS "$API_URL/api/status" > "$EVIDENCE_DIR/status-$label.json" || true
  curl -sS "$API_URL/api/apps/$APP_ID/skills?agent=$AGENT_NAME" \
    > "$EVIDENCE_DIR/skills-$label.json" || true
  curl -sS "$API_URL/api/apps/$APP_ID/skills/operations?agent=$AGENT_NAME" \
    > "$EVIDENCE_DIR/operations-$label.json"
}

task_prompt() {
  local family="$1"
  local artifact="shared/self_evolution_${RUN_DATE}_${DAY_LABEL}_${family}.md"
  case "$family" in
    precipitation_seed)
      cat <<PROMPT
Daily self-evolution monitor ${DAY_LABEL}, task family precipitation_seed.
Create ${artifact}. Capture a reusable, application-neutral procedure from the current Macaca self-evolution evidence. Record only stable refs, bounded counts, and generic steps. Do not include application-specific business logic, provider names, raw prompts, secrets, or unbounded task output.
PROMPT
      ;;
    reuse_followup)
      cat <<PROMPT
Daily self-evolution monitor ${DAY_LABEL}, task family reuse_followup.
Create ${artifact}. Prefer visible workspace Skills if relevant. Verify whether previously materialized Skills are visible, useful, and associated with activation/use/success telemetry. Record exact skill names, stable refs, and telemetry deltas. Keep the artifact application-neutral.
PROMPT
      ;;
    optimization_probe)
      cat <<PROMPT
Daily self-evolution monitor ${DAY_LABEL}, task family optimization_probe.
Create ${artifact}. Review existing self-evolution Skills using telemetry-oriented criteria. If evidence supports an improvement, propose the generic optimization. If not, write a governed no-change decision with the missing metrics. Include rollback fields and audit refs. Keep all content application-neutral.
PROMPT
      ;;
    duplicate_merge_probe)
      cat <<PROMPT
Daily self-evolution monitor ${DAY_LABEL}, task family duplicate_merge_probe.
Create ${artifact}. Identify duplicate or overlapping Skills using slug similarity, trigger similarity, procedure overlap, and telemetry evidence. Recommend merge, suppress, or keep decisions with stable refs and rollback expectations. Do not mutate files directly and do not encode application-specific logic.
PROMPT
      ;;
    archive_readiness_probe)
      cat <<PROMPT
Daily self-evolution monitor ${DAY_LABEL}, task family archive_readiness_probe.
Create ${artifact}. Evaluate inactive Skill archive readiness. Separate real stale-threshold evidence from forced diagnostic stale-threshold evidence. Record candidate Skills, last-use evidence, protection blockers, rollback requirements, and whether archive is production-ready or diagnostic-only.
PROMPT
      ;;
    audit_summary)
      cat <<PROMPT
Daily self-evolution monitor ${DAY_LABEL}, task family audit_summary.
Create ${artifact}. Summarize today's self-evolution evidence using only stable refs and aggregate counts: task completion, proposal creation, materialization, later reuse, optimization, duplicate merge readiness, archive readiness, rollback refs, and API audit status. Keep the summary application-neutral.
PROMPT
      ;;
    *) echo "Unknown task family: $family" >&2; exit 2 ;;
  esac
}

run_task() {
  local index="$1"
  local family="$2"
  local prompt_file="$EVIDENCE_DIR/task-$index-$family.prompt.txt"
  local sse_file="$EVIDENCE_DIR/task-$index-$family.sse"

  task_prompt "$family" > "$prompt_file"

  # The SSE stream is stored verbatim as evidence. The report later extracts
  # only bounded event names, session ids, task ids, and proposal ids.
  curl --max-time 300 -sS -N \
    -H 'Content-Type: application/json' \
    -d "$(jq -n \
      --arg app_id "$APP_ID" \
      --arg prompt "$(cat "$prompt_file")" \
      --arg model "$MODEL" \
      --arg engine "$ENGINE" \
      '{
        app_id:$app_id,
        prompt:$prompt,
        session_id:null,
        engine:$engine
      } + (if $model == "" then {} else {model:$model} end)')" \
    "$API_URL/api/chat/v2" > "$sse_file" || true

  {
    echo "### Task $index: $family"
    echo
    echo "- prompt_file: \`$prompt_file\`"
    echo "- sse_capture: \`$sse_file\`"
    echo "- session_id: $(grep -m1 '^data: {\"session_id\"' "$sse_file" | sed 's/^data: //' | jq -r '.session_id // "unknown"' 2>/dev/null || echo unknown)"
    echo "- task_id: $(grep -m1 '\"task_id\"' "$sse_file" | sed 's/^data: //' | jq -r '.task_id // "unknown"' 2>/dev/null || echo unknown)"
    echo "- terminal_done: $(grep -c '^event: done' "$sse_file" || true)"
    echo "- observer_events: $(grep -c '^event: skill_self_evolution_observer' "$sse_file" || true)"
    echo "- proposal_ids:"
    grep '\"proposal_id\"' "$sse_file" | sed 's/^data: //' \
      | jq -r 'select(.proposal_id? != null) | "  - `" + .proposal_id + "`"' 2>/dev/null \
      | sort -u || true
    echo
  } >> "$EVIDENCE_DIR/task-summary.md"
}

run_materialization_operator() {
  local evidence_json="$EVIDENCE_DIR/evidence-refs.json"
  jq -n \
    --arg report "$REPORT_PATH" \
    --arg day "$DAY_LABEL" \
    --arg root "$EVIDENCE_DIR" \
    '{evidence_ids: [$report, $root, ("monitor-day://" + $day)]}' > "$evidence_json"

  curl -sS -X POST \
    -H 'Content-Type: application/json' \
    -d "$(jq -n \
      --arg reason "daily self-evolution monitor materialization sweep" \
      --argjson evidence "$(jq '.evidence_ids' "$evidence_json")" \
      '{
        dry_run:false,
        batch_limit:6,
        min_ready_score:0,
        entitlement_ready:true,
        package_ready:true,
        evidence_ids:$evidence,
        approval_refs:["daily-self-evolution-monitor"],
        policy_decision_refs:["operator-approved-daily-self-evolution-monitor"],
        reason:$reason
      }')" \
    "$API_URL/api/apps/$APP_ID/skills/operations/materialization/operator/run" \
    > "$EVIDENCE_DIR/materialization-operator.json" || true
}

run_curation() {
  local stale_days="$1"
  local label="$2"
  curl -sS -X POST \
    -H 'Content-Type: application/json' \
    -d "$(jq -n \
      --argjson stale "$stale_days" \
      --arg reason "daily self-evolution monitor curation $label" \
      '{
        stale_after_days:$stale,
        narrow_use_threshold:1,
        entitlement_ready:true,
        package_ready:true,
        approval_refs:["daily-self-evolution-monitor"],
        policy_decision_refs:["operator-approved-daily-self-evolution-monitor"],
        reason:$reason
      }')" \
    "$API_URL/api/apps/$APP_ID/skills/operations/curation/run" \
    > "$EVIDENCE_DIR/curation-$label.json" || true
}

write_report() {
  local pre_summary post_summary materialization_summary default_curation forced_curation
  pre_summary="$(summarize_operations "$EVIDENCE_DIR/operations-before.json")"
  post_summary="$(summarize_operations "$EVIDENCE_DIR/operations-after.json")"
  materialization_summary="$(jq '{run_id:.result.run_id,status:.result.status,mutated:.result.mutated,selected_proposals:.result.selected_proposal_ids,materialized_skill_ids:(.result.materializations // [] | map(.skill_id)),rollback_memento_refs:(.result.materializations // [] | map(.rollback_memento_ref))}' "$EVIDENCE_DIR/materialization-operator.json" 2>/dev/null || echo '{}')"
  default_curation="$(jq '{candidate_count:.result.run.candidate_count,actions:.result.run.actions,semantic_status:.result.semantic_analysis_status,report_ref:.result.report_ref}' "$EVIDENCE_DIR/curation-default.json" 2>/dev/null || echo '{}')"
  forced_curation="$(jq '{candidate_count:.result.run.candidate_count,actions:.result.run.actions,semantic_status:.result.semantic_analysis_status,report_ref:.result.report_ref}' "$EVIDENCE_DIR/curation-forced.json" 2>/dev/null || echo '{}')"

  cat > "$REPORT_PATH" <<REPORT
# Self-Evolution Daily Monitor - $RUN_DATE ($DAY_LABEL)

## Run Identity

- app_id: \`$APP_ID\`
- agent: \`$AGENT_NAME\`
- api: \`$API_URL\`
- run_stamp_utc: \`$RUN_STAMP\`
- tasks_per_day: $TASKS_PER_DAY
- evidence_dir: \`$EVIDENCE_DIR\`

## Pre-Run Snapshot

\`\`\`json
$pre_summary
\`\`\`

## Task Batch

$(cat "$EVIDENCE_DIR/task-summary.md" 2>/dev/null || echo "No task summary captured.")

## Materialization Evidence

\`\`\`json
$materialization_summary
\`\`\`

## Curation Evidence

### Default Threshold Track

\`\`\`json
$default_curation
\`\`\`

### Forced Diagnostic Threshold Track

\`\`\`json
$forced_curation
\`\`\`

The forced diagnostic track proves candidate detection only. It is not counted
as production archive proof unless a policy decision explicitly authorizes the
shortened stale threshold for a test namespace.

## Post-Run Snapshot

\`\`\`json
$post_summary
\`\`\`

## Daily Verdict

- precipitation: inconclusive until observer and proposal ids are reviewed
- materialization: inconclusive until operator status and rollback refs are reviewed
- reuse_or_optimization: inconclusive until telemetry deltas are compared
- merge_readiness: inconclusive until duplicate groups and semantic status are reviewed
- archive_readiness: inconclusive for production unless default-threshold curation recommends archive
- blocking_gap: fill after reviewing evidence refs in \`$EVIDENCE_DIR\`
REPORT
}

echo "Capturing pre-run snapshot..."
capture_snapshot "before"

families=(
  precipitation_seed
  reuse_followup
  optimization_probe
  duplicate_merge_probe
  archive_readiness_probe
  audit_summary
)

: > "$EVIDENCE_DIR/task-summary.md"
for ((i = 0; i < TASKS_PER_DAY; i++)); do
  family="${families[$i]}"
  echo "Running task $((i + 1))/$TASKS_PER_DAY: $family"
  run_task "$((i + 1))" "$family"
done

echo "Waiting $WAIT_SECONDS seconds for observer and telemetry flush..."
sleep "$WAIT_SECONDS"

echo "Running materialization operator..."
run_materialization_operator

echo "Running curation default threshold..."
run_curation "$DEFAULT_STALE_DAYS" "default"

echo "Running curation forced diagnostic threshold..."
run_curation "$FORCED_STALE_DAYS" "forced"

echo "Capturing post-run snapshot..."
capture_snapshot "after"

echo "Writing report..."
write_report

echo "Daily monitor complete:"
echo "  report: $REPORT_PATH"
echo "  evidence: $EVIDENCE_DIR"
