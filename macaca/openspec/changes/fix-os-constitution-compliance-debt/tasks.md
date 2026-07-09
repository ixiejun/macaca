# Tasks: Remediate OS Constitution Compliance Debt

> Execution rules for every task below:
> - All new Rust code carries detailed English comments explaining function and
>   operating principle; key execution nodes emit `tracing` logs; cross-boundary
>   operations are typed Command/Result and carry trace context.
> - No application-specific logic and no hardcoded application/provider/model
>   names below the application layer. Apply a design pattern deliberately
>   (Decorator/Strategy/Specification/State/Factory/Adapter) and record which.
> - After each task: `cargo check --workspace` + relevant unit tests; after each
>   stage: full gate suite (`cargo test -p macaca-integration-tests`).
> - GitNexus impact analysis is RUN and its findings RECORDED in the task notes,
>   but per the requester its CRITICAL/HIGH warnings are non-blocking for this
>   change (memo only).

## 0. Unblock CI (prerequisite)
- [x] 0.1 Split `ai_common.rs` — already resolved in-tree (file is now 243 lines
  with a `define_ai_command_wrappers` macro); no action required
- [x] 0.2 `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`
  confirmed green (3 passed)
- [x] 0.3 GitNexus memo recorded in `gitnexus-memo.md` (warnings non-blocking)

## 1. P0 — Safety & Crash Fixes
- [x] 1.1 Added foundation `macaca-proto::text_sanitize` module (`safe_char_prefix`,
  `truncate_with_marker`, `split_by_chars`, `mask_secret`, `is_secret_shaped`)
  with detailed English docs, key-node tracing on redaction, and multibyte tests
- [x] 1.2 Replaced byte-slice truncation with the safe primitives in
  `macaca-kernel/logging.rs` (mask_sensitive/truncate/mask_json_params),
  `macaca-task/decompose.rs` (parse error path),
  `macaca-gateway/telegram_format.rs` (split_message); added Chinese/emoji tests
- [x] 1.3 `macaca-kernel/logging.rs` secret masking now fully redacts secret-shaped
  values (no sk-/Bearer prefix retained) via structural `mask_secret`
- [ ] 1.4 Fix `macaca-skill/runtime/path_policy.rs:11-26`: canonicalize failure →
  reject; add `..` traversal tests
- [ ] 1.5 Make skill/proposal readiness fail-closed:
  `evolution.rs:48-52,83-86,137-139` defaults → Missing/false;
  `proposal_lifecycle.rs:175-177`, `proposal_processing.rs:183-187`,
  `curation.rs:82-104` require `Some(true)`; `autonomy live_orchestrator.rs:305-337`
  add real lease validation
- [ ] 1.6 P0 execution stop-gap: add path allow-list, output byte cap, timeout
  `child.kill()`, and command-line sanitization to `macaca-tools/builtin.rs`
  ShellTool/FileRead/FileWrite and `macaca-skill/tool.rs` execute paths; require
  trace
- [ ] 1.7 Rewrite `macaca-autonomy-evolution` `sanitize_ref` to structural
  allow-list; unify sanitization at the contract boundary for both JSONL and
  in-memory ledger paths
- [ ] 1.8 State fixes: heartbeat coalesce guard against Running
  (`command_handler.rs:53-87`); scheduler `scheduled_for <= now` filter
  (`run_control.rs:353-376`); zero-pad run/job ids (`store.rs:65-69`)

## 2. Observability Sanitization & Gate Hardening & Structured Absence
- [ ] 2.1 Adopt `redact_text` + bounding in `macaca-llm` provider adapters
  (anthropic/dashscope/openai/openai_compatible) and `macaca-memory`
  embedding.rs/vector.rs error paths
- [ ] 2.2 Bound/sanitize gateway logs (`telegram.rs:114`, `gateway.rs:79-124`) and
  tools trace input/output (`macaca-tools/tool.rs:184-225`, `builtin.rs:135`)
