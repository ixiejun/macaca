//! Facade bridge exposing provider crates to presentation shells during Route C migration.
//!
//! # Role in the architecture
//!
//! Presentation shells (`macaca-web`, `macaca-cli`) must not declare direct `Cargo.toml`
//! dependencies on replaceable capability crates (driver, llm, memory, persist, skill,
//! task, tools) or on composition-host crates (kernel, runtime-host). Those edges are
//! architectural debt tracked by the Route C dependency gate.
//!
//! This module implements the **Facade** pattern: shells depend only on `macaca-sdk`
//! and reach provider types through these re-exported crate aliases. The aliases do not
//! add semantics — they preserve type identity so bootstrap composition roots can still
//! wire concrete providers while the shell's direct dependency graph stays thin.
//!
//! # Migration trajectory
//!
//! Long term, shell code should consume **service clients** (`SystemLlmClient`,
//! `SystemTaskClient`, …) instead of provider construction types. This bridge is a
//! deliberate, auditable seam that shrinks the allowlist without forking types. Each
//! alias removal should coincide with the last shell consumer migrating to a typed
//! service client command.
//!
//! # Trace and audit
//!
//! Provider construction and lifecycle remain owned by `macaca-runtime-host` bootstrap
//! paths. Shells that import through this bridge must still route runtime behavior via
//! `SystemFacade::service` calls so service-call evidence remains the canonical audit chain.

/// Driver capability crate alias (browser/IDE/desktop automation providers).
pub use macaca_driver as driver;

/// LLM routing and provider abstraction crate alias.
pub use macaca_llm as llm;

/// Memory and recall backend crate alias.
pub use macaca_memory as memory;

/// Skill package runtime crate alias.
pub use macaca_skill as skill;

/// Task board, plan/worker loop, and todo persistence crate alias.
pub use macaca_task as task;

/// Builtin and orchestration tool catalog crate alias.
pub use macaca_tools as tools;

/// Microkernel primitives crate alias (identity, policy facade, service-call IPC).
///
/// Shells must not own kernel semantics — they only hold `Arc<Kernel>` handles produced
/// by the composition root and delegate execution through service clients.
pub use macaca_kernel as kernel;

/// Agent execution port and capability-set crate alias.
///
/// Framework construction adapters in presentation shells still need typed access to
/// `AgentServices`, `AgentCapabilitySet`, and transition reasons while the shell's
/// `Cargo.toml` converges on sdk-only workspace edges. This alias preserves type identity
/// without forcing a direct `macaca-agent` dependency on the shell crate.
pub use macaca_agent as agent;

/// Context assembly, reporting, and capability-catalog crate alias.
///
/// Shell adapters that build `ContextAssembleResult`, register context providers, or
/// surface context-service runtime capabilities still need the concrete context types
/// during Route C migration. The SDK already owns `macaca-context` as a transitive
/// dependency; this alias lets presentation shells drop the direct workspace edge while
/// preserving type identity for composition-root wiring and HTTP reporting routes.
pub use macaca_context as context;

/// Agent framework crate alias (ReAct agents, toolkits, session stores, construction).
///
/// Presentation shells still construct framework runners and tool adapters during Route C
/// migration. The SDK owns `macaca-framework` transitively (with `macaca-compat` and
/// `service-clients` features) so shells can drop the direct workspace edge. Framework
/// chat adapters consume [`macaca_llm::LlmServiceChatClient`] — not `SystemLlmClient` —
/// which breaks the historical `sdk ↔ framework` cyclic dependency.
pub use macaca_framework as framework;

/// Application model, loader, registry, and runtime bootstrap crate alias.
///
/// Presentation shells assemble application manifests and loaded-app state through
/// composition-root paths during Route C migration. The SDK owns `macaca-app` as a
/// transitive dependency once the `app → sdk` cycle is broken at the proto contract layer;
/// this alias lets `macaca-web` drop its direct workspace edge while preserving type
/// identity for HTTP routes, discovery, and bootstrap wiring.
pub use macaca_app as app;

/// Runtime-host composition root crate alias (bootstrap, persist, executor, MCP).
///
/// Shells still construct `ServiceRuntime`, `ApplicationExecutorRegistry`, and persist
/// backends at the composition root. Re-exporting through the SDK facade keeps the
/// shell `Cargo.toml` limited to `macaca-proto` + `macaca-sdk` without forking types.
pub use macaca_runtime_host as runtime_host;
