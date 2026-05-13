# Change: Add WASM guest SDK bindgen toolchain

## Why

The current SDK scaffold and harness prove contracts, but third-party
developers need generated bindings, local tests, package fixture generation,
and certification commands to build real WASM applications.

## What Changes

- Add provider-neutral WIT bindgen planning and generated Rust guest scaffold
  DTOs.
- Add local mock host-import test runner surfaces.
- Add package descriptor and fixture generation from WIT and manifest inputs.
- Add SDK tests that prevent runtime/toolchain drift.
- Preserve SDK as a facade/developer API rather than a runtime-host provider
  construction point.

## Governance Constraints

- Must follow SDK/SystemFacade ownership rules from Route C governance.
- SDK must not construct runtime-host providers, engine adapters, daemon
  transports, Web state, CLI state, or provider-specific implementations.
- Generated code must target Macaca ABI and host import contracts, not a
  concrete engine or daemon.

## Impact

- Affected specs: `wasm-guest-toolchain`
- Affected code: `macaca-sdk`, runtime guest harness fixtures
