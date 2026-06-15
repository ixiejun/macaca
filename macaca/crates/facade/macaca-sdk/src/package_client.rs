//! SDK package client boundary for shell-facing package inspection.
//!
//! This package client is a thin Store-service adapter.  The command/result
//! shape remains stable for existing Web/CLI callers while service-backed implementations delegate to
//! `SystemStoreClient` instead of reading package internals directly.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::{MacacaResult, StorePackageInspectCommand, StorePackageScope, TraceContext};

use crate::store_client::SystemStoreClient;

/// Read-only package inspection command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInspectionCommand {
    pub package_ref: Option<String>,
}

impl PackageInspectionCommand {
    /// Inspect all packages.
    pub fn all() -> Self {
        Self { package_ref: None }
    }

    /// Inspect one package reference after trimming empty input.
    pub fn one(package_ref: impl Into<String>) -> Self {
        let package_ref = package_ref.into().trim().to_string();
        Self {
            package_ref: if package_ref.is_empty() {
                None
            } else {
                Some(package_ref)
            },
        }
    }
}

/// Minimal package inspection result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInspectionResult {
    pub package_ref: Option<String>,
    pub packages: Vec<String>,
}

/// Replaceable package inspection client.
#[async_trait]
pub trait SystemPackageClient: Send + Sync {
    /// Inspect package metadata without starting or installing packages.
    async fn inspect_packages(
        &self,
        command: &PackageInspectionCommand,
    ) -> MacacaResult<PackageInspectionResult>;
}

/// Empty local package client used until Store/Application services expose data.
#[derive(Debug, Default, Clone)]
pub struct EmptySystemPackageClient;

#[async_trait]
impl SystemPackageClient for EmptySystemPackageClient {
    async fn inspect_packages(
        &self,
        command: &PackageInspectionCommand,
    ) -> MacacaResult<PackageInspectionResult> {
        info!(
            package_ref = ?command.package_ref,
            "sdk package client returning empty package inspection result"
        );
        Ok(PackageInspectionResult {
            package_ref: command.package_ref.clone(),
            packages: Vec::new(),
        })
    }
}

/// Package client backed by the focused Store Service client.
///
/// This Adapter preserves the stable inspection API while moving real package
/// metadata reads behind Store Service.  It intentionally returns only package
/// display strings because the shell-facing result type is intentionally minimal; callers that
/// need full Store metadata should use `SystemStoreClient` directly.
pub struct StoreBackedSystemPackageClient<S> {
    store: S,
}

impl<S> StoreBackedSystemPackageClient<S> {
    /// Create a package client from any Store client Strategy.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S> SystemPackageClient for StoreBackedSystemPackageClient<S>
where
    S: SystemStoreClient,
{
    async fn inspect_packages(
        &self,
        command: &PackageInspectionCommand,
    ) -> MacacaResult<PackageInspectionResult> {
        let scope = StorePackageScope {
            package_id: None,
            developer_id: None,
            package_ref: command.package_ref.clone(),
            source_id: None,
        };
        let result = self
            .store
            .inspect(StorePackageInspectCommand {
                trace: TraceContext::new("sdk-package-inspection"),
                scope,
                include_installed: true,
            })
            .await?;
        Ok(PackageInspectionResult {
            package_ref: command.package_ref.clone(),
            packages: result
                .into_iter()
                .map(|view| {
                    view.package_id
                        .map(|id| id.to_string())
                        .or(view.package_ref)
                        .unwrap_or_else(|| "unknown".into())
                })
                .collect(),
        })
    }
}
