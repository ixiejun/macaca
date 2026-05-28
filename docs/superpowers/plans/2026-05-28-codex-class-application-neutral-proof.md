# Superpowers Brainstorm and Plan: Codex-class Application-neutral Proof

## Brainstorm

Goal: implement section 19 of `complete-codex-class-application-support` as an
application-neutral executable proof, not a product-specific shortcut.

Options considered:

- Add a production proof runner service.
  - Benefit: reusable runtime entrypoint.
  - Risk: adds new OS behavior before the proof boundary is understood.
- Add an integration proof fixture that composes already landed generic
  services.
  - Benefit: proves the accepted contracts and provider-backed workflow without
    widening production semantics.
  - Risk: needs careful evidence collection so it is not just descriptor
    visibility.
- Add only documentation evidence.
  - Benefit: fast.
  - Risk: fails the proposal completion definition because no provider-backed
    workflow runs.

Selected approach: integration proof fixture. It uses the existing Command,
Strategy, Observer, Facade, and Memento patterns already present in the
workbench service family. The fixture stays in the test layer, constructs a
generic application manifest, and exercises service-owned commands with trace
and bounded evidence.

## Plan

1. Add an application-neutral manifest fixture declaring the full workbench
   capability set, generic service dependencies, permission profiles, tool
   families, MCP/skill/plugin declarations, event subscriptions, and UI surface
   metadata.
2. Add an integration test that runs a real temp repository workflow through
   service-owned providers:
   - Start thread/turn/items through `service.interaction`.
   - Inspect and mutate files through `service.file`.
   - Search through `service.code_intelligence`.
   - Apply a patch and replay rollback marker through `service.git`.
   - Prepare a sandbox and run a bounded process through `service.process`.
   - Create/resolve approval through `service.approval`.
   - Run pre/post hooks and replay hook audit through `service.hook`.
   - Invoke a skill-family tool through `service.tool`.
   - Produce review findings through `service.review`.
   - Produce diagnostics and trace bundles through `service.diagnostics`.
   - Emit app-protocol notifications for all proof evidence classes.
3. Update the OpenSpec task checklist for section 19 after the proof passes.
4. Run focused validation, OpenSpec strict validation, GitNexus detect changes
   or equivalent available checks, then commit only the intended files.
