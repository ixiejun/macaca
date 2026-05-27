//! Managed gateway provider seam for gateway-backed tool capabilities.
//!
//! Managed gateways are optional providers behind `service.tool`; they are not a
//! second control plane.  This module keeps routing metadata, metering, audit,
//! and health as provider-neutral data so gateway vendors remain replaceable.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    MacacaResult, ServiceHealth, ToolAuditRef, ToolFamilyRef, ToolGatewayAuditEvent,
    ToolGatewayHealthResult, ToolGatewayMeteringEvent, ToolManagedGatewayDescriptor,
    ToolManagedGatewayRouteKind, TraceContext,
};

/// Adapter contract implemented by managed gateway providers.
#[async_trait]
pub trait ToolManagedGatewayProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    async fn descriptors(&self) -> MacacaResult<Vec<ToolManagedGatewayDescriptor>>;
    async fn record_metering(&self, event: ToolGatewayMeteringEvent) -> MacacaResult<ToolAuditRef>;
    async fn record_audit(&self, event: ToolGatewayAuditEvent) -> MacacaResult<ToolAuditRef>;
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
