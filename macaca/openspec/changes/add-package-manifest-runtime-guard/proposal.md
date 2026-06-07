# Change: Add package manifest runtime guard

## Why

Route C Phase 04 must establish Macaca OS package metadata and a runtime guard before the system can safely support YAML applications, future WASM applications, plugins, skills, MCP packages, drivers, system modules, and UI component packs through one software distribution model.

Today each surface has its own manifest shape or loader assumptions. That makes compatibility checks, permission checks, trace/audit records, optional module handling, future Store distribution, paid package metadata, and hot installation harder to make consistent without application-specific or provider-specific branches.

## What Changes

- Add provider-neutral Package Manifest v0 contracts in `macaca-proto` for package identity, package type, version, developer id, signature metadata, runtime kind, runtime ABI version, entry, permissions, required services, optional services, provided capabilities, inert commerce metadata, and OS compatibility.
- Add an Application Framework package descriptor path in `macaca-app` that can represent existing YAML `app.yaml` applications as first-class packages rather than second-class legacy inputs.
- Add a YAML application compatibility adapter that maps existing app manifest data into a package descriptor while preserving app id, entry agent, entrypoint/workflow data, required services, agent capabilities, and allowed tools.
- Add a runtime guard validation chain using Specification and Chain of Responsibility patterns: parse/schema validation, signature metadata validation, compatibility validation, permission validation, service requirement validation, optional service availability marking, and inert commerce precheck.
- Add a package loader factory that selects loaders by runtime kind, with real YAML metadata loading and a non-executing WASM metadata stub that returns structured `RuntimeUnavailable` when execution is requested without a runtime.
- Add adapter skeletons or descriptor conversion hooks for skill, driver, and runtime-host package metadata without migrating all existing loaders in this phase.
- Add trace/audit/logging boundaries for package parse, validation, guard decisions, loader selection, optional service degradation, and rejection outcomes.
- Require detailed English comments in all new Rust code explaining package contracts, guard chain operation, trace/audit behavior, loader selection, compatibility rules, and non-goal boundaries.

## Impact

- Affected specs: `package-manifest-runtime-guard`
- Affected crates: `macaca-proto`, `macaca-app`, `macaca-skill`, `macaca-driver`, `macaca-runtime-host`, and verification through `macaca-web`
- Affected tests: package serde tests in `macaca-proto`, package manifest/runtime guard tests in `macaca-app`, targeted descriptor conversion tests in skill/driver/runtime-host where implemented
- Regression matrix references: `RC-APP-001`, `RC-CHAT-001`

## Governance Alignment

- Follows `macaca/docs/agent-os-microkernel-boundaries.md`: package runtime guard is a kernel-level invariant, while application manifests, compatibility adapters, package loaders, plugin runtimes, and optional modules remain outside business workflow code.
- Follows `macaca/docs/route-c-regression-matrix.md`: Phase 04 must preserve YAML application loading and `/api/chat/v2` session creation behavior.
- Follows `macaca/docs/route-c-phase-template.md`: OpenSpec first, additive-first implementation, GitNexus impact before symbol edits, targeted tests, integration smoke, detect_changes before commit.
- Follows `macaca/docs/route-c-architecture-governance.md`: package load decisions must be traceable, policy-ready, auditable, structured on failure, and free of app/provider/driver/gateway/chain hardcoding.

## Non-Goals

- Do not implement a Store, package marketplace, payment flow, subscription billing, entitlement backend, or encrypted paid package enforcement in Phase 04.
- Do not implement WASM execution; WASM package metadata may load, but execution must return structured `RuntimeUnavailable` until the Application ABI/runtime phase.
- Do not migrate all skill, driver, MCP, plugin, runtime-host, web, or CLI loading paths to package descriptors in Phase 04.
- Do not remove or downgrade YAML applications; YAML applications must remain first-class and compatible.
- Do not move application workflow behavior, service provider execution, driver execution, skill runtime execution, MCP runtime execution, Web3, EVM, or GenUI rendering into the package guard.
- Do not hardcode demo application names, workflow names, provider names, driver names, gateway names, chain names, or business-specific routing in package manifests or guard rules.
