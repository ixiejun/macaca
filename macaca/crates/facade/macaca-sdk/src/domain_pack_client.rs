//! SDK-owned discovery Facade for developer/domain packs.
//!
//! The client is intentionally catalog-backed and provider-neutral.  It lets shells and
//! developer tooling list, inspect, resolve, and explain pack declarations without importing
//! optional package crates, runtime-host internals, or provider implementations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use macaca_proto::{
    expand_service_capabilities, AppServiceContractConfig, DomainPackDefinition,
    DomainPackUnavailableDiagnostic, EffectiveServiceCapabilities, MacacaError, MacacaResult,
    SharedDomainPackCatalog, TraceContext,
};

use crate::service_client::ServiceCallCommand;

/// Command used by SDK callers to list pack descriptors from an installed catalog.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackListCommand {
    /// Optional caller scope used only for audit-friendly logs.
    pub scope: String,
}

/// Command used by SDK callers to inspect one pack descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackInspectCommand {
    pub pack_id: String,
}

impl DomainPackInspectCommand {
    /// Build a validated inspect command so empty pack ids fail before catalog lookup.
    pub fn new(pack_id: impl Into<String>) -> MacacaResult<Self> {
        let pack_id = pack_id.into().trim().to_string();
        if pack_id.is_empty() {
            return Err(MacacaError::Config(
                "domain-pack inspect requires non-empty pack_id".into(),
            ));
        }
        Ok(Self { pack_id })
    }
}

/// Command used by SDK callers to resolve application pack declarations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainPackResolveCommand {
    pub declaration: AppServiceContractConfig,
}

/// SDK list result containing sanitized pack descriptors only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackListResult {
    pub packs: Vec<DomainPackDefinition>,
}

/// SDK inspect result. Missing packs are explicit instead of fake empty descriptors.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackInspectResult {
    pub pack: Option<DomainPackDefinition>,
    pub unavailable: Option<DomainPackUnavailableDiagnostic>,
}

/// SDK resolve result with an effective capability memento and unavailable explanations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackResolveResult {
    pub effective: EffectiveServiceCapabilities,
    pub unavailable: Vec<DomainPackUnavailableDiagnostic>,
}

impl DomainPackResolveResult {
    /// Build a canonical traced service command for a service expanded from the declaration.
    ///
    /// This helper is a Facade convenience, not a provider call.  It enforces that the service is
    /// present in the effective capability set, emits bounded audit logs, and returns the same
    /// `ServiceCallCommand` used by the canonical service runtime path.
    pub fn service_call_command(
        &self,
        service_id: impl Into<String>,
        command_name: impl Into<String>,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> MacacaResult<ServiceCallCommand> {
        let service_id = service_id.into();
        if !self.effective.services.contains(&service_id) {
            warn!(
                service_id = %service_id,
                capabilities_hash = %self.effective.capabilities_hash,
                "pack_service_call_failed"
            );
            return Err(MacacaError::Config(format!(
                "service '{service_id}' is not declared by the effective pack capability set"
            )));
        }
        let source = self
            .effective
            .service_sources
            .get(&service_id)
            .map(String::as_str)
            .unwrap_or("direct_service_declaration");
        info!(
            pack_id = source,
            service_id = %service_id,
            trace_id = %trace.trace_id,
            capabilities_hash = %self.effective.capabilities_hash,
            decision = "declared_capability_allowed",
            "pack_policy_decision"
        );
        info!(
            pack_id = source,
            service_id = %service_id,
            trace_id = %trace.trace_id,
            capabilities_hash = %self.effective.capabilities_hash,
            "pack_service_call_requested"
        );
        ServiceCallCommand::new(service_id, command_name, payload)
            .map(|command| command.with_trace(trace))
    }
}

/// Replaceable SDK discovery client for pack metadata and declaration resolution.
#[async_trait]
pub trait SystemDomainPackClient: Send + Sync {
    async fn list_packs(
        &self,
        command: &DomainPackListCommand,
    ) -> MacacaResult<DomainPackListResult>;

