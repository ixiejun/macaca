# Developer Terminal Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.developer.terminal.v1`. Terminal support must expose process/session
lifecycle, PTY-like IO, streams, cwd/env, dimensions, resize, signals, timeout,
abort, exit status, container exec, and diagnostics through typed service
commands. It must not expose host shell, raw PTY, SSH, Docker, IDE, remote
execution, platform syscall, or application workflow semantics directly.

## Source Baseline

- VS Code Terminal/Pseudoterminal API and shell integration:
  <https://code.visualstudio.com/api/references/vscode-api>
  and <https://code.visualstudio.com/docs/terminal/shell-integration>
- Node.js `child_process`:
  <https://nodejs.org/api/child_process.html>
- Python `subprocess`:
  <https://docs.python.org/3/library/subprocess.html>
- Docker Engine Exec API:
  <https://docs.docker.com/reference/api/engine/version/v1.43/>

## Supplier API Notes

- VS Code Terminal/Pseudoterminal contributes terminal lifecycle, PTY-like IO,
  dimensions, close events, shell integration, command detection, working
  directory detection, and extension boundary behavior. Macaca should model a
  terminal session and shell integration metadata without relying on VS Code.
- Node.js `child_process` contributes spawn/exec/execFile/fork, stdio streams,
  cwd, env, shell mode, signals, exit events, timeout, and AbortSignal-like
  cancellation. Macaca should prefer argument-vector commands and treat shell
  mode as a separately approved capability.
- Python `subprocess` contributes `Popen`, pipes, environment, cwd, return
  codes, timeout, terminate, kill, and wait/communicate behavior. Macaca should
  model process lifecycle, timeout, and signal semantics provider-neutrally.
- Docker Engine Exec contributes exec create, start/attach streaming, resize,
  inspect, container-scoped IO, TTY mode, and container capability boundaries.
  Macaca should model container exec as an optional provider capability, not a
  terminal invariant.

## Macaca-Owned Abstractions

`pack.developer.terminal.v1` should define `TerminalSession`,
`TerminalCommand`, `TerminalProcess`, `TerminalStream`,
`TerminalEnvironment`, `TerminalWorkingDirectory`, `TerminalDimensions`,
`TerminalSignal`, `TerminalExitStatus`, `TerminalTimeoutPolicy`,
`TerminalExecTarget`, `TerminalOutputChunk`, `TerminalTranscript`, and
`TerminalProviderCapability`.

The DTOs must carry target identity, command argv, shell-mode policy, cwd/env
redaction, stream framing, PTY/TTY mode, dimensions, resize, signal/abort
state, timeout and resource budgets, exit status, bounded transcript handles,
provider capability hashes, and replay pointers. Raw shell strings, secrets,
credentials, unbounded stdout/stderr, private env vars, host paths beyond policy,
and provider-native exec payloads are rejected.

## Explicit Non-Goals

- Do not implement concrete host shell, PTY, SSH, Docker, IDE,
  platform-syscall, remote execution, process supervisor, or application
  workflow providers in this research phase.
- Do not define build, deployment, repository, CI, scraping, or
  application-specific terminal workflows in OS layers.
- Do not expose raw shell commands, provider process ids, host paths,
  environment variables, or provider-specific routing as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, repository/CI/code adjacency, file handles, and secrets-reference
  handles provide reusable substrate.
- Current evidence does not prove terminal DTOs, providers, SDK helpers, WASM
  ABI, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
