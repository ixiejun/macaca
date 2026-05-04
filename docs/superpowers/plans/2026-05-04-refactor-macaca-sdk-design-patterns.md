# macaca-sdk Design Pattern Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `macaca-sdk` with additive design-pattern primitives so SDK agent declaration, persona reuse, validation, registration, and trace policy have stable boundaries without changing current behavior.

**Architecture:** Use additive-first primitives. Add `AgentSpec` as the builder product, `PersonaPrototype` as clone/override support, `SdkValidationChain` as the validation pipeline, and `MacacaSdk` as a facade over registry adapters. Existing APIs stay callable and delegate to the new primitives; behavior remains 1:1.

**Tech Stack:** Rust, `macaca-sdk`, `macaca-agent`, `macaca-kernel`, `macaca-proto`, `macaca-tools`, OpenSpec, GitNexus, cargo test/check.

---

## Current Context

`macaca-sdk` currently has five source files:

- `macaca/crates/macaca-sdk/src/lib.rs`
- `macaca/crates/macaca-sdk/src/config.rs`
- `macaca/crates/macaca-sdk/src/builder.rs`
- `macaca/crates/macaca-sdk/src/persona.rs`
- `macaca/crates/macaca-sdk/src/registry_api.rs`

Current key APIs:

- `AgentConfig::from_yaml`
- `AgentConfig::from_toml`
- `AgentConfig::from_file`
- `AgentConfig::validate`
- `AgentBuilder::from_config`
- `AgentBuilder::build`
- `AgentBuilder::build_with_manifest`
- `DeclarativeAgent`
- `AgentPersona::load_from_directory`
- `AgentPersona::to_system_prompt`
- `register_from_config`
- `register_from_file`

GitNexus notes:

- `AgentBuilder`, `DeclarativeAgent`, and `AgentPersona` have LOW upstream risk in the current index.
- `register_from_config` has CRITICAL upstream risk because it is used by `macaca-app::start_app`, web startup app loading, and integration tests.
- Therefore implementation must first add builder/persona/validation primitives and only later wrap registry API without changing registration semantics.

## File Map

### OpenSpec

- Create: `openspec/changes/refactor-macaca-sdk-patterns/proposal.md`
- Create: `openspec/changes/refactor-macaca-sdk-patterns/design.md`
- Create: `openspec/changes/refactor-macaca-sdk-patterns/tasks.md`
- Create: `openspec/changes/refactor-macaca-sdk-patterns/specs/macaca-sdk-patterns/spec.md`

### SDK source

- Create: `macaca/crates/macaca-sdk/src/spec.rs`
  - Owns `AgentSpec`, `AgentSpecBuilder`, `TracePolicy`, and conversion helpers.
- Modify: `macaca/crates/macaca-sdk/src/builder.rs`
  - Add `AgentBuilder::build_spec`.
  - Make `build` / `build_with_manifest` delegate to `AgentSpec`.
- Create: `macaca/crates/macaca-sdk/src/persona_prototype.rs`
  - Owns `PersonaPrototype` and `PersonaOverrides`.
- Modify: `macaca/crates/macaca-sdk/src/persona.rs`
  - Keep existing loader and prompt behavior unchanged.
  - Add helper method only if needed by `PersonaPrototype`.
- Create: `macaca/crates/macaca-sdk/src/validation.rs`
  - Owns `SdkValidator`, `SdkValidationChain`, and default validators.
- Modify: `macaca/crates/macaca-sdk/src/config.rs`
  - Route `AgentConfig::validate` through `SdkValidationChain::default`.
- Create: `macaca/crates/macaca-sdk/src/facade.rs`
  - Owns `MacacaSdk`, `AgentRegistryApi`, and kernel registry adapter.
- Modify: `macaca/crates/macaca-sdk/src/registry_api.rs`
  - Keep old functions, mark deprecated after facade exists, delegate to `MacacaSdk`.
- Modify: `macaca/crates/macaca-sdk/src/lib.rs`
  - Export new additive primitives.

## Task 1: Create OpenSpec Change

**Files:**