    async fn inspect_pack(
        &self,
        command: &DomainPackInspectCommand,
    ) -> MacacaResult<DomainPackInspectResult>;

    async fn resolve_declaration(
        &self,
        command: &DomainPackResolveCommand,
    ) -> MacacaResult<DomainPackResolveResult>;
}

/// Catalog-backed client used by shells and developer tooling.
#[derive(Clone)]
pub struct CatalogBackedDomainPackClient {
    catalog: SharedDomainPackCatalog,
}

impl CatalogBackedDomainPackClient {
    /// Inject the host-owned catalog Strategy. The SDK never constructs providers.
    pub fn new(catalog: SharedDomainPackCatalog) -> Self {
        Self { catalog }
    }
}

#[async_trait]
impl SystemDomainPackClient for CatalogBackedDomainPackClient {
    async fn list_packs(
        &self,
        command: &DomainPackListCommand,
    ) -> MacacaResult<DomainPackListResult> {
        let packs = self.catalog.list();
        info!(
            scope = %command.scope,
            pack_count = packs.len(),
            "pack_catalog_loaded"
        );
        Ok(DomainPackListResult { packs })
    }

    async fn inspect_pack(
        &self,
        command: &DomainPackInspectCommand,
    ) -> MacacaResult<DomainPackInspectResult> {
        let pack = self.catalog.resolve(&command.pack_id);
        let unavailable = pack.is_none().then(|| {
            DomainPackUnavailableDiagnostic::new(
                command.pack_id.clone(),
                false,
                "pack_not_installed",
                "pack descriptor is not installed in the active catalog",
            )
        });
        if unavailable.is_some() {
            warn!(
                pack_id = %command.pack_id,
                "pack_unavailable"
            );
        }
        Ok(DomainPackInspectResult { pack, unavailable })
    }

    async fn resolve_declaration(
        &self,
        command: &DomainPackResolveCommand,
    ) -> MacacaResult<DomainPackResolveResult> {
        let effective =
            expand_service_capabilities(Some(&command.declaration), self.catalog.as_ref());
        let unavailable = unavailable_diagnostics(&effective);
        info!(
            resolved_pack_count = effective.resolved_packs.len(),
            unavailable_pack_count = unavailable.len(),
            capabilities_hash = %effective.capabilities_hash,
            "pack_resolved"
        );
        Ok(DomainPackResolveResult {
            effective,
            unavailable,
        })
    }
}

/// Null Object client used when a shell has no pack catalog injected.
#[derive(Debug, Clone, Default)]
pub struct EmptySystemDomainPackClient;

#[async_trait]
impl SystemDomainPackClient for EmptySystemDomainPackClient {
    async fn list_packs(
        &self,
        command: &DomainPackListCommand,
    ) -> MacacaResult<DomainPackListResult> {
        info!(
            scope = %command.scope,
            "pack_catalog_loaded"
        );
        Ok(DomainPackListResult::default())
    }

    async fn inspect_pack(
        &self,
        command: &DomainPackInspectCommand,
    ) -> MacacaResult<DomainPackInspectResult> {
        warn!(
            pack_id = %command.pack_id,
            "pack_unavailable"
        );
        Ok(DomainPackInspectResult {
            pack: None,
            unavailable: Some(DomainPackUnavailableDiagnostic::new(
                command.pack_id.clone(),
                false,
                "pack_catalog_unavailable",
                "domain-pack catalog is not configured",
            )),
        })
    }

