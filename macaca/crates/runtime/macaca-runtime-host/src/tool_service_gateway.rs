//! Managed gateway provider seam for gateway-backed tool capabilities.
//!
//! Managed gateways are optional providers behind `service.tool`; they are not a
//! second control plane.  This module keeps routing metadata, metering, audit,
//! and health as provider-neutral data so gateway vendors remain replaceable.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    CapabilityToolInvocation, CapabilityToolInvocationResult, MacacaResult, ServiceHealth,
    ToolAuditRef, ToolFamilyRef, ToolGatewayAuditEvent, ToolGatewayHealthResult,
    ToolGatewayMeteringEvent, ToolManagedGatewayDescriptor, ToolManagedGatewayRouteKind,
    TraceContext,
};

/// Adapter contract implemented by managed gateway providers.
#[async_trait]
pub trait ToolManagedGatewayProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    async fn descriptors(&self) -> MacacaResult<Vec<ToolManagedGatewayDescriptor>>;
    async fn invoke(
        &self,
        invocation: ToolManagedGatewayInvocation,
    ) -> MacacaResult<CapabilityToolInvocationResult>;
    async fn record_metering(&self, event: ToolGatewayMeteringEvent) -> MacacaResult<ToolAuditRef>;
    async fn record_audit(&self, event: ToolGatewayAuditEvent) -> MacacaResult<ToolAuditRef>;
}

/// Provider-neutral invocation envelope for gateway-backed routes.
///
/// Managed gateways often front browsers, web APIs, document services, and
/// enterprise connectors.  The raw input remains inside the invocation object
/// for provider execution, while logs and audit hooks consume hashes,
/// provider ids, route ids, and artifact refs only.
#[derive(Debug, Clone)]
pub struct ToolManagedGatewayInvocation {
    pub invocation: CapabilityToolInvocation,
    pub gateway_id: String,
    pub provider_id: String,
    pub family: ToolFamilyRef,
    pub tool_id: String,
    pub input_hash: String,
}

/// Null Object gateway provider for absent optional gateway modules.
pub struct UnavailableToolManagedGatewayProvider {
    provider_id: String,
    gateway_id: String,
    route_kind: ToolManagedGatewayRouteKind,
    family: ToolFamilyRef,
    reason: String,
}

