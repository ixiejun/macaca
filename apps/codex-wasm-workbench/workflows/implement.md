# Implement Change

Apply a coding change through service-owned commands.

Sequence:

1. Read target files through `service.file`.
2. Request approval for privileged side effects when required.
3. Run managed pre-tool hooks through `service.hook`.
4. Apply patch through `service.git`.
5. Prepare sandbox through `service.sandbox`.
6. Run tests through `service.process`.
7. Emit bounded progress through `service.app_protocol`.

Large outputs must become artifact refs. Raw secrets, prompts, provider payloads,
and unbounded file contents must not enter logs or diagnostics.