- [ ] 2.3 Harden `sdk_no_provider_construction_gate` to naming-pattern + mandatory
  registration; add anti-`concat!`-splitting rule to the no-hardcoded-names gate;
  add a `use`-level boundary scan
- [ ] 2.4 Structured absence (`provider-absence-contract`): gateway telegram/discord
  (`telegram.rs:67-77,169-179`, `discord.rs:44-65`), tools ListAgents
  (`orchestration.rs:303-310`), driver loader (`loader.rs:141-144`), skill facade
  (`facade.rs:139-148`)
- [ ] 2.5 Scheduled-agent-task rollback on registration failure
  (`local_provider.rs:161-199`); goal-evaluator no fake `Satisfied` on parse
  failure (`goal_evaluator.rs:66-108`)
- [ ] 2.6 Event-log truthful durability: `event_log.rs:178-203,130-132` propagate
  write errors, fix sequence gap

## 3. Side-Effect Guard & Trace Closure (OpenSpec-tracked; run GitNexus memo)
- [ ] 3.1 Implement `SideEffectGuard` decorator + shared `Readiness` type in
  runtime-host (Decorator + Strategy), with trace→policy→entitlement→budget→
  resource→execute→audit order and full English docs + logs
- [ ] 3.2 Route Task write commands
  (`task_lifecycle_commands.rs`/`assignment_commands.rs`/`goal_commands.rs`) and
  skill/tool execution through the guard; enforce declared `task.manage` etc.
- [ ] 3.3 Gateway inbound/outbound: add trace propagation and pre-send policy/budget
  checks (`telegram.rs:89-160`)
- [ ] 3.4 Scheduler scope enforcement on `trigger_job`/`get_run`
  (`service.rs:209-274,315-347`); context policy passthrough in `into_engine_input`
  (`service_contract.rs:131-142`); skill command trace mandatory
  (`service_contract.rs:167-185`)
- [ ] 3.5 LLM factory + budget: replace `router.rs:125-130` name-match construction
  with a ProviderFactory registry constructed in the host composition root; inject
  `max_budget_usd` from config (`router.rs:140-152`); confirm/close the
  `impl LlmProvider for LlmRouter` direct-call bypass (Open Question OQ1)
- [ ] 3.6 Move LLM pricing table (`cost.rs:20-59`) and minimax URL branch
  (`coding_plans.rs:12-24`) to descriptor/config data; move DashScope formatter
  (`macaca-framework/formatter.rs:357-400`) to the LLM provider adapter layer
- [ ] 3.7 `service_router` idempotency-aware retry (`service_router.rs:177-220`)
  reading the descriptor idempotent flag (protects payment/evm)

## 4. State-Machine & Concurrency Correctness
- [ ] 4.1 Implement generic `TransitionMatrix<S>` (Specification pattern) and adopt
  in task/scheduler/heartbeat/autonomy transitions; illegal → structured conflict
- [ ] 4.2 Autonomy live-tick TOCTOU single-lock (`local_provider.rs:235-289`); task
  claim CAS (`task_board.rs:66-142,153-193`); goal crash recovery
  (`todo_store.rs:203-212`); worker wait timeout + send-failure rollback
  (`worker_loop.rs:122-138,184-195`); queue result write-back
  (`queue.rs:312,335`); graph-admission self-equality fix
  (`graph_admission.rs:70-76`)
- [ ] 4.3 Cron standard OR semantics + step/range/list (`schedule.rs:196-239`);
  stagger anchor drift fix (`materialization.rs:48-52`); telegram getUpdates
  backoff (`telegram.rs:111-117`)
- [ ] 4.4 Driver nested-runtime fix (spawn_blocking for health/shutdown,
  `sdk.rs`/`dynamic_proxy.rs:152-164`); streaming callback lifetime guard
  (`dynamic_proxy.rs:134-135`)
