# Change: Add Developer Pack Capability Platform

## Why

Macaca OS already has a small domain-pack mechanism and one finance package, but application developers need a broader, discoverable, versioned, and serviceized pack ecosystem comparable to the capability surfaces exposed by macOS, Windows, Android, and HarmonyOS. The initial pack model must support useful early application development while accepting that the first catalog is incomplete and must evolve without rewriting the microkernel, service runtime, SDK, or application ABI.

## Research Summary

Official platform documentation converges on the same operating-system pattern:

- Apple developer documentation organizes application power through frameworks, capabilities, entitlements, app services, background execution, CloudKit, notifications, payments, health/home/media APIs, and privacy-scoped declarations.
- Microsoft Windows documentation exposes developer value through Windows App SDK, app lifecycle, app services, notifications, storage, identity, packaging, deployment, and extension points while keeping platform services behind APIs and manifests.
- Android documentation exposes app components, permissions, Jetpack libraries, WorkManager, Room, foreground/background services, content providers, sensors, media, location, billing, and Play distribution as declared and permissioned capabilities.
- HarmonyOS documentation exposes application abilities, ExtensionAbility patterns, module declarations, permissions, distributed/device capabilities, ArkUI, Data/Network/AI/Media/Map/Payment-style kits, and developer kits as capability families.

The common lesson is not to put business features into the kernel. Mature operating systems expose a stable developer surface as versioned kits/packs backed by manifests, permissions, lifecycle policy, SDK clients, diagnostics, and replaceable providers. Macaca should follow that model with pack families and sub-packs that expand to service contracts and application permissions while all execution remains on the canonical service path.

Reference entry points used for this proposal:

- Apple Developer Documentation: https://developer.apple.com/documentation/
- Apple Entitlements: https://developer.apple.com/documentation/bundleresources/entitlements
- Microsoft Windows App SDK: https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/
- Android App Manifest and App Fundamentals: https://developer.android.com/guide/topics/manifest/manifest-intro
- Android App Architecture: https://developer.android.com/topic/architecture
- HarmonyOS Guides: https://developer.huawei.com/consumer/en/doc/harmonyos-guides/

## What Changes

- Define a generic Developer Pack Platform as a developer-facing capability layer above service contracts and below applications.
- Extend the pack model from `pack_id + services` toward versioned pack families, sub-packs, service contracts, permission scopes, policy templates, lifecycle hooks, data governance, and SDK discovery metadata.
- Keep pack implementations outside the base runtime host and register them as optional package/plugin providers through descriptor-owned service registrations.
- Add a pack catalog architecture that supports partial initial coverage and incremental extension without OS-layer business branches.
- Specify admission, resolution, trace, audit, health, snapshot, and unavailable behavior for packs and sub-packs.
- Provide an implementation roadmap for foundational packs and future pack families without committing all domains in the first release.

## Impact

- Affected specs: `developer-pack-platform`, `service-runtime`, `sdk-system-facade`, `unified-execution-path`, `web-cli-thin-shell-completion`.
- Affected code later: `macaca-proto` pack contracts, `macaca-app` manifest/admission projection, `macaca-sdk` discovery clients, `macaca-runtime-host` generic registration/adapters, optional package crates under `crates/packages/`, boundary/audit tests.
- Non-goal: implementing concrete finance, office, media, or device business logic in this proposal.
