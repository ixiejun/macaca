## ADDED Requirements

### Requirement: New Payment / A2A consumers SHALL use service-first paths

New production Payment / A2A consumers SHALL use Payment Service through SDK clients rather than direct kernel coordinator or adapter execution APIs.

#### Scenario: Kernel coordinator remains compatibility-only

- **WHEN** S10 introduces the Payment Service path
- **THEN** existing kernel A2A coordinator and adapter APIs SHALL remain available for compatibility
- **AND** they SHALL be marked deprecated with replacement guidance to Payment Service and `SystemPaymentClient`

#### Scenario: New consumer avoids direct payment coordinator construction

- **WHEN** a new Web, CLI, Gateway, Application Framework, or agent-facing code path needs payment quote, approval, settlement, or receipt behavior
- **THEN** it SHALL use `SystemPaymentClient`
- **AND** it SHALL NOT directly instantiate `A2ACoordinator`, `LocalSimulatedA2AAdapter`, `PaymentStore`, or runtime-host provider types

### Requirement: Web and CLI SHALL remain thin shells for Payment / A2A

Web and CLI SHALL only act as composition root, command adapter, approval surface, status renderer, or trace viewer for Payment / A2A.

#### Scenario: Web startup may register built-in Payment Service

- **WHEN** Web initializes the local runtime-host composition root
- **THEN** it MAY register and start the built-in local simulated Payment Service
- **AND** Web SHALL NOT own payment lifecycle semantics, policy rules, adapter selection semantics, Store semantics, Entitlement semantics, Web3 semantics, EVM semantics, or provider-specific payment logic

#### Scenario: CLI payment surface uses SystemFacade

- **WHEN** a future CLI payment command inspects receipts, approvals, or service status
- **THEN** it SHALL use `SystemFacade` or focused SDK clients
- **AND** it SHALL NOT depend on Web internals or concrete payment provider implementations

### Requirement: Payment / A2A migration SHALL preserve existing task and trace behavior

S10 migration SHALL be additive-first and preserve existing `/api/chat/v2`, session, task, trace, resume, and no-payment application flows.

#### Scenario: No-payment task flow remains unaffected

- **WHEN** a normal application task does not require Payment / A2A capability
- **THEN** it SHALL continue to run without a payment provider, wallet, Web3 module, EVM module, external network, or payment approval
- **AND** existing trace/event behavior SHALL not regress because Payment Service is unavailable

#### Scenario: Payment-required flow fails closed

- **WHEN** an A2A paid capability requires payment and Payment Service or approval is unavailable
- **THEN** the payment-required flow SHALL return structured unavailable, denied, approval-required, over-budget, or adapter-unavailable status
- **AND** it SHALL NOT silently allow execution

### Requirement: Payment / A2A implementation SHALL not hardcode ecosystem-specific names

Payment / A2A serviceization SHALL not introduce control flow hardcoded to any application, workflow, provider, driver, gateway, model, chain, token, payment vendor, tenant, or business-specific name.

#### Scenario: Adapter and rail identities remain data-driven

- **WHEN** a payment rail, adapter, provider, or requester/provider identity is evaluated
- **THEN** the implementation SHALL treat it as provider-neutral data, descriptor, metadata, or Strategy selection input
- **AND** it SHALL NOT route behavior through hardcoded names such as one specific app, chain, driver, payment vendor, gateway, or model

