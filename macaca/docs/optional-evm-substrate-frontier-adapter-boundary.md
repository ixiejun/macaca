# Optional EVM / Substrate / Frontier Adapter Boundary

Route C Phase 11 defines EVM/DApp support as an optional Web3 submodule. Macaca does not implement a custom EVM and does not require a real Substrate/Frontier node, RPC provider, browser wallet, or external network in the base OS.

## Boundary Summary

The EVM layer is a provider-neutral command surface. Applications and SDKs construct commands. Kernel/service code evaluates availability, policy, trace, and audit. Optional adapters translate approved commands into provider-specific behavior.

## Provider Adapter Responsibilities

Future Substrate, Frontier, EVM RPC, local sandbox, enterprise proxy, or plugin adapters own:

- Mapping Macaca `EvmChainId`, `ContractAddress`, `ContractAbiRef`, and `ContractFunctionRef` values to provider-specific values.
- Encoding ABI invocation payloads and decoding provider responses into bounded result digests.
- Normalizing provider-specific errors into `EvmError`.
- Normalizing deploy, call, read, subscription, gas estimate, and receipt results into Macaca mementos.
- Managing transport concerns such as RPC clients, local node handles, WebSocket subscriptions, retry behavior, and provider backpressure.
- Redacting sensitive payloads before returning data to the kernel/service boundary.

Adapters must not bypass Macaca policy. They execute only after the optional EVM facade has approved the command.

## Kernel / Service Boundary Responsibilities

Kernel and service-facing code own:

- Optional module availability.
- Adapter registration and replacement.
- Policy orchestration for signing, payment, gas, permission scope, and compliance.
- Trace and audit event emission.
- Structured unavailable, policy-denied, and adapter-failed outcomes.
- No-op/null behavior when EVM is absent.

Kernel code must not implement provider-specific RPC, Substrate, Frontier, ABI codec, browser wallet, private-key, token, application, workflow, or business routing logic.

## Application and SDK Responsibilities

Application/package metadata owns capability intent only:

- Declare optional DApp/EVM capability such as `web3.evm`.
- Provide chain/account/contract intent as metadata and policy input.
- Treat unavailable or denied EVM as structured data.

SDK code owns command construction only:

- Build provider-neutral deploy, call, read, subscription, gas, and receipt commands.
- Delegate commands to the optional EVM service boundary.
- Never instantiate concrete providers or bypass policy, trace, or audit paths.

## Web Shell Responsibilities

Web-facing code remains a thin shell:

- Display availability, denial, approval prompts, and bounded audit status when exposed by service APIs.
- Do not define EVM provider semantics, signing semantics, gas semantics, contract execution semantics, or policy rules.

## Mock Adapter Rule

The mock adapter exists for deterministic no-network tests. Its outputs must include simulated provenance and must never be treated as real chain evidence.

## Trace and Audit Rule

EVM logs and trace events should include bounded identifiers such as chain id, operation, request id, contract address, transaction id, receipt id, session id, task id, status, timestamp, and error code. They must not include private keys, seed phrases, credentials, raw encrypted payloads, provider secrets, raw unbounded ABI arguments, or unredacted signatures.
