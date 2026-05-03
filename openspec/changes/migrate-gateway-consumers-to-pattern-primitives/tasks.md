## 1. Preparation

- [x] 1.1 Review gateway refactor plan and existing gateway OpenSpec change.
- [x] 1.2 Confirm direct Cargo consumers are only `macaca-cli` and `macaca-integration-tests`.
- [x] 1.3 Run GitNexus impact for `run_kernel`.
- [x] 1.4 Scan deprecated gateway usage across direct consumers.

## 2. OpenSpec

- [x] 2.1 Create proposal, design, tasks, and delta spec.
- [x] 2.2 Validate `migrate-gateway-consumers-to-pattern-primitives` with `openspec validate --strict`.

## 3. Production Consumer Migration

- [x] 3.1 Keep `macaca-cli::run_kernel` on `GatewayBuilder`.
- [x] 3.2 Verify `macaca-cli` does not import or call deprecated `Gateway`, `ImAdapter`, `EventHandler`, `TelegramAdapter`, or `DiscordAdapter` lifecycle APIs.

## 4. Integration Coverage Migration

- [x] 4.1 Replace legacy lifecycle integration tests with `GatewayBuilder` lifecycle coverage.
- [x] 4.2 Replace direct `EventHandler` tests with `GatewayMediator` / `GatewayEventSink` tests.
- [x] 4.3 Replace `ImAdapter::send_message` tests with non-deprecated `GatewayTransport::send_reply` tests.
- [x] 4.4 Remove top-level `#[allow(deprecated)]` from gateway integration tests.

## 5. Verification

- [x] 5.1 Run `cargo fmt`.
- [x] 5.2 Run `cargo test -p macaca-gateway -- --nocapture`.
- [x] 5.3 Run `cargo test -p macaca-integration-tests gateway -- --nocapture`.
- [x] 5.4 Run `cargo check -p macaca-gateway -p macaca-cli -p macaca-integration-tests`.
- [x] 5.5 Run deprecated gateway usage grep for upper consumers.
- [x] 5.6 Run `openspec validate migrate-gateway-consumers-to-pattern-primitives --strict`.
- [x] 5.7 Run `npx gitnexus detect-changes --repo agent --scope all`.
