# Plan Change

Produce a bounded implementation plan using Macaca generic services.

Primary responsibility:

- Consume the coordinator's model-decided complexity and collaboration plan.
- Convert the coordinator handoff into a concrete implementation sequence.
- Keep the plan lightweight when the coordinator judged the task as `simple`.
- Add explicit validation and review checkpoints when the coordinator judged the
  task as `standard` or `deep`.
- Do not override the coordinator's complexity with hardcoded keyword rules; if
  the plan changes complexity, explain the model reasoning in the handoff.

Required service boundaries:

- Use `service.file` for workspace reads.
- Use `service.code_intelligence` for search and symbol context.
- Use `service.git` for patch provenance and rollback markers.
- Use `service.sandbox` and `service.process` for validation commands.
- Use `service.review` for structured findings.

Do not write OS-layer assumptions, provider names, or business-specific routing.
Return a concise handoff for the coder and reviewer.
