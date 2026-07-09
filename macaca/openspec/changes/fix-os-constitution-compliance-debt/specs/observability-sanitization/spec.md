## ADDED Requirements

### Requirement: UTF-8-Safe Truncation Is The Only Truncation Primitive

All string truncation on OS observability, error, and payload paths SHALL use a
shared UTF-8-safe truncation primitive that computes a character boundary and
never indexes a `&str` by a raw byte offset. Direct byte-range slicing of
`&str` on production paths SHALL be prohibited.

#### Scenario: Multibyte input does not panic
- **WHEN** a value containing multibyte characters (e.g. Chinese text or emoji) is
  truncated for logging, error construction, or splitting
- **THEN** truncation SHALL return a valid string on a character boundary without
  panicking

#### Scenario: Byte-slice truncation is gated out
- **WHEN** the sanitization gate scans OS-layer production source
- **THEN** it SHALL flag any `&s[..n]`-style byte slice used for truncation of a
  `&str` value

### Requirement: Observability Surfaces Redact By Structural Allow-List

Logs, traces, snapshots, and error strings SHALL NOT contain raw secrets,
prompts, provider payloads, private keys, credentials, or unbounded output.
Redaction SHALL use a structural allow-list (accepting only controlled
identifier/URI shapes and rejecting or hashing everything else), not a
literal-word deny-list. Raw provider response bodies SHALL be redacted and
length-bounded before entering any error or log string.

#### Scenario: Secret-shaped value is redacted even without a keyword
- **WHEN** a value resembling an API key or bearer token appears in a reference,
  metadata field, or error body without a `secret`/`credential` keyword
- **THEN** the sanitizer SHALL redact or hash it rather than emit it verbatim

#### Scenario: Provider error body is bounded and sanitized
- **WHEN** a provider returns a non-2xx response whose body flows into an error or
  log
- **THEN** the body SHALL be passed through redaction and length-bounding before
  inclusion, consistently across all provider adapters

### Requirement: Metadata And References Entering Snapshots Are Bounded

Free-text and metadata entering snapshots, audit records, or event payloads SHALL
be key allow-listed and length-bounded, and diagnostic collections SHALL have an
explicit upper bound with aggregate counting beyond the bound.

#### Scenario: Snapshot metadata is allow-listed and capped
- **WHEN** caller-supplied metadata is written into a run/task/service snapshot
- **THEN** only allow-listed keys SHALL be retained and each value SHALL be length
  bounded

#### Scenario: Diagnostics are bounded
- **WHEN** replay or validation produces many malformed-line diagnostics
- **THEN** the output SHALL retain at most a fixed number and report an aggregate
  count for the remainder
