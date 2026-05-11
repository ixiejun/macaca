//! Concrete plugin host skeletons.
//!
//! The host implementations are intentionally small Strategy objects. They
//! encode the safety behavior for each runtime kind while sharing the same
//! `PluginHostRuntime` trait, so future real WASM/process/remote execution can
//! replace only one strategy without changing callers or SDK contracts.

use async_trait::async_trait;
use macaca_proto::{
    PluginHealth, PluginHostCommand, PluginHostHealthSnapshot, PluginHostOperation,
    PluginHostResult, PluginHostStatus, PluginRuntimeKind,
};
use tracing::{info, warn};

/// Result alias used by plugin host strategies.
pub type PluginHostRuntimeResult<T> = Result<T, macaca_proto::PluginError>;

/// Stable host strategy interface selected by the Abstract Factory.
#[async_trait]
pub trait PluginHostRuntime: Send + Sync {
    /// Runtime kind handled by this host strategy.
    fn runtime_kind(&self) -> PluginRuntimeKind;

    /// Execute a bounded host command.
    async fn execute(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostResult>;

    /// Probe host health without executing untrusted plugin code.
    async fn health(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostHealthSnapshot>;
}

/// Descriptor-only host that accepts metadata lifecycle commands.
#[derive(Debug, Default)]
pub struct DescriptorPluginRuntimeHost;

#[async_trait]
impl PluginHostRuntime for DescriptorPluginRuntimeHost {
    fn runtime_kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::DescriptorOnly
    }

    async fn execute(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostResult> {
        info!(
            plugin_id = %command.descriptor.plugin_id,
            runtime_kind = %command.descriptor.runtime_kind,
            operation = %command.operation,
            trace_id = %command.trace.trace_id,
            "descriptor plugin host accepted metadata-only command"
        );
        let mut result = PluginHostResult::accepted(
            &command,
            "descriptor host accepted command without executing plugin code",
        );
        result.output = serde_json::json!({ "metadata_only": true });
        Ok(result)
    }

    async fn health(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostHealthSnapshot> {
        host_health(command, PluginHealth::Healthy)
    }
}

/// Built-in adapter host for already-compiled OS services.
#[derive(Debug, Default)]
pub struct BuiltInAdapterPluginRuntimeHost;

#[async_trait]
impl PluginHostRuntime for BuiltInAdapterPluginRuntimeHost {
    fn runtime_kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::BuiltInAdapter
    }

    async fn execute(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostResult> {
        info!(
            plugin_id = %command.descriptor.plugin_id,
            operation = %command.operation,
            trace_id = %command.trace.trace_id,
            "built-in adapter plugin host accepted canonical adapter command"
        );
        let mut result = PluginHostResult::accepted(
            &command,
            "built-in adapter host accepted command through canonical API",
        );
        result
            .metadata
            .insert("adapter_kind".into(), "built_in".into());
        Ok(result)
    }

    async fn health(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostHealthSnapshot> {
        host_health(command, PluginHealth::Healthy)
    }
}

/// Unavailable-safe WASM host skeleton.
#[derive(Debug, Default)]
pub struct WasmPluginRuntimeHost;

/// Unavailable-safe process host skeleton.
#[derive(Debug, Default)]
pub struct ProcessPluginRuntimeHost;

/// Unavailable-safe remote proxy host skeleton.
#[derive(Debug, Default)]
pub struct RemoteProxyPluginRuntimeHost;

/// Generic unavailable host used for native/custom runtimes not implemented in v1.
#[derive(Debug, Clone)]
pub struct UnavailablePluginRuntimeHost {
    runtime_kind: PluginRuntimeKind,
    reason: String,
}

impl UnavailablePluginRuntimeHost {
    /// Create an unavailable host for a runtime kind and diagnostic reason.
    pub fn new(runtime_kind: PluginRuntimeKind, reason: impl Into<String>) -> Self {
        Self {
            runtime_kind,
            reason: reason.into(),
        }
    }
}

macro_rules! unavailable_host {
    ($ty:ty, $kind:expr, $reason:expr) => {
        #[async_trait]
        impl PluginHostRuntime for $ty {
            fn runtime_kind(&self) -> PluginRuntimeKind {
                $kind
            }

            async fn execute(
                &self,
                command: PluginHostCommand,
            ) -> PluginHostRuntimeResult<PluginHostResult> {
                unavailable_result(command, $reason)
            }

            async fn health(
                &self,
                command: PluginHostCommand,
            ) -> PluginHostRuntimeResult<PluginHostHealthSnapshot> {
                host_health(
                    command,
                    PluginHealth::Unavailable {
                        reason: $reason.into(),
                    },
                )
            }
        }
    };
}

unavailable_host!(
    WasmPluginRuntimeHost,
    PluginRuntimeKind::Wasm,
    "WASM plugin execution is not enabled in host skeleton v1"
);
unavailable_host!(
    ProcessPluginRuntimeHost,
    PluginRuntimeKind::Process,
    "process plugin execution is not enabled in host skeleton v1"
);
unavailable_host!(
    RemoteProxyPluginRuntimeHost,
    PluginRuntimeKind::RemoteProxy,
    "remote proxy plugin execution is not enabled in host skeleton v1"
);

#[async_trait]
impl PluginHostRuntime for UnavailablePluginRuntimeHost {
    fn runtime_kind(&self) -> PluginRuntimeKind {
        self.runtime_kind.clone()
    }

    async fn execute(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostResult> {
        unavailable_result(command, &self.reason)
    }

    async fn health(
        &self,
        command: PluginHostCommand,
    ) -> PluginHostRuntimeResult<PluginHostHealthSnapshot> {
        host_health(
            command,
            PluginHealth::Unavailable {
                reason: self.reason.clone(),
            },
        )
    }
}

fn unavailable_result(
    command: PluginHostCommand,
    reason: &str,
) -> PluginHostRuntimeResult<PluginHostResult> {
    warn!(
        plugin_id = %command.descriptor.plugin_id,
        runtime_kind = %command.descriptor.runtime_kind,
        operation = %command.operation,
        trace_id = %command.trace.trace_id,
        reason,
        "plugin host skeleton returned structured unavailable result"
    );
    let mut result = PluginHostResult::unavailable(&command, reason);
    result.status = match command.operation {
        PluginHostOperation::HealthProbe => PluginHostStatus::Unavailable,
        _ => PluginHostStatus::Unavailable,
    };
    Ok(result)
}

fn host_health(
    command: PluginHostCommand,
    health: PluginHealth,
) -> PluginHostRuntimeResult<PluginHostHealthSnapshot> {
    info!(
        plugin_id = %command.descriptor.plugin_id,
        runtime_kind = %command.descriptor.runtime_kind,
        trace_id = %command.trace.trace_id,
        "plugin host health probe completed"
    );
    Ok(PluginHostHealthSnapshot {
        plugin_id: command.descriptor.plugin_id,
        runtime_kind: command.descriptor.runtime_kind,
        health,
        checked_at: chrono::Utc::now(),
        trace: command.trace,
        metadata: Default::default(),
    })
}
