# Implement Change

Apply a coding change through service-owned commands.

Primary responsibility:

- Consume both the coordinator collaboration plan and the planner handoff before
  editing or generating artifacts.
- Match implementation depth to the model-decided complexity. Simple tasks may
  produce a small artifact, while standard or deep tasks should preserve the
  planned architecture, validation points, and review evidence.
- Keep all work application-owned and workspace-scoped. Do not add Codex-specific
  behavior to Macaca OS, service runtime, SDK, Web shell, or frontend code.

Sequence:

1. When the delegated task asks for files or an application artifact, call
   `file_write` for every required file before the final response.
2. Place generated files under `delegated_context.workspace.shared_path` unless
   the delegated task provides a more specific workspace-scoped evidence path.
3. Request approval for privileged side effects when required.
4. Run managed pre-tool hooks through `service.hook` when those service tools
   are declared for the current run.
5. Apply patch or Git operations through service-owned commands when available.
6. Prepare sandbox and run tests through service-owned process commands when
   those tools are declared for the current run.
7. Emit bounded progress through `service.app_protocol`.

Completion rule:

- A natural-language answer without the requested `file_write` calls is not a
  completed filesystem task.
- Never write outside the delegated workspace.
- Return a coder handoff that lists files written, tool calls performed,
  validation status, and any remaining risk for reviewer consumption.

Large outputs must become artifact refs. Raw secrets, prompts, provider payloads,
and unbounded file contents must not enter logs or diagnostics.
