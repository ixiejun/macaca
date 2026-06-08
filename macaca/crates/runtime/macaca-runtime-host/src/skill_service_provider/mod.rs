//! Runtime-host adapter for the Route C Skill Service (Facade module tree).
//!
//! The provider bridges typed Skill service commands to existing skill runtime
//! facades.  It keeps Web and CLI from constructing skill runtimes directly
//! while preserving the old semantics through deprecated compatibility anchors.
//!
//! Module layout (Facade + Command dispatch):
//! - `system_service.rs` — `SystemService` lifecycle hooks and command admission logging
//! - `command_dispatch.rs` — Command Router match table delegating to focused handlers
//! - `runtime_facade_commands.rs` — snapshot, executable load, tool catalog/invoke, status
//! - `operator_commands.rs` — Skill operator lifecycle commands
//! - `governance_alias_commands.rs` — governance read model and alias routing
//! - `evolution_commands.rs` — experience proposals, draft promotion/rejection
//! - `evaluation_content_commands.rs` — self-evolution evaluation and content mutation
//! - `delegated_commands.rs` — curation, lifecycle, and materialization Strategy bridges

mod command_dispatch;
mod delegated_commands;
mod evaluation_content_commands;
mod evolution_commands;
mod governance_alias_commands;
mod operator_commands;
mod runtime_facade_commands;
mod system_service;

use std::path::PathBuf;
use std::sync::Arc;

use macaca_memory::MemoryRuntimeFacade;
use macaca_proto::{ServiceCommand, ServiceDescriptor, ServiceError, ServiceResult, TraceContext};
use macaca_skill::{skill_service_descriptor, ExecutableSkillToolSet, SkillRuntimeFacade};
use tokio::sync::Mutex;

use crate::skill_operator_lifecycle::SkillOperatorLifecycle;
use crate::skill_service_content_mutation::LocalSkillContentMutationStrategy;
use crate::skill_service_experience_routing::SkillExperienceDestinationRouter;
use crate::skill_service_provider_state::SkillProviderGovernanceState;

/// Host-owned Skill service provider backed by skill facades.
pub struct SkillSystemServiceProvider {
    pub(crate) descriptor: ServiceDescriptor,
    pub(crate) snapshot_facade: Option<SkillRuntimeFacade>,
    pub(crate) executable_tools: Arc<Mutex<ExecutableSkillToolSet>>,
    pub(crate) governance_state: Arc<SkillProviderGovernanceState>,
    pub(crate) experience_router: SkillExperienceDestinationRouter,
    pub(crate) content_mutation: LocalSkillContentMutationStrategy,
    pub(crate) operator_lifecycle: SkillOperatorLifecycle,
    pub(crate) materialized_skill_roots: Vec<PathBuf>,
    pub(crate) governance_event_journal_path: Option<PathBuf>,
}

impl SkillSystemServiceProvider {
    /// Create a provider with the default skill runtime facades.
    pub fn new() -> Self {
        Self {
            descriptor: skill_service_descriptor(),
            snapshot_facade: Some(SkillRuntimeFacade::new()),
            executable_tools: Arc::new(Mutex::new(ExecutableSkillToolSet::new())),
            governance_state: Arc::new(SkillProviderGovernanceState::default()),
            experience_router: SkillExperienceDestinationRouter::default(),
            content_mutation: LocalSkillContentMutationStrategy,
            operator_lifecycle: SkillOperatorLifecycle::new(),
            materialized_skill_roots: Vec::new(),
            governance_event_journal_path: None,
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: skill_service_descriptor(),
            snapshot_facade: None,
            executable_tools: Arc::new(Mutex::new(ExecutableSkillToolSet::new())),
            governance_state: Arc::new(SkillProviderGovernanceState::default()),
            experience_router: SkillExperienceDestinationRouter::default(),
            content_mutation: LocalSkillContentMutationStrategy,
            operator_lifecycle: SkillOperatorLifecycle::new(),
            materialized_skill_roots: Vec::new(),
            governance_event_journal_path: None,
        }
    }

    /// Configure generic Skill package roots used for restart recovery.
    ///
    /// The roots are source directories such as a workspace or application
    /// `skills/` directory.  The provider does not infer application behavior
    /// from the path; it only scans direct child packages for bounded
    /// materialization provenance so active governance identities can survive a
    /// process restart.
    pub fn with_materialized_skill_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.materialized_skill_roots = roots;
        self
    }

    /// Configure the provider-local durable governance event journal.
    ///
    /// The journal is a local Memento for sanitized Skill governance events. It
    /// is intentionally optional so tests, unavailable providers, and future
    /// Store/EventLog-backed providers can use the same service contract without
    /// depending on a filesystem path.
    pub fn with_governance_event_journal_path(mut self, path: PathBuf) -> Self {
        self.governance_event_journal_path = Some(path);
        self
    }

    /// Attach the Memory runtime facade used for MemoryFact and KnowledgeDigest
    /// destinations.
    ///
    /// Skill Evolution owns classification and governance proposal creation,
    /// while Memory owns fact persistence and knowledge compilation.  Injecting
    /// this facade keeps the runtime-host composition root responsible for
    /// wiring replaceable providers and prevents Skill service code from
    /// constructing memory stores directly.
    pub fn with_memory_runtime(mut self, memory_runtime: Arc<dyn MemoryRuntimeFacade>) -> Self {
        self.experience_router =
            SkillExperienceDestinationRouter::with_memory_runtime(memory_runtime);
        self
    }

    /// Resolve the optional snapshot facade or return a structured unavailable error.
    pub(crate) fn facade(&self) -> ServiceResult<SkillRuntimeFacade> {
        self.snapshot_facade.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("skill runtime is not configured".into())
        })
    }

    /// Extract the mandatory trace context from an inbound service command.
    pub(crate) fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }
}

impl Default for SkillSystemServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}
