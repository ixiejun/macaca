## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study VS Code Terminal and Pseudoterminal APIs for terminal lifecycle, PTY-like IO, dimensions, close events, shell integration, and extension boundary behavior.
- [x] 1.3 Study Node.js `child_process` for spawn/exec/execFile/fork, stdio streams, cwd, env, shell mode, signals, exit events, timeout, and abort behavior.
- [x] 1.4 Study Python `subprocess` for `Popen`, argument vectors, stdin/stdout/stderr pipes, environment, cwd, return codes, timeout, terminate, and kill behavior.
- [x] 1.5 Study Docker Engine Exec API for exec create, start/attach streaming, resize, inspect, and container-scoped provider capability behavior.
- [x] 1.6 Produce a supplier capability comparison memo mapping VS Code, Node.js, Python, and Docker concepts into Macaca provider-neutral terminal DTOs and commands.
- [x] 1.7 Define explicit non-goals for concrete host shell, PTY, SSH, Docker, IDE, remote execution, platform syscall providers, application workflows, raw provider pass-through, and provider-specific routing.
- [x] 1.8 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.developer.terminal.v1` descriptor metadata: pack id, family, lifecycle, stability, provider support, spawn support, shell support, PTY support, stdin support, stream support, resize support, signal support, snapshot support, env support, cwd support, network modes, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `TerminalScope`, `TerminalProviderCapability`, `TerminalProcessSpec`, `TerminalEnvironmentPolicy`, `TerminalWorkdirScope`, `TerminalPtyProfile`, `TerminalSpawnPlan`, `TerminalSession`, `TerminalStreamCursor`, `TerminalOutputChunk`, `TerminalStdinFrame`, `TerminalSignalIntent`, `TerminalExitStatus`, `TerminalResourceUsage`, and `TerminalSnapshotHandle`.
- [x] 2.3 Define typed command/result DTOs for `terminal.inspect_provider`, `terminal.plan_spawn`, `terminal.spawn_request`, `terminal.stream_output`, `terminal.send_stdin`, `terminal.resize`, `terminal.inspect_process`, `terminal.collect_exit`, `terminal.cancel`, `terminal.snapshot_workdir`, and `terminal.cleanup_session`.
- [x] 2.4 Define typed success, streaming, paged, partial, denied, unavailable, unsupported, conflict, not-running, stale-handle, invalid-command, invalid-workdir, invalid-env, stream-truncated, quota, timeout, cancellation, approval-required, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, process spec hashing, spawn plan hashing, environment policy hashing, workdir scope hashing, stream cursor hashing, output chunk hashing, exit status hashing, snapshot handle hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, process specs, env policies, workdir scopes, PTY profiles, stream cursors, output chunks, stdin frames, signals, exit statuses, snapshots, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.developer.terminal.v1` declarations.
- [x] 3.2 Implement permission validation for `terminal.provider.inspect`, `terminal.spawn`, `terminal.stream.read`, `terminal.stdin.write`, `terminal.resize`, `terminal.process.inspect`, `terminal.exit.collect`, `terminal.cancel`, `terminal.workdir.snapshot`, and `terminal.session.cleanup`.
- [ ] 3.3 Implement provider/workspace/process/session scope checks for declared workspaces, cwd handles, environment handles, stream handles, process handles, snapshot handles, denied scopes, and stale handles.
- [ ] 3.4 Implement policy checks for command allowlist strategy, argument-vector validation, shell-mode policy, cwd/workspace policy, environment policy, filesystem policy, network policy, stdio policy, PTY profile, timeout, cancellation strategy, stream redaction, snapshot retention, and output bounds.
- [ ] 3.5 Implement resource reservation for process count, duration, CPU class, memory class, disk bytes, network bytes, stdout/stderr bytes, stdin bytes, stream retention, snapshot size, timeout, provider quota, and retained replay metadata.
- [ ] 3.6 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing host capability, missing workspace permission, missing credential reference, unsupported PTY, unsupported stdin, unsupported resize, unsupported signals, disabled network, missing entitlement, provider quota, and host resource denial.
- [ ] 3.7 Implement approval behavior for sensitive env keys, secret-reference use, writes outside declared workspace scope, network access, privilege escalation, long-running processes, destructive commands, external side effects, terminal snapshots, and cancellation escalation from graceful terminate to force kill.
- [ ] 3.8 Add tests proving denied, validation, quota, unavailable, conflict, stale-handle, invalid-command, invalid-workdir, invalid-env, unsupported, timeout, cancellation, and approval-required paths do not call concrete providers, spawn processes, send stdin, resize terminals, terminate processes, read file content, contact networks, or expose raw output.

