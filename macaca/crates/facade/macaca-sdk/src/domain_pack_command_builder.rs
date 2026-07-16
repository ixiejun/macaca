//! Descriptor-driven SDK command builders for domain packs.
//!
//! This module is the generic SDK-side **Facade + Command** boundary for pack
//! calls. It reads provider-neutral command schemas from a pack descriptor and
//! produces canonical traced `ServiceCallCommand` envelopes. It deliberately
//! does not import providers, open credentials, perform IO, or branch on any
//! application-specific workflow.

use macaca_proto::{DomainPackDefinition, MacacaError, MacacaResult, TraceContext};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

/// Provider-neutral descriptor for one SDK-buildable pack command.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DomainPackCommandSpec {
    pub pack_id: String,
    pub service_id: String,
    pub command_name: String,
}

/// Descriptor-derived command catalog used by generated or hand-written SDK helpers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackCommandCatalogBuilder {
    pack_id: String,
    commands: Vec<DomainPackCommandSpec>,
}

impl DomainPackCommandCatalogBuilder {
    /// Create a command catalog from a pack descriptor without making it callable.
    ///
    /// Preview-unavailable descriptors can still generate SDK helper metadata,
    /// but `build` later requires an effective capability projection where the
    /// pack is available. This split keeps docs/codegen useful while preserving
    /// runtime admission semantics.
    pub fn from_pack_definition(definition: &DomainPackDefinition) -> MacacaResult<Self> {
        if definition.pack_id.trim().is_empty() {
            return Err(MacacaError::Config(
                "domain-pack command catalog requires a non-empty pack_id".into(),
            ));
        }

        let mut commands = definition
            .metadata
            .service_command_schemas
            .iter()
            .flat_map(|(service_id, command_names)| {
                command_names
                    .iter()
                    .map(|command_name| DomainPackCommandSpec {
                        pack_id: definition.pack_id.clone(),
                        service_id: service_id.clone(),
                        command_name: command_name.clone(),
                    })
            })
            .collect::<Vec<_>>();
        commands.sort();

        if commands.is_empty() {
            return Err(MacacaError::Config(format!(
                "domain-pack '{}' does not declare SDK-buildable commands",
                definition.pack_id
            )));
        }

        info!(
            pack_id = %definition.pack_id,
            command_count = commands.len(),
            "domain_pack_command_catalog_built"
        );
        Ok(Self {
            pack_id: definition.pack_id.clone(),
            commands,
        })
    }

    /// Return the pack id that owns this command catalog.
    pub fn pack_id(&self) -> &str {
        &self.pack_id
    }

    /// Return descriptor-owned command specs in deterministic order.
    pub fn command_specs(&self) -> &[DomainPackCommandSpec] {
        &self.commands
    }

    /// Create a canonical command builder for one declared command name.
    ///
    /// The payload stays opaque at this layer because command DTO validation is
    /// owned by the pack contract and service provider. The SDK only proves that
    /// calls use declared service ids, declared command names, and trace context.
    pub fn command_builder(
        &self,
        command_name: impl Into<String>,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> MacacaResult<DomainPackDeclaredCommandBuilder> {
        let command_name = command_name.into().trim().to_string();
        let matches = self
            .commands
            .iter()
            .filter(|spec| spec.command_name == command_name)
            .cloned()
            .collect::<Vec<_>>();

        match matches.as_slice() {
            [spec] => Ok(DomainPackDeclaredCommandBuilder {
                spec: spec.clone(),
                payload,
                trace,
            }),
            [] => {
                warn!(
                    pack_id = %self.pack_id,
                    command = %command_name,
                    "domain_pack_command_builder_rejected"
                );
                Err(MacacaError::Config(format!(
                    "command '{command_name}' is not declared by pack '{}'",
                    self.pack_id
                )))
            }
            _ => Err(MacacaError::Config(format!(
                "command '{command_name}' is ambiguous across services in pack '{}'",
                self.pack_id
            ))),
        }
    }
}

/// One pending service-call command produced from descriptor metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainPackDeclaredCommandBuilder {
    pub spec: DomainPackCommandSpec,
    payload: serde_json::Value,
    trace: TraceContext,
}

impl DomainPackDeclaredCommandBuilder {
    /// Build the canonical traced service call after effective capability admission.
    ///
    /// This method delegates to the existing generic service-call builder so all
    /// pack helpers share one SDK execution path and cannot bypass service
    /// runtime trace, policy, resource, and audit decorators.
    pub fn build(self, resolved: &DomainPackResolveResult) -> MacacaResult<ServiceCallCommand> {
        info!(
            pack_id = %self.spec.pack_id,
            service_id = %self.spec.service_id,
            command = %self.spec.command_name,
            trace_id = %self.trace.trace_id,
            "domain_pack_command_builder_ready"
        );
        DomainPackServiceCallBuilder::new(
            self.spec.service_id,
            self.spec.command_name,
            self.payload,
            self.trace,
        )?
        .build(resolved)
    }
}

#[cfg(test)]
#[path = "domain_pack_command_builder_tests.rs"]
mod tests;
