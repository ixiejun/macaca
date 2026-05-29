# Plan Change

Produce a bounded implementation plan using Macaca generic services.

Required service boundaries:

- Use `service.file` for workspace reads.
- Use `service.code_intelligence` for search and symbol context.
- Use `service.git` for patch provenance and rollback markers.
- Use `service.sandbox` and `service.process` for validation commands.
- Use `service.review` for structured findings.

Do not write OS-layer assumptions, provider names, or business-specific routing.