    async fn resolve_declaration(
        &self,
        command: &DomainPackResolveCommand,
    ) -> MacacaResult<DomainPackResolveResult> {
        let effective = expand_service_capabilities(
            Some(&command.declaration),
            macaca_proto::empty_domain_pack_catalog().as_ref(),
        );
        let unavailable = unavailable_diagnostics(&effective);
        Ok(DomainPackResolveResult {
            effective,
            unavailable,
        })
    }
}

fn unavailable_diagnostics(
    effective: &EffectiveServiceCapabilities,
) -> Vec<DomainPackUnavailableDiagnostic> {
    let required = effective.unresolved_required_packs.iter().map(|pack_id| {
        DomainPackUnavailableDiagnostic::new(
            pack_id.clone(),
            true,
            "required_pack_unresolved",
            "required pack is absent or unavailable",
        )
    });
    let optional = effective.unresolved_optional_packs.iter().map(|pack_id| {
        DomainPackUnavailableDiagnostic::new(
            pack_id.clone(),
            false,
            "optional_pack_unresolved",
            "optional pack is absent or unavailable",
        )
    });
    let incompatible = effective.incompatible_packs.iter().map(|pack_id| {
        DomainPackUnavailableDiagnostic::new(
            pack_id.clone(),
            true,
            "pack_incompatible",
            "pack descriptor failed executable specification validation",
        )
    });
    required.chain(optional).chain(incompatible).collect()
}

#[cfg(test)]
mod tests {
    use macaca_proto::{
        compose_installed_domain_pack_catalog, foundation_pack_definition,
        reference_domain_pack_definitions, TraceContext,
    };

    use super::*;

    #[tokio::test]
    async fn catalog_client_lists_reference_pack_metadata() {
        let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
        let client = CatalogBackedDomainPackClient::new(catalog);

        let result = client
            .list_packs(&DomainPackListCommand {
                scope: "sdk-test".into(),
            })
            .await
            .unwrap();

        assert!(result
            .packs
            .iter()
            .any(|pack| pack.pack_id == "pack.foundation.v1"));
    }

    #[tokio::test]
    async fn catalog_client_resolves_required_and_optional_unavailable() {
        let catalog = compose_installed_domain_pack_catalog([foundation_pack_definition()]);
        let client = CatalogBackedDomainPackClient::new(catalog);
        let declaration = AppServiceContractConfig {
            required_packs: vec!["pack.foundation.v1".into(), "pack.absent.v1".into()],
            optional_packs: vec!["pack.optional.v1".into()],
            ..Default::default()
        };

        let result = client
            .resolve_declaration(&DomainPackResolveCommand { declaration })
            .await
            .unwrap();

        assert!(result
            .effective
            .resolved_packs
            .contains(&"pack.foundation.v1".to_string()));
        assert!(result
            .unavailable
            .iter()
            .any(|diagnostic| { diagnostic.pack_id == "pack.absent.v1" && diagnostic.required }));
        assert!(result.unavailable.iter().any(|diagnostic| {
            diagnostic.pack_id == "pack.optional.v1" && !diagnostic.required
        }));
    }

    #[tokio::test]
    async fn resolved_pack_service_invocation_stays_trace_addressable() {
        let catalog = compose_installed_domain_pack_catalog([foundation_pack_definition()]);
        let client = CatalogBackedDomainPackClient::new(catalog);
        let declaration = AppServiceContractConfig {
            required_packs: vec!["pack.foundation.v1".into()],
            ..Default::default()
        };

        let result = client
            .resolve_declaration(&DomainPackResolveCommand { declaration })
            .await
            .unwrap();
        let service_id = result
            .effective
            .services
            .iter()
            .next()
            .expect("foundation pack exposes at least one service")
            .clone();
        let command = result
            .service_call_command(
                service_id.clone(),
                "service.snapshot",
                serde_json::json!({}),
                TraceContext::new("trace-pack-service-call"),
            )
            .unwrap();

        assert_eq!(command.service_id, service_id);
        assert_eq!(
            command.trace.as_ref().map(|trace| trace.trace_id.as_str()),
            Some("trace-pack-service-call")
        );
    }
}