- Create: `openspec/changes/refactor-macaca-sdk-patterns/proposal.md`
- Create: `openspec/changes/refactor-macaca-sdk-patterns/design.md`
- Create: `openspec/changes/refactor-macaca-sdk-patterns/tasks.md`
- Create: `openspec/changes/refactor-macaca-sdk-patterns/specs/macaca-sdk-patterns/spec.md`

- [ ] **Step 1: Review OpenSpec context**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec list
openspec list --specs
```

Expected:

```text
refactor-macaca-sdk-patterns does not already exist.
```

- [ ] **Step 2: Create proposal**

Create `openspec/changes/refactor-macaca-sdk-patterns/proposal.md`:

```markdown
# Change: Refactor macaca-sdk with design pattern primitives

## Why

`macaca-sdk` is the developer-facing boundary for declaring agents and registering them into Agent OS. Today the SDK builder directly constructs runtime agents, persona reuse is copy-based, validation is a monolithic method, and registry helpers directly depend on `Kernel`.

## What Changes

- Add `AgentSpec` as the SDK builder product while preserving existing `DeclarativeAgent` behavior.
- Add persona prototype and override primitives.
- Add SDK validation chain primitives and route current validation through the default chain.
- Add `MacacaSdk` facade and registry adapter primitives.
- Require SDK-built agent specs to carry trace policy metadata.
- Keep old helper functions callable but mark replaced registry helpers deprecated after facade exists.

## Impact

- Affected specs: `macaca-sdk-patterns`
- Affected code: `macaca-sdk`, selected tests in `macaca-app` / `macaca-integration-tests` only if needed for compatibility
- Non-impact: no app runtime behavior change; no kernel registration behavior change; no trace/EventLog/SSE/task/driver/skill/MCP behavior change.
```

- [ ] **Step 3: Create design**

Create `openspec/changes/refactor-macaca-sdk-patterns/design.md`:

```markdown
## Context

`macaca-sdk` exposes declarative config parsing, agent builder, persona loading, and registry helpers. It is consumed by `macaca-app`, `macaca-web`, integration tests, and application developers.

`register_from_config` is a critical path because app startup uses it to register application agents. This change must be additive-first and must not change `Kernel::register_agent` semantics.

## Goals

- Keep existing behavior 1:1 compatible.
- Add `AgentSpec` as a stable SDK declaration product.
- Add persona prototype clone/override support.
- Split validation into a chain of small validators while preserving current validation results.
- Add an SDK facade and registry adapter without changing registration behavior.
- Ensure SDK-built agent specs carry trace policy metadata.

## Non-Goals

- Do not remove `DeclarativeAgent`.
- Do not remove `AgentBuilder::build` or `build_with_manifest`.
- Do not remove `register_from_config` or `register_from_file`.
- Do not connect SDK directly to web/session/EventLog/SSE in this change.
- Do not migrate app/web consumers to the new facade in this change unless required by tests.
- Do not introduce new dependencies.
- Do not hardcode application, workflow, driver, skill, or agent names.

## Decisions

- `AgentBuilder::build_spec` is the new primary builder product.
- `AgentBuilder::build` delegates through `AgentSpec` to keep current behavior.
- `TracePolicy::Required` is the default for all `AgentSpec` values.
- `SdkValidationChain::default` implements the exact current `AgentConfig::validate` rules.
- `MacacaSdk` wraps an `AgentRegistryApi` trait; the first adapter targets `macaca_kernel::Kernel`.
- Deprecated registry helpers delegate to `MacacaSdk` after the facade exists.
```

- [ ] **Step 4: Create tasks**

Create `openspec/changes/refactor-macaca-sdk-patterns/tasks.md`:

```markdown
## 1. Preparation

- [ ] 1.1 Run GitNexus impact for `AgentBuilder`, `DeclarativeAgent`, `AgentPersona`, and `register_from_config`.
- [ ] 1.2 Run baseline `cargo test -p macaca-sdk -- --nocapture`.
- [ ] 1.3 Confirm current public exports and direct consumer grep.

## 2. AgentSpec builder product

