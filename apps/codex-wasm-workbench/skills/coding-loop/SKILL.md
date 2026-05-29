# Coding Loop Skill

Use this application-owned skill when a coding request needs the Codex WASM
Workbench service sequence.

## Procedure

1. Start or resume a thread through `service.interaction`.
2. Subscribe to workbench events through `service.app_protocol`.
3. Inspect files through `service.file`.
4. Search code through `service.code_intelligence`.
5. Gate side effects through `service.approval` and `service.hook`.
6. Apply changes through `service.git`.
7. Validate through `service.sandbox` and `service.process`.
8. Produce review findings through `service.review`.
9. Capture diagnostics through `service.diagnostics`.
10. Complete the turn through `service.interaction`.

## Boundary

This skill belongs to the application package. It does not change Macaca OS
service ownership and must not rely on provider names, model names, or app-name
branches below the application layer.