- [ ] 4.5 Context truncation pairs assistant+tool messages / falls back to
  System/User boundary (`context_window.rs:105-123`, `iteration.rs:132-138`); add
  post-trim reconvergence (`context_window.rs:81-83`)
- [ ] 4.6 Runtime-host six-lock deadlock fix: clone-then-drop before construction
  (`skill_service_provider_merge.rs:174-200`)

## 5. Boundary Extraction & Refactors (each item OpenSpec-tracked; GitNexus memo)
- [ ] 5.1 Create `crates/foundation/macaca-domain-pack-contracts`; move
  `domain_pack_contract/*` out of `macaca-proto` (re-export for deprecation
  window); move approval/bounds/reports semantics to packs/services; replace the
  `(domain, slug)` match with a self-registration registry
- [ ] 5.2 SDK generic preflight builder skeleton + table-driven client tests
  (collapse the 12 `domain_pack_client_*_tests.rs`)
- [ ] 5.3 Foundation purity: neutralize `proto/config/root.rs:71-109`; feature-gate
  `macaca-ipc/web3_bridge` and `async-nats`; move `macaca-persist/payment_store.rs`
  to the payment service; fix `execution_port.rs:52-55` read-lock-across-await;
  move `macaca-kernel/alert.rs:123-160` convenience methods to service layer
- [ ] 5.4 Shell semantic re-homing: four Task/Autonomy commands
  (`build_task_execution_prompt`/`retry_task`/`build_followup_planning_prompt`/
  `cancel_partial_goal_tasks`) replacing `macaca-web/loop_manager/*` prompt/retry/
  replan/repair; goal-eval explicit outcome
- [ ] 5.5 CLI: replace three reqwest clients with SDK clients; remove `reqwest`
  from `macaca-cli/Cargo.toml`; drop hardcoded `127.0.0.1:3001`
- [ ] 5.6 Remaining hardcode/semantic leakage: skill provisioner/source/discovery
  client lists → config/persona injection; `macaca-tools/todo/create_todo.rs`
  decomposition keywords → injected `DependencyInferenceStrategy` from macaca-task;
  autonomy admission skill-specifics → replaceable provider

## 6. Resilience Hygiene & Lifecycle Completeness (continuous)
- [ ] 6.1 Lock-poison sweep: `lock().expect("poisoned")` → `into_inner()` recovery
  or structured failure across services (esp. `execution_control_runtime.rs`,
  autonomy ledger, scheduled-agent-task, heartbeat memento, task snapshot, llm
  cost); add gate against reintroduction
- [ ] 6.2 Bounded growth: retention/TTL/eviction on snapshot maps, terminal-run
  history, payload stores, audit/diagnostic buffers; evaluate bounded channels for
  the 7 unbounded ones
- [ ] 6.3 Service-contract completeness: tools ServiceContract + typed DTOs +
  descriptor; gateway lifecycle/CancellationToken + real health; autonomy/skill
  start/pause/resume/shutdown; scheduled-agent-task health probes scheduler
- [ ] 6.4 Near-limit file splits (evm_service_provider.rs first) and dead-code
  cleanup (`loop_detector.rs` window fields, skill discovery MAX_DEPTH,
  `task/tracker.rs` after confirmation)

## 7. Validation & Documentation
- [ ] 7.1 Add executable gates/tests: side-effect guard order, fail-closed
  readiness, UTF-8-safe truncation, structural redaction, provider-absence
  structured states, idempotent retry, transition matrix
- [ ] 7.2 Regression: `/api/chat/v2` session create/recover, session-scoped task
  boards, trace/audit replay after refresh
- [ ] 7.3 Update the three constitution documents and the audit report status
  entries; append the deferred GitNexus warning memo
- [ ] 7.4 `openspec validate fix-os-constitution-compliance-debt --strict` green;
  record answers to Open Questions OQ1–OQ4
