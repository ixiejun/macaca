use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_proto::{
    MacacaResult, ServiceHealth, ToolCatalogPlanCommand, ToolCatalogPlanResult,
    ToolServiceHealthResult, TraceContext, TOOL_CATALOG_PLAN_COMMAND, TOOL_PROVIDER_HEALTH_COMMAND,
    TOOL_SERVICE_ID,
};
use macaca_sdk::{
    ServiceBackedToolClient, ServiceCallCommand, ServiceCallResult, ServiceInspectionCommand,
    ServiceInspectionResult, SystemServiceClient, SystemToolClient, UnavailableSystemToolClient,
};

#[tokio::test]
async fn unavailable_tool_client_returns_structured_unavailable_state() {
    let client = UnavailableSystemToolClient;
    let trace = TraceContext::new("trace-tool-unavailable");

    let plan = client
        .catalog_plan(ToolCatalogPlanCommand::new(trace.clone()).unwrap())
        .await
        .unwrap();
    let health = client.provider_health(trace).await.unwrap();

    assert!(plan.visible.is_empty());
    assert!(plan.hidden.is_empty());
    assert!(matches!(health.health, ServiceHealth::Unavailable { .. }));
}

#[tokio::test]
async fn service_backed_tool_client_routes_through_generic_service_client() {
    let service = Arc::new(CapturingServiceClient::default());
    let client = ServiceBackedToolClient::new(service.clone());

    let result = client
        .catalog_plan(ToolCatalogPlanCommand::new(TraceContext::new("trace-tool-plan")).unwrap())
        .await
        .unwrap();
    let health = client
        .provider_health(TraceContext::new("trace-tool-health"))
        .await
        .unwrap();

    let calls = service.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].service_id, TOOL_SERVICE_ID);
    assert_eq!(calls[0].command_name, TOOL_CATALOG_PLAN_COMMAND);
    assert_eq!(calls[1].command_name, TOOL_PROVIDER_HEALTH_COMMAND);
    assert!(result.visible.is_empty());
    assert_eq!(health.provider_count, 0);
}

#[derive(Default)]
struct CapturingServiceClient {
    calls: Mutex<Vec<ServiceCallCommand>>,
}

#[async_trait]
impl SystemServiceClient for CapturingServiceClient {
    async fn inspect_services(
        &self,
        command: &ServiceInspectionCommand,
    ) -> MacacaResult<ServiceInspectionResult> {
        Ok(ServiceInspectionResult {
            scope: command.scope.clone(),
            services: Vec::new(),
        })
    }

    async fn call_service(&self, command: &ServiceCallCommand) -> MacacaResult<ServiceCallResult> {
        self.calls.lock().unwrap().push(command.clone());
        let output = match command.command_name.as_str() {
            TOOL_CATALOG_PLAN_COMMAND => {
                serde_json::to_value(ToolCatalogPlanResult::empty(command.trace.clone().unwrap()))?
            }
            TOOL_PROVIDER_HEALTH_COMMAND => serde_json::to_value(ToolServiceHealthResult {
                trace: command.trace.clone().unwrap(),
                health: ServiceHealth::Healthy,
                provider_count: 0,
                captured_at: chrono::Utc::now(),
                metadata: Default::default(),
            })?,
            other => panic!("unexpected command: {other}"),
        };

        Ok(ServiceCallResult {
            service_id: command.service_id.clone(),
            output,
        })
    }
}