## 4. Service Provider And Runtime Integration

- [ ] 4.1 Implement or bind the terminal/process service provider behind the service runtime; do not construct terminal providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [ ] 4.3 Add mock provider support for provider inspection, spawn planning/request, stream output, stdin, resize, process inspection, exit collection, cancellation, workdir snapshots, cleanup, health, and provider capability inspection.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded streaming, stream retention, output truncation, dropped-output counters, stale-handle diagnostics, and rate-limit diagnostics.
- [ ] 4.5 Add Strategy implementations for provider adapters, command validators, shell-mode policy, workdir policy, environment policy, stream redaction, cancellation strategy, snapshot strategy, provider capability reporting, and unavailable behavior.
- [ ] 4.6 Add side-effect safety support for idempotency keys, provider state validation, process handle freshness, resource reservation, approval state, stream cursor validation, cancellation escalation, cleanup, and non-mutating plan commands.
- [ ] 4.7 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, host-limited, PTY-limited, stream-limited, stdin-limited, resize-limited, signal-limited, snapshot-limited, network-limited, workspace-limited, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.developer.terminal.v1` with command schemas, provider support, PTY support, stream support, stdin support, resize support, signal support, snapshot support, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `terminal.*` commands; helpers must only build canonical traced service calls and must never construct process clients, access credentials, call host process APIs, spawn commands, send stdin, terminate processes, read raw streams, or bypass policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover terminal commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for inspecting provider capability, planning spawn, requesting spawn, streaming output, sending stdin, resizing PTY, inspecting process state, collecting exit status, cancelling, creating snapshot handles, and cleaning up sessions.
- [x] 5.6 Add unavailable-provider, missing-workspace-permission, invalid-command, invalid-env, invalid-workdir, stream-truncated, stdin-denied, resize-unsupported, cancellation-approval, provider-quota, timeout, network-denied, and snapshot-denied examples that demonstrate diagnostics without provider names, credentials, private env values, private file content, raw output, or workflow-specific conventions.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, provider-inspection, spawn-plan, spawn-request, stream-output, stdin-send, resize, process-inspection, exit-collection, cancellation, workdir-snapshot, cleanup, policy, entitlement, resource, approval, health, snapshot, unavailable, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, env values, secret material, private file content, raw terminal output, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded streams.
- [ ] 6.3 Add replay tests proving every `terminal.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [ ] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete shell, PTY, SSH, Docker, IDE terminal, platform process, credential-manager, filesystem-provider, network-provider, or remote execution adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [ ] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, spawns processes, sends stdin, terminates processes, snapshots workdirs, contacts networks, reads raw file content, emits raw output, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-developer-terminal --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/developer/terminal.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, provider scopes, workspace scopes, process specs, shell mode, argument vectors, cwd, env, stdio, PTY profiles, streams, stdin, resize, cancellation, exit status, resource usage, snapshots, cleanup, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, redaction behavior, streaming/pagination behavior, timeout/cancellation behavior, plan/request behavior, approval behavior, snapshot retention behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: VS Code Terminal/Pseudoterminal, Node.js `child_process`, Python `subprocess`, and Docker Engine Exec concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for provider inspection, spawn planning/request, output streaming, stdin, resize, process inspection, exit collection, cancellation, snapshot handles, cleanup, and unavailable diagnostics using synthetic process data only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, command allowlist validation, workdir/env policy, PTY support, stream redaction, cancellation behavior, exit diagnostics, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-developer-terminal` complete.