impl UnavailableToolManagedGatewayProvider {
    pub fn new(
        provider_id: impl Into<String>,
        gateway_id: impl Into<String>,
        route_kind: ToolManagedGatewayRouteKind,
        family: ToolFamilyRef,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            gateway_id: gateway_id.into(),
            route_kind,
            family,
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ToolManagedGatewayProvider for UnavailableToolManagedGatewayProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn descriptors(&self) -> MacacaResult<Vec<ToolManagedGatewayDescriptor>> {
        tracing::warn!(
            provider_id = %self.provider_id,
            gateway_id = %self.gateway_id,
            reason_code = "managed_gateway_provider_unavailable",
            "tool managed gateway provider unavailable"
        );
        Ok(vec![ToolManagedGatewayDescriptor::unavailable(
            self.gateway_id.clone(),
            self.provider_id.clone(),
            self.route_kind.clone(),
            self.family.clone(),
            self.reason.clone(),
        )])
    }

    async fn invoke(
        &self,
        invocation: ToolManagedGatewayInvocation,
    ) -> MacacaResult<CapabilityToolInvocationResult> {
        tracing::warn!(
            trace_id = %invocation.invocation.trace.trace_id,
            provider_id = %self.provider_id,
            gateway_id = %self.gateway_id,
            tool_id = %invocation.tool_id,
            reason_code = "managed_gateway_provider_unavailable",
            "tool managed gateway invocation stopped because provider is unavailable"
        );
        Ok(CapabilityToolInvocationResult::failed(
            "service.tool.managed_gateway",
            macaca_proto::CapabilityToolOriginKind::Mcp,
            invocation.invocation.tool_name,
            "managed gateway provider is unavailable",
            invocation.invocation.trace,
        ))
    }

    async fn record_metering(&self, event: ToolGatewayMeteringEvent) -> MacacaResult<ToolAuditRef> {
        tracing::warn!(
            trace_id = %event.trace.trace_id,
            provider_id = %self.provider_id,
            gateway_id = %event.gateway_id,
            tool_id = %event.tool_id,
            metering_ref = %event.metering_ref,
            reason_code = "managed_gateway_provider_unavailable",
            "tool managed gateway metering recorded as unavailable"
        );
        Ok(ToolAuditRef::new("tool.gateway.meter.unavailable"))
    }

    async fn record_audit(&self, event: ToolGatewayAuditEvent) -> MacacaResult<ToolAuditRef> {
        tracing::warn!(
            trace_id = %event.trace.trace_id,
            provider_id = %self.provider_id,
            gateway_id = %event.gateway_id,
            tool_id = %event.tool_id,
            status = %event.status,
            reason_code = "managed_gateway_provider_unavailable",
            "tool managed gateway audit recorded as unavailable"
        );
        Ok(ToolAuditRef::new("tool.gateway.audit.unavailable"))
    }
}

/// Descriptor-backed gateway provider useful for built-in, remote, and test
/// adapters whose concrete routing is configured outside OS control flow.
pub struct StaticToolManagedGatewayProvider {
    provider_id: String,
    descriptors: Vec<ToolManagedGatewayDescriptor>,
    metering_events: Mutex<Vec<ToolGatewayMeteringEvent>>,
    audit_events: Mutex<Vec<ToolGatewayAuditEvent>>,
}

impl StaticToolManagedGatewayProvider {
    pub fn new(
        provider_id: impl Into<String>,
        descriptors: Vec<ToolManagedGatewayDescriptor>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            descriptors,
            metering_events: Mutex::new(Vec::new()),
            audit_events: Mutex::new(Vec::new()),
        }
    }

    pub fn recorded_metering_count(&self) -> usize {
        self.metering_events
            .lock()
            .map(|items| items.len())
            .unwrap_or(0)
    }

    pub fn recorded_audit_count(&self) -> usize {
        self.audit_events
            .lock()
            .map(|items| items.len())
            .unwrap_or(0)
    }
}

#[async_trait]
impl ToolManagedGatewayProvider for StaticToolManagedGatewayProvider {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    async fn descriptors(&self) -> MacacaResult<Vec<ToolManagedGatewayDescriptor>> {
        Ok(self.descriptors.clone())
    }

