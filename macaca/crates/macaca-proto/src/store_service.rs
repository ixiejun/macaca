//! Provider-neutral Store Service DTOs for Route C S9.
//!
//! The Store Service owns package distribution metadata, install/status
//! orchestration, and sanitized snapshots.  It deliberately does not own
//! payment settlement, raw package transfer, application execution, or
//! entitlement policy internals.  Keeping this module in `macaca-proto` lets
//! SDK clients and runtime-host providers exchange commands without depending
//! on each other's concrete crates.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    CommerceMetadata, DeveloperId, KernelServiceId, PackageId, PackageRuntimeKind, TraceContext,
};

/// Stable Store Service id used by descriptors and SDK clients.
pub const STORE_SERVICE_ID: &str = "service.store";

/// Store Service command names.
pub const STORE_PACKAGE_INSPECT_COMMAND: &str = "store.package.inspect";
pub const STORE_PACKAGE_RESOLVE_COMMAND: &str = "store.package.resolve";
pub const STORE_PACKAGE_INSTALL_COMMAND: &str = "store.package.install";
pub const STORE_PACKAGE_STATUS_COMMAND: &str = "store.package.status";
pub const STORE_SNAPSHOT_COMMAND: &str = "store.snapshot";

/// Scope shared by Store Service package commands.
///
/// The scope contains only bounded identifiers and optional source references.
/// Raw package bytes, credentials, encrypted payloads, and provider-specific
/// handles are intentionally excluded so this DTO is safe for Web/CLI logs and
/// future remote service transport.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorePackageScope {
    pub package_id: Option<PackageId>,
    pub developer_id: Option<DeveloperId>,
    pub package_ref: Option<String>,
    pub source_id: Option<String>,
}

impl StorePackageScope {
    /// Build a scope for one package id.
    pub fn package(package_id: PackageId) -> Self {
        Self {
            package_id: Some(package_id),
            developer_id: None,
            package_ref: None,
            source_id: None,
        }
    }

    /// Return a safe display label for logs and diagnostics.
    pub fn label(&self) -> String {
        self.package_id
            .as_ref()
            .map(ToString::to_string)
            .or_else(|| self.package_ref.clone())
            .unwrap_or_else(|| "all".to_string())
    }
}

/// Inspect package metadata without mutating install state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePackageInspectCommand {
    pub trace: TraceContext,
    pub scope: StorePackageScope,
    pub include_installed: bool,
}

/// Resolve a package reference to sanitized package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePackageResolveCommand {
    pub trace: TraceContext,
    pub scope: StorePackageScope,
}

/// Request a metadata-level package install.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePackageInstallCommand {
    pub trace: TraceContext,
    pub scope: StorePackageScope,
    pub commerce: CommerceMetadata,
    pub runtime_kind: Option<PackageRuntimeKind>,
}

/// Query package installation/status state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePackageStatusCommand {
    pub trace: TraceContext,
    pub scope: StorePackageScope,
}

/// Query a sanitized Store Service snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSnapshotCommand {
    pub trace: TraceContext,
}

/// Sanitized package view safe for Web, CLI, SDK, and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorePackageView {
    pub package_id: Option<PackageId>,
    pub developer_id: Option<DeveloperId>,
    pub package_ref: Option<String>,
    pub version: Option<String>,
    pub runtime_kind: Option<PackageRuntimeKind>,
    pub commerce: CommerceMetadata,
    pub install_status: String,
    pub source_status: String,
    pub diagnostics: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl StorePackageView {
    /// Build a minimal unavailable view for one scope.
    pub fn unavailable(scope: &StorePackageScope, reason: impl Into<String>) -> Self {
        Self {
            package_id: scope.package_id.clone(),
            developer_id: scope.developer_id.clone(),
            package_ref: scope.package_ref.clone(),
            version: None,
            runtime_kind: None,
            commerce: CommerceMetadata::default(),
            install_status: "unavailable".into(),
            source_status: "unavailable".into(),
            diagnostics: vec![reason.into()],
            metadata: BTreeMap::new(),
        }
    }
}

/// Result returned by package install commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreInstallResult {
    pub package: StorePackageView,
    pub allowed: bool,
    pub status: String,
    pub reason: String,
    pub trace: TraceContext,
}

/// Structured unavailable state shared by Store Service commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreEntitlementUnavailable {
    pub service_id: KernelServiceId,
    pub command: String,
    pub reason: String,
    pub trace_id: Option<String>,
}

impl StoreEntitlementUnavailable {
    /// Build an unavailable DTO with optional trace correlation.
    pub fn new(
        service_id: impl Into<String>,
        command: impl Into<String>,
        reason: impl Into<String>,
        trace: Option<&TraceContext>,
    ) -> Self {
        Self {
            service_id: KernelServiceId::new(service_id),
            command: command.into(),
            reason: reason.into(),
            trace_id: trace.map(|ctx| ctx.trace_id.clone()),
        }
    }
}

/// Sanitized Store Service snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreServiceSnapshot {
    pub service_id: KernelServiceId,
    pub health: String,
    pub package_count: usize,
    pub source_count: usize,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub diagnostics: Vec<String>,
}

impl StoreServiceSnapshot {
    /// Build a healthy snapshot from counts only.
    pub fn healthy(package_count: usize, source_count: usize) -> Self {
        Self {
            service_id: KernelServiceId::new(STORE_SERVICE_ID),
            health: "healthy".into(),
            package_count,
            source_count,
            last_sync_at: Some(Utc::now()),
            diagnostics: Vec::new(),
        }
    }

    /// Build an unavailable snapshot that still has a stable shape.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: KernelServiceId::new(STORE_SERVICE_ID),
            health: "unavailable".into(),
            package_count: 0,
            source_count: 0,
            last_sync_at: None,
            diagnostics: vec![reason.into()],
        }
    }
}

pub type StorePackageInspectResult = Vec<StorePackageView>;
pub type StorePackageStatusResult = Vec<StorePackageView>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_package_view_round_trips() {
        let scope = StorePackageScope::package(PackageId::new("pkg.test"));
        let view = StorePackageView::unavailable(&scope, "missing source");
        let encoded = serde_json::to_string(&view).unwrap();
        let decoded: StorePackageView = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.install_status, "unavailable");
        assert_eq!(decoded.package_id.unwrap().as_str(), "pkg.test");
    }
}
