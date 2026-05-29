# Codex WASM Workbench Application

This package is a Macaca OS application, not Macaca OS code. It lives outside
`macaca/` so product workflow, UI, and WASM guest logic stay in the application
boundary.

## Purpose

`codex-wasm-workbench` is a Codex-class coding application package built on
Macaca's generic workbench services. The app declares the full coding workbench
surface in `app.yaml` and keeps product behavior in application-owned files:

- `component/` contains the WASM guest source.
- `ui/` contains the app-owned workspace UI bundle.
- `workflows/` contains application-owned coding workflow prompts.
- `skills/` contains optional reusable procedure guidance.

The application does not hardcode Macaca OS behavior. It asks for generic
services such as `service.interaction`, `service.file`, `service.process`,
`service.sandbox`, `service.git`, `service.review`, and `service.diagnostics`.

## Build

```bash
apps/codex-wasm-workbench/scripts/build-wasm.sh
```

The build produces:

```text
apps/codex-wasm-workbench/dist/component/codex_wasm_workbench.wasm
```

The current Macaca WASM runtime can validate/admit WASM metadata and component
contracts. Provider-backed live execution still depends on the target Macaca OS
deployment and its WASM runtime/provider availability.

## Validate Package Files

```bash
apps/codex-wasm-workbench/scripts/validate-package.sh
```

This checks that the app is outside `macaca/`, the manifest declares the generic
workbench services, and the WASM artifact exists after build.

## Runtime Boundary

The app follows the Macaca Codex-class guide:

- Thread, turn, and item state belong to `service.interaction`.
- Transport and subscriptions belong to `service.app_protocol`.
- File, process, sandbox, Git, review, diagnostics, MCP, Skill, and Plugin
  behavior belong to their owning services.
- The app owns UI, prompt text, workflow shape, and coding persona.
- Optional providers must surface structured unavailable states rather than
  fake success.
