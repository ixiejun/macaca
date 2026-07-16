# Change: Add Developer Code Pack

## Why

Developers need `pack.developer.code.v1` as an industrial code-intelligence
capability for source indexing, syntax parsing, symbol lookup, diagnostics, code
actions, edit planning, patch generation, patch validation, diff inspection,
impact analysis, test suggestion, and structured code-scan results. It must not
be a thin "generate code" prompt wrapper or an application-specific workflow.

Code operations touch host files, repositories, credentials, generated patches,
build artifacts, and sometimes proprietary source. Macaca must expose code
capabilities through a provider-neutral, serviceized, permission-bound,
traceable, auditable, replayable pack. Absent providers, missing workspace
access, denied write permission, unsupported languages, stale indexes, and unsafe
patches must return structured diagnostics instead of fake success.

## Research And Supplier/API Baseline

Official supplier and standards references considered for this pack:

- Language Server Protocol 3.17 defines a common protocol between tools and
  language servers for diagnostics, document symbols, workspace symbols,
  definition, references, formatting, semantic tokens, code actions, commands,
  and workspace edits. Reference:
  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
- Visual Studio Code Extension API exposes extension-facing concepts such as
  workspaces, documents, diagnostics, symbols, code actions, workspace edits,
  commands, authentication, tasks, and language features. References:
  https://code.visualstudio.com/api/references/vscode-api and
  https://code.visualstudio.com/api/language-extensions/programmatic-language-features
- Tree-sitter is an incremental parsing system that builds concrete syntax
  trees and updates them efficiently as source files change, supporting editor
  and code-intelligence scenarios. Reference:
  https://tree-sitter.github.io/tree-sitter/
- GitHub CodeQL documents semantic code analysis, CodeQL databases, queries,
  code scanning alerts, supported languages, and SARIF result exchange.
  References:
  https://docs.github.com/code-security/code-scanning/introduction-to-code-scanning/about-code-scanning-with-codeql
  and https://docs.github.com/en/code-security/concepts/code-scanning/sarif-files

Macaca maps these capabilities into stable DTOs and commands. It does not clone
any provider API and does not place language servers, parsers, security scanners,
model clients, or patch engines in kernel, SDK, shell, or application-framework
layers.

## What Changes

- Add provider-neutral `pack.developer.code.v1` under the `developer` family.
- Define command namespace `code.*` for:
  - workspace/source inventory and indexing
  - document parsing and syntax tree summary
  - symbol lookup and references
  - diagnostics and code scan import/inspection
  - code action discovery
  - edit planning
  - patch generation, validation, application request, and rollback planning
  - diff inspection
  - impact analysis
  - test suggestion
  - provider capability inspection
- Define DTOs for workspace handles, source documents, syntax trees, symbols,
  ranges, diagnostics, code actions, workspace edits, edit plans, patches, diffs,
  impact reports, test suggestions, scan findings, SARIF-like results, provider
  capability, and diagnostics.
- Define permission scopes, policy defaults, host/workspace resource gates,
  approval rules, entitlement checks, structured unavailable behavior, SDK
  discovery, developer documentation, trace/audit events, snapshots, replay, and
  boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/developer/code.md` before implementation completion.

## Impact

- Affected specs: `pack-developer-code`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, code-intelligence
  service provider or unavailable provider, runtime-host provider adapters,
  trace/audit schemas, replay tests, dependency-boundary gates, and developer
  documentation.
- Non-goals: no concrete LSP, VS Code, Tree-sitter, CodeQL, model, repository,
  or terminal implementation in this proposal; no application-specific coding
  workflow; no provider-name routing in OS layers; no raw source or patch
  payloads in observability; no SDK/shell/kernel provider construction; no fake
  success when provider, workspace, entitlement, permission, language, or host
  support is absent.
