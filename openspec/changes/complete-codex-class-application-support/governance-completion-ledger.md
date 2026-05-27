# Governance Completion Ledger

This ledger is the phase-0 control surface for
`complete-codex-class-application-support`. It maps every capability gap from
`docs/macaca-codex-application-capability-gap-research.md` to an owner and to
the checklist item that must close the gap. A later phase cannot claim
completion when the mapped item only exposes a catalog entry, descriptor, or
synthetic healthy state.

## Scope Rules

- Application packages own product workflows, prompts, UI, keyboard behavior,
  and domain-specific orchestration.
- Macaca OS owns only provider-neutral services, service descriptors, policy,
  resource control, trace, audit, artifacts, diagnostics, and SDK facades.
- Runtime-host composition roots may construct providers through Abstract
  Factory seams; SDK, Web, CLI, and frontend surfaces may not construct
  providers.
- Optional providers must report structured unavailable diagnostics. They must
  not crash, hang, silently fall back, or report fake success.
- Completion evidence must include policy-before-side-effect behavior,
  sanitized trace/audit records, and real provider-backed execution whenever the
  task requires a provider.

## Capability Gap Map

| Research gap | Owner boundary | Checklist target | Completion guard |
| --- | --- | --- | --- |
| Thread/Turn/Item lifecycle | `service.interaction` | 2.1-2.10 | Durable replay, fork/archive/rollback, turn steering, item stream, and audit; descriptor-only state fails. |
| Bidirectional app protocol | `service.app_protocol` shell/gateway adapter over focused clients | 3.1-3.10 | Gateway owns transport only; interaction/file/process/plugin semantics must remain in services. |
| Dynamic/deferred tool lifecycle parity | `service.tool` plus owning provider services | 1.3, 4.7, 5.8, 12.5, 13.4, 16.1-16.6 | Tool descriptors must route to provider-backed owners or truthful unavailable providers. |
| Filesystem RPC and watchers | `service.file` | 4.1-4.8 | Real read/write/patch/list/watch provider, path policy, artifacts, and audit; file-family catalog alone fails. |
| Command exec and PTY | `service.process` with sandbox preflight | 5.1-5.9 | Exec/spawn/stdin/resize/terminate/output streaming must run through policy and sandbox gates. |
| Sandboxing and permission profiles | `service.sandbox` | 6.1-6.7 | Profile resolution, environment lifecycle, optional provider seams, and cleanup evidence are required. |
| Approvals and reviewer flow | `service.approval` with shell rendering only | 7.1-7.7 | Requests must persist, resolve, expire, emit events, and audit decisions before privileged side effects. |
| Hooks | `service.hook` decorator chain | 8.1-8.8 | Managed-only policy, pre/post ordering, bounded mutations, provider seams, and audit are required. |
| MCP manager lifecycle | upgraded `service.mcp` | 12.1-12.6 | Status, reload, resources, OAuth, diagnostics, and `service.tool` invocation routing must be service-owned. |
| Skills lifecycle | upgraded `service.skill` | 13.1-13.5 | Read/config/watch/enablement/provenance must be app/workspace scoped and policy gated. |
| Plugin marketplace | `service.plugin_marketplace`, store, entitlement, policy | 11.1-11.7 | Install/upgrade/uninstall/auth and bundled capability registration must pass admission and audit. |
| Config and requirements | `service.config` | 9.1-9.8 | Layered config, requirements, permission profiles, feature flags, hot reload, and redaction are required. |
| Model catalog and LLM hardening | upgraded `service.llm` | 10.1-10.6 | Provider capability reads, continuation validation, route diagnostics, and degradation states are required. |
| Memory mode and recall audit | existing `service.memory` and context services | 1.4, 16.4, 19.12 | Per-thread context delivery must remain traceable and bounded; storage-only proof is insufficient. |
| Git/diff/patch/review | `service.git` and `service.review` | 14.3-14.8 | Patch provenance, rollback markers, structured findings, and audit replay are required. |
| Code intelligence | `service.code_intelligence` | 14.1-14.2 | Search and symbol context must use provider adapters and structured unavailable behavior. |
| Realtime text/audio | optional `service.realtime` | 15.4 | Absence must be structured unavailable; presence must not change base OS semantics. |
| Remote environments | optional `service.remote_environment` | 15.5-15.6 | Health, workspace roots, cleanup, and remote selection must be traceable optional-module behavior. |
| Feedback and diagnostics | `service.diagnostics` | 15.1-15.3 | Bundles and health summaries must be privacy-filtered, bounded, and replayable. |
| Secrets/keyring and scoped injection | service-owned config/secret policy boundary | 9.2, 9.3, 11.2, 15.3 | Raw secrets must never enter snapshots, logs, traces, or diagnostics. |
| Application manifest capability declaration | application framework contract | 17.1-17.6 | Capability access must come from manifest declarations and policy, not OS application-name branches. |
| Thin Web/CLI/frontend/IDE surfaces | shells and app protocol adapters | 18.1-18.6 | Shells may parse/render/subscribe only; ownership tests must reject semantic drift. |
| End-to-end parity proof | application-neutral fixture application | 19.1-19.14 | The proof must run real multi-service workflow with no application-specific OS code. |

## GitNexus Notes

- `forbidden_tokens` impact: LOW; direct caller is
  `serviceization_escape_hatches_reject_new_production_references`.
- `is_approved_migration_surface` impact: LOW; direct caller is `scan_file`,
  then the serviceization escape-hatch integration test.
- Adding the Codex-class scope-control integration test creates new test symbols
  not present in the current GitNexus index, so pre-edit symbol impact is
  recorded as index-missing advisory rather than a blocker.
