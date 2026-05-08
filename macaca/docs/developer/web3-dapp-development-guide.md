# Web3 And DApp Development Guide

Web3 and EVM/DApp support are optional Macaca OS modules. A base OS installation must keep working when these modules are unavailable. Applications can declare optional or required capability metadata, but concrete nodes, wallets, signing providers, and EVM execution remain service/provider concerns.

## Optional Web3

Optional Web3 packages declare an optional service such as a Web3 node service. Certification reports unavailable optional modules as warnings and keeps the package metadata-compatible.

## Optional EVM / DApp

Optional EVM/DApp packages declare EVM runtime capability metadata and optional EVM service requirements. Missing EVM runtime must become a structured warning unless the package declares it as required.

## Required Modules

If a package declares Web3 or EVM as a required service, certification fails when the host cannot provide it. This prevents hidden runtime hangs and makes deployment constraints explicit.

## Trace And Audit

Wallet access, signing requests, chain queries, transaction submission, contract calls, optional module degradation, and policy denial must emit trace/audit events. Certification validates metadata and unavailable-safe behavior without connecting to a real chain.

## Store And Entitlement

Paid Web3 or DApp packages also follow Store submission rules. A2A payment and chain settlement remain separate service contracts and must not be hardcoded into application packages.

## Certification

Run:

```bash
cargo test -p macaca-integration-tests package_certification
```

Certification checks Web3 and EVM packages without requiring a blockchain node or EVM runtime.
