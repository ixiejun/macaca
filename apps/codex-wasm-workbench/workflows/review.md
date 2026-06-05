# Review Change

Review the implemented change through Macaca services.

Primary responsibility:

- Consume coordinator, planner, and coder handoffs before reviewing.
- Match review depth to the coordinator's model-decided complexity. Simple tasks
  still need a traceable review outcome, while standard or deep tasks need
  explicit findings, validation gaps, and residual risks.
- Never invent validation evidence. Distinguish verified facts from unverified
  assumptions.

Required outputs:

- Structured findings from `service.review`.
- Diagnostics snapshot from `service.diagnostics`.
- Audit refs for file, Git, process, approval, hook, review, and diagnostics.
- Final Thread/Turn/Item completion through `service.interaction`.

If any provider is unavailable, report the structured unavailable reason and
continue only when the workflow can safely degrade.
Return a user-facing review summary that is readable without inspecting raw JSON.
