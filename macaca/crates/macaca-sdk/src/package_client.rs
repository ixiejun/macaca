//! SDK package client boundary for shell-facing package inspection.
//!
//! Package install/start/status will become Store/Application service behavior
//! in later Route C phases. S3 only defines an auditable command/client shape.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::MacacaResult;

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