- [ ] 2.1 Add `spec.rs` with `AgentSpec`, `AgentSpecBuilder`, and `TracePolicy`.
- [ ] 2.2 Add `AgentBuilder::build_spec`.
- [ ] 2.3 Make `AgentBuilder::build` delegate through `AgentSpec`.
- [ ] 2.4 Add spec parity tests for manifest, permission, capabilities, LLM options, prompt template, and trace policy.

## 3. Persona prototype

- [ ] 3.1 Add `persona_prototype.rs` with `PersonaPrototype` and `PersonaOverrides`.
- [ ] 3.2 Add clone/override tests proving the original persona is not mutated.
- [ ] 3.3 Export persona prototype primitives.

## 4. SDK validation chain

- [ ] 4.1 Add `validation.rs` with `SdkValidator` and `SdkValidationChain`.
- [ ] 4.2 Implement validators for current name, permission level, capability name, and temperature rules.
- [ ] 4.3 Route `AgentConfig::validate` through `SdkValidationChain::default`.
- [ ] 4.4 Add parity tests for all current validation success and failure cases.

## 5. SDK facade and registry adapter

- [ ] 5.1 Add `facade.rs` with `MacacaSdk`, `AgentRegistryApi`, and kernel adapter.
- [ ] 5.2 Make facade register `AgentSpec` while preserving `Kernel::register_agent` behavior.
- [ ] 5.3 Route `register_from_config` and `register_from_file` through `MacacaSdk`.
- [ ] 5.4 Mark replaced registry helper functions deprecated but keep callable.
- [ ] 5.5 Add facade registration tests.

## 6. Verification

- [ ] 6.1 Run `cargo fmt`.
- [ ] 6.2 Run `cargo test -p macaca-sdk -- --nocapture`.
- [ ] 6.3 Run `cargo test -p macaca-app -- --nocapture`.
- [ ] 6.4 Run `cargo test -p macaca-integration-tests kernel -- --nocapture`.
- [ ] 6.5 Run `cargo check -p macaca-sdk -p macaca-app -p macaca-web -p macaca-cli`.
- [ ] 6.6 Run `openspec validate refactor-macaca-sdk-patterns --strict`.
- [ ] 6.7 Run `gitnexus_detect_changes(scope: "all")`.
```

- [ ] **Step 5: Create delta spec**

Create `openspec/changes/refactor-macaca-sdk-patterns/specs/macaca-sdk-patterns/spec.md`:

```markdown
## ADDED Requirements

### Requirement: SDK builder produces AgentSpec

The SDK SHALL provide an `AgentSpec` builder product that captures declarative agent configuration without directly requiring runtime registration.

#### Scenario: Build spec from config

- **WHEN** an `AgentBuilder` builds an `AgentSpec` from a valid `AgentConfig`
- **THEN** the spec contains the same name, capabilities, permission, prompt template, LLM options, and trace policy metadata needed to build the current `DeclarativeAgent`.

### Requirement: Existing agent builder behavior remains compatible

Existing `AgentBuilder::build` and `AgentBuilder::build_with_manifest` behavior SHALL remain compatible.

#### Scenario: Build declarative agent through compatibility path

- **WHEN** existing code calls `AgentBuilder::from_config(config).build_with_manifest()`
- **THEN** it receives a `DeclarativeAgent` and `AgentManifest` with fields equivalent to the pre-refactor behavior.

### Requirement: Persona prototype supports clone and override

The SDK SHALL provide persona prototype primitives that instantiate modified personas without mutating the original prototype.

#### Scenario: Override identity

- **WHEN** a persona prototype with base identity is instantiated with an identity override
- **THEN** the returned persona contains the override
- **AND** the prototype's base persona remains unchanged.

### Requirement: SDK validation is chain-based

The SDK SHALL validate agent configs through a default validation chain equivalent to current validation behavior.

#### Scenario: Invalid permission level

- **WHEN** an agent config has an unsupported permission level
- **THEN** validation fails with a config error equivalent to current behavior.

### Requirement: SDK facade registers traceable specs

The SDK SHALL provide a facade that registers SDK-built agent specs through a registry adapter and requires trace policy metadata.

