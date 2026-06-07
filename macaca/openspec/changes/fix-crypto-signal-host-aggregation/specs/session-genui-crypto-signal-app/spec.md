## MODIFIED Requirements
### Requirement: Crypto signal app host aggregation
The WASM crypto signal app SHALL aggregate successful market, news, technical, sentiment, and risk evidence into the final LLM analysis payload even when earlier host-command results include fail-closed entries.

#### Scenario: Delegated technical evidence is available
- **WHEN** a crypto signal pipeline dispatches market data, news, three delegated agents, final analysis, and UI render
- **THEN** the final analysis payload includes the successful delegated technical, sentiment, and risk outputs by their actual host-command result indexes
- **AND** the UI render uses the final analysis output from the matching result index

### Requirement: Finance market data import error semantics
The WASM host import bridge SHALL distinguish policy denials from service failures in host-command status metadata and runtime status.

#### Scenario: Primary market data source fails but fallback succeeds
- **WHEN** the primary no-key crypto quote source is unavailable
- **THEN** the finance market-data provider attempts a second no-key public quote source
- **AND** returns a normalized market snapshot without surfacing DisabledByPolicy

#### Scenario: All market data sources fail
- **WHEN** every live crypto quote source is unavailable
- **THEN** the host-command result reports a service-unavailable style status, not a policy-denied status

#### Scenario: Market data receives an untyped prompt
- **WHEN** a finance market-data service call receives raw chat prose instead of a typed `symbol` or `ticker`
- **THEN** the service returns a structured `InvalidArgument` error
- **AND** the WASM host-command result reports a rejected invalid-argument reason rather than parsing the prompt

### Requirement: Typed chat payload ownership
The WASM crypto signal app SHALL provide typed service arguments through app-owned metadata or coordinator output before invoking finance services.

#### Scenario: App-owned typed symbol is supplied
- **WHEN** a WASM component host-command payload references `${chat.symbol}`
- **AND** the app/coordinator supplies a structured chat payload containing `symbol`
- **THEN** Macaca resolves the exact JSON field into the service payload
- **AND** Macaca does not infer the field from `${chat.input}`
