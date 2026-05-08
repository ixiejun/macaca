//! `aos-sdk` — declarative agent SDK for Agent OS.
//!
//! Provides YAML/TOML configuration parsing, a fluent builder API,
//! and registration helpers to construct and register agents with
//! the kernel from declarative config files.

pub mod application;
pub mod builder;
pub mod config;
pub mod evm;
pub mod facade;
pub mod persona;
pub mod persona_prototype;
pub mod registry_api;
pub mod spec;
pub mod system_facade;
pub mod validation;

pub use application::{
    service_call_command, trace_emit_command, ApplicationAbiBuilder, ApplicationHostCommandBuilder,
};
pub use builder::{AgentBuilder, DeclarativeAgent};
pub use config::AgentConfig;
pub use evm::MacacaEvmSdk;
pub use facade::{AgentRegistryApi, KernelAgentRegistry, KernelPrimitiveSdk, MacacaSdk};
pub use persona::AgentPersona;
pub use persona_prototype::{PersonaOverrides, PersonaPrototype};
#[allow(deprecated)]
pub use registry_api::{register_from_config, register_from_file};
pub use spec::{AgentSpec, AgentSpecBuilder, TracePolicy};
pub use system_facade::{
    kernel_status_snapshot, ApprovalDecisionCommand, PackageInspectionCommand,
    ServiceInspectionCommand, SessionEventQueryCommand, StaticSystemStatusDataSource, SystemFacade,
    SystemStatusDataSource, SystemStatusSnapshot, TaskBoardDataSource, TaskBoardQueryCommand,
    TaskBoardQueryResult, TodoStoreTaskBoardDataSource, TraceTailCommand,
};
pub use validation::{SdkValidationChain, SdkValidator};
