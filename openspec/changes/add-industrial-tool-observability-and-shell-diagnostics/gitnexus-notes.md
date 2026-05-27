# GitNexus Notes

## 2026-05-27

- `agent` index did not find newly introduced tool-service symbols: `ToolServiceProviderState`, `ToolPlanningService::plan`, `ToolInvocationService::invoke`, or `SystemToolClient`.
- `agent-macaca-phase07` index also did not find `ToolServiceProviderState`, `ToolPlanningService`, or `ToolInvocationService`.
- `frontend` impact for `ApplicationOperationsDialog`, `OperationsModeTabs`, and `SkillOperationsPanel` returned LOW risk with zero direct upstream impacts.
- Pre-commit `gitnexus_detect_changes(scope=all)` reported HIGH root risk because this slice touches CLI and Web entrypoints; affected processes were CLI `main` flows and Web `serve_web_server` flows. Frontend detect-changes stayed LOW with no affected processes.
- Per operator instruction, missing or high-risk GitNexus advisory output for this slice is recorded as governance evidence and is not treated as a blocker. Current implementation was validated through focused Rust tests, frontend lint, OpenSpec strict validation, and diff checks.