    async fn invoke(
        &self,
        invocation: ToolManagedGatewayInvocation,
    ) -> MacacaResult<CapabilityToolInvocationResult> {
        let gateway = self
            .descriptors
            .iter()
            .find(|descriptor| descriptor.gateway_id == invocation.gateway_id)
            .cloned();
        let Some(gateway) = gateway else {
            tracing::warn!(
                trace_id = %invocation.invocation.trace.trace_id,
                provider_id = %self.provider_id,
                gateway_id = %invocation.gateway_id,
                tool_id = %invocation.tool_id,
                reason_code = "managed_gateway_route_not_found",
                "tool managed gateway route was not registered on provider"
            );
            return Ok(CapabilityToolInvocationResult::failed(
                "service.tool.managed_gateway",
                macaca_proto::CapabilityToolOriginKind::Mcp,
                invocation.invocation.tool_name,
                "managed gateway route is not registered",
                invocation.invocation.trace,
            ));
        };
        if !matches!(gateway.health, ServiceHealth::Healthy) {
            return Ok(CapabilityToolInvocationResult::failed(
                "service.tool.managed_gateway",
                macaca_proto::CapabilityToolOriginKind::Mcp,
                invocation.invocation.tool_name,
                "managed gateway provider is not healthy",
                invocation.invocation.trace,
            ));
        }

        let metering_ref = format!("meter.{}", invocation.tool_id);
        if gateway.metering_required {
            self.record_metering(ToolGatewayMeteringEvent {
                trace: invocation.invocation.trace.clone(),
                gateway_id: invocation.gateway_id.clone(),
                provider_id: invocation.provider_id.clone(),
                tool_id: invocation.tool_id.clone(),
                metering_ref: metering_ref.clone(),
                units: 1,
                metadata: BTreeMap::new(),
            })
            .await?;
        }
        if gateway.audit_required {
            self.record_audit(ToolGatewayAuditEvent {
                trace: invocation.invocation.trace.clone(),
                gateway_id: invocation.gateway_id.clone(),
                provider_id: invocation.provider_id.clone(),
                tool_id: invocation.tool_id.clone(),
                status: "ok".into(),
                latency_millis: 0,
                input_hash: invocation.input_hash.clone(),
                output_hash: format!("sha256:{}", invocation.input_hash),
                artifact_refs: Vec::new(),
                metering_ref: Some(metering_ref.clone()),
                metadata: BTreeMap::new(),
            })
            .await?;
        }
        tracing::info!(
            trace_id = %invocation.invocation.trace.trace_id,
            provider_id = %self.provider_id,
            gateway_id = %invocation.gateway_id,
            family = %invocation.family.as_str(),
            tool_id = %invocation.tool_id,
            input_hash = %invocation.input_hash,
            "tool managed gateway provider executed sanitized invocation"
        );
        Ok(CapabilityToolInvocationResult::ok(
            "service.tool.managed_gateway",
            macaca_proto::CapabilityToolOriginKind::Mcp,
            invocation.invocation.tool_name,
            serde_json::json!({
                "route_kind": "managed_gateway",
                "family": invocation.family.as_str(),
                "provider_id": invocation.provider_id,
                "gateway_id": invocation.gateway_id,
                "metering_ref": metering_ref,
                "input_hash": invocation.input_hash,
                "status": "executed",
            }),
            invocation.invocation.trace,
        ))
    }

    async fn record_metering(&self, event: ToolGatewayMeteringEvent) -> MacacaResult<ToolAuditRef> {
        tracing::info!(
            trace_id = %event.trace.trace_id,
            provider_id = %self.provider_id,
            gateway_id = %event.gateway_id,
            tool_id = %event.tool_id,
            metering_ref = %event.metering_ref,
            units = event.units,
            "tool managed gateway metering hook recorded"
        );
        if let Ok(mut events) = self.metering_events.lock() {
            events.push(event);
        }
        Ok(ToolAuditRef::new("tool.gateway.meter.recorded"))
    }

    async fn record_audit(&self, event: ToolGatewayAuditEvent) -> MacacaResult<ToolAuditRef> {
        tracing::info!(
            trace_id = %event.trace.trace_id,
            provider_id = %self.provider_id,
            gateway_id = %event.gateway_id,
            tool_id = %event.tool_id,
            status = %event.status,
            latency_millis = event.latency_millis,
            "tool managed gateway audit hook recorded"
        );
        if let Ok(mut events) = self.audit_events.lock() {
            events.push(event);
        }
        Ok(ToolAuditRef::new("tool.gateway.audit.recorded"))
    }
}

/// Registry-style gateway diagnostics surface.
pub struct ToolManagedGatewayService {
    providers: Vec<Arc<dyn ToolManagedGatewayProvider>>,
}

impl ToolManagedGatewayService {
    pub fn new(providers: Vec<Arc<dyn ToolManagedGatewayProvider>>) -> Self {
        Self { providers }
    }

    pub async fn health(&self, trace: TraceContext) -> MacacaResult<ToolGatewayHealthResult> {
        let mut gateways = Vec::new();
        for provider in &self.providers {
            gateways.extend(provider.descriptors().await?);
        }
        let unavailable_count = gateways
            .iter()
            .filter(|gateway| matches!(gateway.health, ServiceHealth::Unavailable { .. }))
            .count();
        tracing::info!(
            trace_id = %trace.trace_id,
            provider_count = self.providers.len(),
            gateway_count = gateways.len(),
            unavailable_count,
            "tool managed gateway health snapshot captured"
        );
        Ok(ToolGatewayHealthResult {
            trace,
            gateways,
            captured_at: chrono::Utc::now(),
            audit_refs: vec![ToolAuditRef::new("tool.gateway.health")],
            metadata: BTreeMap::new(),
        })
    }