#### Scenario: Register config through SDK facade

- **WHEN** a valid `AgentConfig` is registered through `MacacaSdk`
- **THEN** the underlying registry receives a `DeclarativeAgent` and matching manifest
- **AND** the spec used for registration has trace policy metadata.

### Requirement: Legacy registry helpers remain callable

Legacy registry helper functions SHALL remain callable but deprecated after the facade exists.

#### Scenario: Existing helper remains compatible

- **WHEN** existing code calls `register_from_config`
- **THEN** registration succeeds with behavior equivalent to the pre-refactor implementation
- **AND** the function delegates to the new facade path.
```

## Task 2: AgentSpec Slice

**Files:**

- Create: `macaca/crates/macaca-sdk/src/spec.rs`
- Modify: `macaca/crates/macaca-sdk/src/builder.rs`
- Modify: `macaca/crates/macaca-sdk/src/lib.rs`

- [ ] **Step 1: Run impact before editing**

Run:

```text
gitnexus_impact({ target: "AgentBuilder", direction: "upstream", repo: "agent", includeTests: true })
gitnexus_impact({ target: "DeclarativeAgent", direction: "upstream", repo: "agent", includeTests: true })
```

Expected:

```text
Risk should be LOW or limited to sdk/integration tests. If higher, stop and report blast radius.
```

- [ ] **Step 2: Add failing tests for spec parity**

Add tests in `macaca/crates/macaca-sdk/src/spec.rs`:

```rust
#[test]
fn agent_spec_from_config_preserves_fields() {
    let config = crate::config::AgentConfig::from_yaml(
        r#"
name: spec-agent
capabilities:
  - name: writing
    description: Writes text
permission_level: system
allowed_tools:
  - file_read
prompt_template: "You are a writer."
model: deepseek-chat
max_tokens: 1024
temperature: 0.3
"#,
    )
    .unwrap();

    let spec = crate::AgentBuilder::from_config(config).build_spec().unwrap();
    assert_eq!(spec.name, "spec-agent");
    assert_eq!(spec.capabilities[0].name, "writing");
    assert_eq!(spec.permission.allowed_tools, vec!["file_read"]);
    assert_eq!(spec.llm_options.model, "deepseek-chat");
    assert_eq!(spec.prompt_template, "You are a writer.");
    assert!(matches!(spec.trace_policy, TracePolicy::Required));
}
```

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo test -p macaca-sdk agent_spec_from_config_preserves_fields -- --nocapture
```

Expected:

```text
FAIL because spec.rs / build_spec does not exist.
```

- [ ] **Step 3: Implement `spec.rs`**

Create:

```rust
//! Declarative SDK agent specification primitives.

use macaca_proto::{AgentId, AgentManifest, AgentState, Capability, LlmOptions, Permission};

use crate::builder::DeclarativeAgent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracePolicy {
    Required,
}

#[derive(Debug, Clone)]
pub struct AgentSpec {
    pub id: AgentId,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub permission: Permission,
    pub prompt_template: String,
    pub llm_options: LlmOptions,
    pub state: AgentState,
    pub trace_policy: TracePolicy,
}

impl AgentSpec {
    pub fn into_agent(self) -> DeclarativeAgent {
        DeclarativeAgent::from_spec(self)
    }

    pub fn manifest(&self) -> AgentManifest {
        AgentManifest {
            id: self.id,
            name: self.name.clone(),
            capabilities: self.capabilities.clone(),
            permission: self.permission.clone(),
            state: self.state,
            created_at: chrono::Utc::now(),
            model: self.llm_options.model.clone(),
        }
    }
}
```

- [ ] **Step 4: Wire builder through spec**

In `builder.rs`:

- Add `AgentBuilder::build_spec`.
- Add `DeclarativeAgent::from_spec`.
- Make `build` call `self.build_spec()?.into_agent()`.
- Make `manifest` delegate to an internal spec-equivalent field mapping or keep existing output exactly equivalent.

- [ ] **Step 5: Export new primitives**

In `lib.rs`:

