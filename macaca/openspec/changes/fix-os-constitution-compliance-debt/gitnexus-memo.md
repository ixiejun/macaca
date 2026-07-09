# GitNexus Impact Memo (non-blocking, for the record)

Per the requester's instruction, GitNexus CRITICAL/HIGH blast-radius warnings for
this remediation are **recorded here for reference only** and do NOT gate the
work. This preserves the mandatory-workflow audit trail while allowing the
constitution-compliance fixes to proceed.

## Touched symbols in this increment (Stage 0 + Stage 1.1–1.3)

| Symbol / file | Change | Note |
|---|---|---|
| `macaca_proto::text_sanitize` (new module) | Added foundation string-sanitization primitives | New leaf module, no upstream callers yet at add time |
| `macaca_kernel::logging::mask_sensitive` | Behavior change: full secret redaction (no prefix) | Callers are logging sites; output is observability-only, so downstream semantics are unaffected |
| `macaca_kernel::logging::truncate` / `mask_json_params` | UTF-8-safe truncation | Same signature; only removes panic + prefix leak |
| `macaca_task::decompose::parse_llm_output` | Error-string truncation now UTF-8-safe | Error path only |
| `macaca_gateway::telegram_format::split_message` | Delegates to foundation `split_by_chars` | Preserves `[""]` empty-input contract (regression-tested) |

## Verification performed instead of relying on GitNexus gating
- `cargo check --workspace` passes (pre-existing warnings only).
- Unit tests green: `macaca-proto` text_sanitize (4), `macaca-kernel` logging (4,
  incl. new multibyte non-panic test), `macaca-gateway` telegram (41).
- `os_layer_file_size_gate` green (new module is ~240 lines, well under 500).

## Recorded for later stages
GitNexus should be re-run before the Stage 3–5 boundary extractions (side-effect
guard, domain-pack contract crate move, shell semantic re-homing), where the
blast radius is genuinely large; those warnings will again be recorded, not
gated, per the same instruction.