    pub async fn record_metering(
        &self,
        event: ToolGatewayMeteringEvent,
    ) -> MacacaResult<ToolAuditRef> {
        for provider in &self.providers {
            if provider.provider_id() == event.provider_id {
                return provider.record_metering(event).await;
            }
        }
        tracing::warn!(
            trace_id = %event.trace.trace_id,
            provider_id = %event.provider_id,
            gateway_id = %event.gateway_id,
            reason_code = "managed_gateway_provider_not_found",
            "tool managed gateway metering target was not registered"
        );
        Ok(ToolAuditRef::new("tool.gateway.meter.not_found"))
    }

    pub async fn invoke(
        &self,
        invocation: ToolManagedGatewayInvocation,
    ) -> MacacaResult<CapabilityToolInvocationResult> {
        for provider in &self.providers {
            if provider.provider_id() == invocation.provider_id {
                return provider.invoke(invocation).await;
            }
        }
        tracing::warn!(
            trace_id = %invocation.invocation.trace.trace_id,
            provider_id = %invocation.provider_id,
            gateway_id = %invocation.gateway_id,
            tool_id = %invocation.tool_id,
            reason_code = "managed_gateway_provider_not_found",
            "tool managed gateway invocation target was not registered"
        );
        Ok(CapabilityToolInvocationResult::failed(
            "service.tool.managed_gateway",
            macaca_proto::CapabilityToolOriginKind::Mcp,
            invocation.invocation.tool_name,
            "managed gateway provider is not registered",
            invocation.invocation.trace,
        ))
    }

    pub async fn record_audit(&self, event: ToolGatewayAuditEvent) -> MacacaResult<ToolAuditRef> {
        for provider in &self.providers {
            if provider.provider_id() == event.provider_id {
                return provider.record_audit(event).await;
            }
        }
        tracing::warn!(
            trace_id = %event.trace.trace_id,
            provider_id = %event.provider_id,
            gateway_id = %event.gateway_id,
            reason_code = "managed_gateway_provider_not_found",
            "tool managed gateway audit target was not registered"
        );
        Ok(ToolAuditRef::new("tool.gateway.audit.not_found"))
    }
}

/// Build the default provider-backed managed gateway service.
///
/// The descriptors model generic OS gateways for families that require network,
/// browser, media, document, or enterprise connector mediation.  Product- or
/// application-specific gateway code can be installed later by replacing these
/// providers through the same Adapter contract.
pub fn industrial_tool_managed_gateway_service() -> ToolManagedGatewayService {
    ToolManagedGatewayService::new(
        [
            ("browser", ToolManagedGatewayRouteKind::Browser),
            ("web", ToolManagedGatewayRouteKind::Web),
            ("media", ToolManagedGatewayRouteKind::Media),
            (
                "communication",
                ToolManagedGatewayRouteKind::EnterpriseConnector,
            ),
            (
                "enterprise_api",
                ToolManagedGatewayRouteKind::EnterpriseConnector,
            ),
        ]
        .into_iter()
        .map(|(family, route_kind)| {
            let provider_id = format!("provider.tool.family.{family}");
            let descriptor = ToolManagedGatewayDescriptor {
                gateway_id: format!("gateway.tool.family.{family}"),
                provider_id: provider_id.clone(),
                route_kind,
                family: ToolFamilyRef::new(family).expect("static family ids are non-empty"),
                health: ServiceHealth::Healthy,
                metering_required: true,
                audit_required: true,
                metadata: BTreeMap::new(),
            };
            Arc::new(StaticToolManagedGatewayProvider::new(
                provider_id,
                vec![descriptor],
            )) as Arc<dyn ToolManagedGatewayProvider>
        })
        .collect(),
    )
}