```rust
pub mod spec;
pub use spec::{AgentSpec, TracePolicy};
```

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test -p macaca-sdk builder spec -- --nocapture
```

Expected:

```text
All sdk builder/spec tests pass.
```

## Task 3: Persona Prototype Slice

**Files:**

- Create: `macaca/crates/macaca-sdk/src/persona_prototype.rs`
- Modify: `macaca/crates/macaca-sdk/src/lib.rs`

- [ ] **Step 1: Run impact before editing**

Run:

```text
gitnexus_impact({ target: "AgentPersona", direction: "upstream", repo: "agent", includeTests: true })
```

- [ ] **Step 2: Add prototype tests**

Create tests in `persona_prototype.rs`:

```rust
#[test]
fn instantiate_with_overrides_does_not_mutate_base() {
    let base = crate::AgentPersona {
        identity: Some("base identity".into()),
        tools: Some("base tools".into()),
        ..Default::default()
    };
    let prototype = PersonaPrototype::new(base);

    let persona = prototype.instantiate(
        PersonaOverrides::default().with_identity("override identity"),
    );

    assert_eq!(persona.identity.as_deref(), Some("override identity"));
    assert_eq!(persona.tools.as_deref(), Some("base tools"));
    assert_eq!(prototype.base().identity.as_deref(), Some("base identity"));
}
```

- [ ] **Step 3: Implement prototype**

Create:

```rust
//! Persona prototype clone/override primitives.

use crate::AgentPersona;

#[derive(Debug, Clone)]
pub struct PersonaPrototype {
    base: AgentPersona,
}

#[derive(Debug, Clone, Default)]
pub struct PersonaOverrides {
    pub bootstrap: Option<String>,
    pub identity: Option<String>,
    pub agents: Option<String>,
    pub soul: Option<String>,
    pub user: Option<String>,
    pub tools: Option<String>,
    pub heartbeat: Option<String>,
}

impl PersonaPrototype {
    pub fn new(base: AgentPersona) -> Self {
        Self { base }
    }

    pub fn base(&self) -> &AgentPersona {
        &self.base
    }

    pub fn instantiate(&self, overrides: PersonaOverrides) -> AgentPersona {
        let mut persona = self.base.clone();
        if let Some(value) = overrides.bootstrap { persona.bootstrap = Some(value); }
        if let Some(value) = overrides.identity { persona.identity = Some(value); }
        if let Some(value) = overrides.agents { persona.agents = Some(value); }
        if let Some(value) = overrides.soul { persona.soul = Some(value); }
        if let Some(value) = overrides.user { persona.user = Some(value); }
        if let Some(value) = overrides.tools { persona.tools = Some(value); }
        if let Some(value) = overrides.heartbeat { persona.heartbeat = Some(value); }
        persona
    }
}

impl PersonaOverrides {
    pub fn with_identity(mut self, value: impl Into<String>) -> Self {
        self.identity = Some(value.into());
        self
    }
}
```

- [ ] **Step 4: Export prototype**

In `lib.rs`:

```rust
pub mod persona_prototype;
pub use persona_prototype::{PersonaOverrides, PersonaPrototype};
```

- [ ] **Step 5: Run tests**

Run:

```bash
cargo test -p macaca-sdk persona_prototype -- --nocapture
```

Expected:

```text
All persona prototype tests pass.
```

## Task 4: Validation Chain Slice

**Files:**

- Create: `macaca/crates/macaca-sdk/src/validation.rs`
- Modify: `macaca/crates/macaca-sdk/src/config.rs`
- Modify: `macaca/crates/macaca-sdk/src/lib.rs`

- [ ] **Step 1: Add validation parity tests**

Create tests in `validation.rs` covering:

- empty name fails
- invalid permission level fails
- empty capability name fails
- temperature outside `[0.0, 2.0]` fails
- valid minimal config passes

- [ ] **Step 2: Implement validation chain**

Create:

```rust
//! SDK config validation chain.

use macaca_proto::{MacacaError, MacacaResult};

use crate::AgentConfig;

pub trait SdkValidator: Send + Sync {
    fn validate(&self, config: &AgentConfig) -> MacacaResult<()>;
}

