# Change: Add Developer Terminal Pack

## Why

Developers need `pack.developer.terminal.v1` as an industrial command execution
capability for spawning bounded processes, interacting with stdin/stdout/stderr,
allocating PTY sessions when supported, streaming output, resizing terminal
sessions, cancelling processes, collecting exit diagnostics, inspecting process
state, and capturing sanitized working-directory snapshots. It must not be a
thin wrapper around one shell, IDE terminal, container runtime, or host platform.

Terminal execution can read files, write files, contact networks, consume host
resources, leak secrets, modify repositories, trigger external side effects, or
run indefinitely. Macaca must therefore expose terminal behavior only through
provider-neutral typed service commands with permission, policy, entitlement,
resource, approval, trace, audit, cancellation, health, snapshot, replay, and
structured unavailable behavior.

## Research And Supplier/API Baseline

Official references considered for this pack:

- VS Code Extension API terminal and pseudoterminal models expose terminal
  creation, process-like IO, terminal dimensions, close events, and shell
  integration concepts. Reference: https://code.visualstudio.com/api/references/vscode-api
- Node.js `child_process` exposes asynchronous spawning, stdio streams, exit
  events, signals, shell/non-shell execution, environment, working directory,
  timeout, and abort behavior. Reference: https://nodejs.org/api/child_process.html
- Python `subprocess` exposes `Popen`, arguments, stdin/stdout/stderr pipes,
  environment, working directory, return codes, timeout, and termination.
  Reference: https://docs.python.org/3/library/subprocess.html
- Docker Engine Exec API separates exec creation, start/attach streaming,
  resize, and inspect for container-scoped command execution. Reference:
  https://docs.docker.com/reference/api/engine/version/v1.43/

Macaca maps these mature API families into terminal session, process spec,
environment, working directory, PTY profile, stream cursor, stdin frame, signal
intent, exit status, resource usage, output redaction, and provider capability
DTOs. Concrete host shells, remote terminals, container exec providers,
PTY libraries, and platform-specific process APIs remain behind replaceable
providers.

## What Changes

- Add provider-neutral `pack.developer.terminal.v1` under the `developer`
  family.
- Define command namespace `terminal.*` for:
  - provider and host capability inspection
  - process/session spawn planning
  - process/session spawn requests
  - output stream subscription and chunk retrieval
  - stdin frame submission
  - terminal resize
  - process inspection
  - exit collection
  - cancellation/termination
  - sanitized working-directory snapshot handles
  - session cleanup and replay diagnostics
- Define DTOs for terminal scope, provider capability, process spec, environment
  policy, working-directory scope, PTY profile, stream cursor, output chunk,
  stdin frame, signal intent, process state, exit status, resource usage,
  snapshot handle, spawn plan, and diagnostics.
- Define permission scopes, policy defaults, command allowlist strategy,
  workspace and filesystem boundaries, network/resource limits, approval rules,
  SDK discovery, developer documentation, trace/audit events, snapshots, replay,
  and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/developer/terminal.md` before implementation completion.

## Impact

- Affected specs: `pack-developer-terminal`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, terminal/process
  service provider or unavailable provider, runtime-host provider adapters,
  stream/cancellation/snapshot support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete host shell, PTY, Docker, SSH, IDE, platform process, or
  remote execution provider implementation in this proposal; no
  application-specific build/test/deploy workflow; no provider-name or shell-name
  routing in OS layers; no raw secrets, environment values, private file content,
  prompts, manifests, package bytes, raw terminal output, or unbounded streams in
  observability; no SDK/shell/kernel provider construction; no fake success when
  provider, command support, workspace scope, entitlement, permission, approval,
  resource, or host support is absent.
