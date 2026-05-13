## Context

Current admission validates artifact digest and ABI metadata. It does not
validate signatures, signer trust, source origin, or build provenance.

## Goals / Non-Goals

- Goals: signed artifact DTOs, provenance DTOs, deterministic test
  verification, admission integration, certification report compatibility, and
  sanitized audit reason codes.
- Non-Goals: production KMS integration, one Store-specific policy, raw key
  material logging, kernel-owned package trust decisions, or presentation-shell
  admission semantics.

## Decisions

- Use Specification for verification rules.
- Use Memento-style sanitized verification reports.
- Keep trust policy provider-neutral so Store, CI, or an enterprise compliance
  service can supply trusted signer sets later.
- Start with deterministic verifier traits and fixtures before adding any real
  cryptographic provider dependency.

## Governance

Package admission belongs to Application Framework and provider-neutral proto
DTOs. Trust policy integration must remain service/facade compatible and must
not make Store, Web, CLI, or kernel the owner of WASM artifact semantics.

## Risks / Trade-offs

- Crypto dependencies can increase build scope. Mitigation: start with
  deterministic verifier traits and test fixtures, then add real crypto only
  behind approved dependency review.
- Provenance records can leak source paths or credentials. Mitigation: reports
  expose safe origin labels and stable reason codes only.

## Migration Plan

Existing packages without signatures remain non-industrial-ready until policy
explicitly allows development mode.