#[derive(Default)]
pub struct SdkValidationChain {
    validators: Vec<Box<dyn SdkValidator>>,
}

impl SdkValidationChain {
    pub fn with_default_validators() -> Self {
        let mut chain = Self { validators: Vec::new() };
        chain.push(NameValidator);
        chain.push(PermissionLevelValidator);
        chain.push(CapabilityNameValidator);
        chain.push(TemperatureValidator);
        chain
    }

    pub fn push<V: SdkValidator + 'static>(&mut self, validator: V) {
        self.validators.push(Box::new(validator));
    }

    pub fn validate(&self, config: &AgentConfig) -> MacacaResult<()> {
        for validator in &self.validators {
            validator.validate(config)?;
        }
        Ok(())
    }
}

pub struct NameValidator;
pub struct PermissionLevelValidator;
pub struct CapabilityNameValidator;
pub struct TemperatureValidator;
```

Implement each validator with exactly the current error messages from `AgentConfig::validate`.

- [ ] **Step 3: Route `AgentConfig::validate`**

Replace the body of `AgentConfig::validate` with:

```rust
crate::validation::SdkValidationChain::with_default_validators().validate(self)
```

- [ ] **Step 4: Export validation primitives**

In `lib.rs`:

```rust
pub mod validation;
pub use validation::{SdkValidationChain, SdkValidator};
```

- [ ] **Step 5: Run config and validation tests**

Run:

```bash
cargo test -p macaca-sdk config validation -- --nocapture
```

Expected:

```text
All current config tests and new validation tests pass.
```

## Task 5: SDK Facade and Registry Adapter Slice

**Files:**

- Create: `macaca/crates/macaca-sdk/src/facade.rs`
- Modify: `macaca/crates/macaca-sdk/src/registry_api.rs`
- Modify: `macaca/crates/macaca-sdk/src/lib.rs`

- [ ] **Step 1: Run impact before editing registry path**

Run:

```text
gitnexus_impact({ target: "register_from_config", direction: "upstream", repo: "agent", includeTests: true })
```

Expected:

```text
Risk is CRITICAL. Report that only facade delegation will change; registration semantics remain unchanged.
```

- [ ] **Step 2: Add facade tests**

In `facade.rs`, add a mock registry test:

```rust
#[tokio::test]
async fn sdk_facade_registers_config_with_trace_policy() {
    let registry = Arc::new(MockRegistry::default());
    let sdk = MacacaSdk::new(registry.clone());
    let config = AgentConfig::from_yaml(
        r#"
name: facade-agent
capabilities:
  - name: test
prompt_template: "Hello"
"#,
    )
    .unwrap();

    let id = sdk.register_config(config).await.unwrap();
    let stored = registry.last_manifest.read().await.clone().unwrap();
    assert_eq!(stored.id, id);
    assert_eq!(stored.name, "facade-agent");
}
```

- [ ] **Step 3: Implement facade and adapter**

Create:

```rust
//! SDK facade and registry adapters.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_agent::Agent;
use macaca_kernel::Kernel;
use macaca_proto::{AgentId, AgentManifest, MacacaResult};

use crate::{AgentBuilder, AgentConfig, AgentSpec};

#[async_trait]
pub trait AgentRegistryApi: Send + Sync {
    async fn register_agent(
        &self,
        agent: Box<dyn Agent>,
        manifest: AgentManifest,
    ) -> MacacaResult<AgentId>;
}

pub struct KernelAgentRegistry<'a> {
    kernel: &'a Kernel,
}

impl<'a> KernelAgentRegistry<'a> {
    pub fn new(kernel: &'a Kernel) -> Self {
        Self { kernel }
    }
}

#[async_trait]
impl AgentRegistryApi for KernelAgentRegistry<'_> {
    async fn register_agent(
        &self,
        agent: Box<dyn Agent>,
        manifest: AgentManifest,
    ) -> MacacaResult<AgentId> {
        self.kernel.register_agent(agent, manifest).await
    }
}

pub struct MacacaSdk<R> {
    registry: R,
}

