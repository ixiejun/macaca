# Review Change

Review the implemented change through Macaca services.

Required outputs:

- Structured findings from `service.review`.
- Diagnostics snapshot from `service.diagnostics`.
- Audit refs for file, Git, process, approval, hook, review, and diagnostics.
- Final Thread/Turn/Item completion through `service.interaction`.

If any provider is unavailable, report the structured unavailable reason and
continue only when the workflow can safely degrade.
