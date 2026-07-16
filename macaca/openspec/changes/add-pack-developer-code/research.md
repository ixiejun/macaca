# Developer Code Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.developer.code.v1`. Code intelligence support must expose documents,
workspace symbols, diagnostics, references, semantic tokens, actions, commands,
formatting, edits, parse trees, scans, and SARIF-like results through typed
service commands. It must not hardcode editor integrations, parser engines,
model clients, repository workflows, or application-specific coding flows.

## Source Baseline

- Language Server Protocol 3.17:
  <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/>
- VS Code Extension API and programmatic language features:
  <https://code.visualstudio.com/api/references/vscode-api>
  and
  <https://code.visualstudio.com/api/language-extensions/programmatic-language-features>
- Tree-sitter:
  <https://tree-sitter.github.io/>
  and <https://github.com/tree-sitter/tree-sitter>
- GitHub CodeQL/code scanning and SARIF support:
  <https://docs.github.com/en/rest/code-scanning/code-scanning>
  and
  <https://docs.github.com/en/code-security/reference/code-scanning/sarif-files/sarif-support>

## Supplier API Notes

- LSP 3.17 contributes diagnostics, document/workspace symbols, references,
  semantic tokens, code actions, commands, formatting, workspace edits, and
  request/notification capability negotiation. Macaca should model document
  handles, language features, edit plans, and server capability metadata.
- VS Code Extension API contributes workspace/document handles, diagnostics,
  code actions, commands, tasks, authentication, language features, and
  workspace edits. Macaca should use it as an integration pattern, not as OS
  semantics.
- Tree-sitter contributes incremental parsing, concrete syntax trees, changed
  ranges, grammar support, parser lifecycle, and error recovery. Macaca should
  model parse trees and changed ranges without exposing grammar-specific node
  names as generic OS behavior.
- CodeQL/code scanning and SARIF contribute semantic scan databases, alerts,
  severity/security-severity, taxonomy, rules, locations, related locations,
  code flows, and result exchange. Macaca should model scan result transport and
  normalized severity/taxonomy.

## Macaca-Owned Abstractions

`pack.developer.code.v1` should define `CodeWorkspace`, `CodeDocument`,
`CodeLanguage`, `CodeDiagnostic`, `CodeSymbol`, `CodeReference`,
`CodeSemanticToken`, `CodeAction`, `CodeCommand`, `CodeFormatRequest`,
`CodeWorkspaceEdit`, `CodeParseTree`, `CodeChangedRange`,
`CodeScanResult`, `CodeScanRule`, and `CodeProviderCapability`.

The DTOs must carry workspace/document handles, language identity, versioned
document state, diagnostics, symbol/reference locations, semantic-token
support, edit idempotency, parse-tree snapshots, changed ranges, scan severity,
related locations, provider capability hashes, redaction profile, and replay
pointers. Raw source text, raw provider payloads, secrets, prompt/model output,
repository credentials, and unbounded scan/log output are rejected.

## Explicit Non-Goals

- Do not implement concrete LSP clients/servers, VS Code extensions,
  Tree-sitter parsers, CodeQL engines, SARIF uploaders, model clients,
  repository providers, terminal providers, or build systems in this research
  phase.
- Do not define code-generation workflows, bug-fixing flows, review policies,
  repository flows, or application-specific coding assistants in OS layers.
- Do not expose raw editor APIs, parser nodes, query languages, source text, or
  provider-specific routes as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, repository pack adjacency, terminal pack adjacency, and AI model
  pack adjacency provide reusable substrate.
- Current evidence does not prove code DTOs, providers, SDK helpers, WASM ABI,
  tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
