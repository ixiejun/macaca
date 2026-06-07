# Change: Add OS-Code Evolution Proposal Adapter

## Why

Macaca's self-evolution loop now has a control plane, admission gates,
normalized benchmarking, release safety, and a governance ledger boundary. The
remaining complete-self-evolution slice is OS-code evolution. That path must be
non-mutating first: agents may propose governed source changes, but source
mutation is blocked until OpenSpec, Superpowers, GitNexus impact evidence,
tests, release gates, and human/policy approval are present.

## What Changes

- Add a provider-neutral OS-code proposal adapter Strategy that emits
  OpenSpec/Superpowers/GitNexus proposal bundles.
- Require proposal, design, tasks, impact, expected tests, release gate, and
  rollback evidence refs before a proposal can be considered ready for later
  execution.
- Return explicit non-mutating output: the adapter never writes source files,
  runs shell commands, commits, or applies patches.
- Add tests proving safe proposal creation, missing gate denial, high
  blast-radius quarantine, and source mutation refusal.

## Impact

- Affected specs: `autonomy-evolution-control-plane`
- Affected code:
  - `macaca/crates/services/macaca-autonomy-evolution`
  - targeted service tests for the OS-code proposal adapter