impl<R> MacacaSdk<R> {
    pub fn new(registry: R) -> Self {
        Self { registry }
    }
}

impl<R> MacacaSdk<R>
where
    R: AgentRegistryApi,
{
    pub async fn register_config(&self, config: AgentConfig) -> MacacaResult<AgentId> {
        let spec = AgentBuilder::from_config(config).build_spec()?;
        self.register_spec(spec).await
    }

    pub async fn register_spec(&self, spec: AgentSpec) -> MacacaResult<AgentId> {
        let manifest = spec.manifest();
        let agent = spec.into_agent();
        self.registry.register_agent(Box::new(agent), manifest).await
    }
}
```

- [ ] **Step 4: Delegate legacy registry helpers**

In `registry_api.rs`, change:

```rust
pub async fn register_from_config(kernel: &Kernel, config: AgentConfig) -> MacacaResult<AgentId> {
    let registry = crate::facade::KernelAgentRegistry::new(kernel);
    crate::facade::MacacaSdk::new(registry).register_config(config).await
}
```

After all tests pass, mark both helper functions:

```rust
#[deprecated(note = "use MacacaSdk with KernelAgentRegistry or another AgentRegistryApi adapter")]
```

- [ ] **Step 5: Export facade**

In `lib.rs`:

```rust
pub mod facade;
pub use facade::{AgentRegistryApi, KernelAgentRegistry, MacacaSdk};
```

- [ ] **Step 6: Run facade and registry tests**

Run:

```bash
cargo test -p macaca-sdk facade registry_api -- --nocapture
```

Expected:

```text
All facade and legacy registry helper tests pass.
```

## Task 6: Final Verification

**Files:**

- No additional source files expected.

- [ ] **Step 1: Run formatting**

Run:

```bash
cd /Users/quantum/Code/dev/agent/macaca
cargo fmt
```

Expected:

```text
No formatting errors.
```

- [ ] **Step 2: Run SDK tests**

Run:

```bash
cargo test -p macaca-sdk -- --nocapture
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run app tests**

Run:

```bash
cargo test -p macaca-app -- --nocapture
```

Expected:

```text
test result: ok
```

- [ ] **Step 4: Run integration kernel tests**

Run:

```bash
cargo test -p macaca-integration-tests kernel -- --nocapture
```

Expected:

```text
No failures
```

- [ ] **Step 5: Run workspace check slice**

Run:

```bash
cargo check -p macaca-sdk -p macaca-app -p macaca-web -p macaca-cli
```

Expected:

```text
Finished `dev` profile
```

- [ ] **Step 6: Validate OpenSpec**

Run:

```bash
cd /Users/quantum/Code/dev/agent
openspec validate refactor-macaca-sdk-patterns --strict
```

Expected:

```text
Change 'refactor-macaca-sdk-patterns' is valid
```

- [ ] **Step 7: Run deprecated/API containment grep**

Run:

```bash
rg -n "register_from_config|register_from_file|AgentBuilder::build\\(" macaca/crates --glob '*.rs'
```

Expected:

```text
Legacy helpers remain callable. Production migrations are not required in this refactor proposal unless a deprecated warning blocks checks.
```

- [ ] **Step 8: Run GitNexus detect changes**

Run:

```text
gitnexus_detect_changes({ scope: "all", repo: "agent" })
```

Expected:

```text
Changed symbols are limited to macaca-sdk primitives, OpenSpec files, and any necessary compatibility tests.
```

## Self-Review

- Spec coverage: The plan covers `AgentSpec`, persona prototype, validation chain, SDK facade/registry adapter, trace policy, deprecated compatibility, and verification.
- Placeholder scan: No unresolved placeholders or unspecified implementation steps remain.
- Type consistency: The plan consistently uses `AgentSpec`, `TracePolicy`, `PersonaPrototype`, `PersonaOverrides`, `SdkValidationChain`, `MacacaSdk`, `AgentRegistryApi`, and `KernelAgentRegistry`.
- Scope check: The plan does not change app runtime behavior, kernel registration semantics, web/session/trace/EventLog/SSE, task loops, drivers, skills, MCP, or application-specific logic.
