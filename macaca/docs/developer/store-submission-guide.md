# Store Submission Guide

Store submission is the path for distributing free, open-source, paid, subscription, metered, encrypted, or commercial packages. Phase 13 defines certification rules and metadata; it does not implement a real marketplace backend.

## Package Metadata

Submissions must declare:

- package id
- developer id
- package type
- runtime kind
- version
- manifest version
- ABI version
- permissions with reasons
- capabilities
- required and optional services
- signature metadata when available
- commerce metadata when the package is paid or Store-gated

## Commerce Metadata

Paid package metadata must declare license type, Store requirement, entitlement id or plan metadata where applicable, metering intent if needed, and offline/revocation policy where applicable. Certification reports entitlement missing as a warning for metadata certification; runtime start remains gated by entitlement services.

## Encrypted Packages

Encrypted skills or paid packages may expose encrypted metadata markers. Certification validates metadata shape and entitlement expectations. It does not decrypt paid assets, bypass Store policy, or enforce payment settlement.

## Trace And Audit

Install, entitlement check, capability metering, encrypted package access, rejection, and revocation paths must be auditable. Certification reports missing trace metadata.

## Certification

Run:

```bash
cargo test -p macaca-integration-tests package_certification
```

Store submission should not require modifying Macaca source code. Packages that fail certification must receive stable diagnostic codes and actionable field paths.
