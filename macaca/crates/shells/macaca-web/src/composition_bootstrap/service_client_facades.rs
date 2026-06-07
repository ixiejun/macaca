//! Materialize typed SDK clients and the web system facade after service providers register.
//!
//! Split from [`super::service_runtime_wiring`] to keep each bootstrap module under the 500-line
//! file-size constitution while preserving the original registration-then-client ordering.

use std::sync::Arc;

use macaca_proto::MacacaResult;

use crate::shell::WebSystemFacadeBundle;

use super::bootstrap_ctx::BootstrapCtx;

/// Build `System*Client` handles and `WebSystemFacadeBundle` from the generic service bus client.
pub(crate) fn materialize_service_clients(ctx: &mut BootstrapCtx) -> MacacaResult<()> {
    let generic_service_client =
        Arc::clone(ctx.generic_service_client.as_ref().expect("bootstrap: generic_service_client"));

    let llm_client: Arc<dyn macaca_sdk::SystemLlmClient> = Arc::new(
        macaca_sdk::ServiceBackedLlmClient::new(Arc::clone(&generic_service_client)),
    );
    let context_client: Arc<dyn macaca_sdk::SystemContextClient> = Arc::new(
        macaca_sdk::ServiceBackedContextClient::new(Arc::clone(&generic_service_client)),
    );
    let driver_client: Arc<dyn macaca_sdk::SystemDriverClient> = Arc::new(
        macaca_sdk::ServiceBackedDriverClient::new(Arc::clone(&generic_service_client)),
    );
    let skill_client: Arc<dyn macaca_sdk::SystemSkillClient> = Arc::new(
        macaca_sdk::ServiceBackedSkillClient::new(Arc::clone(&generic_service_client)),
    );
    let mcp_client: Arc<dyn macaca_sdk::SystemMcpClient> = Arc::new(
        macaca_sdk::ServiceBackedMcpClient::new(Arc::clone(&generic_service_client)),
    );
    let tool_client: Arc<dyn macaca_sdk::SystemToolClient> = Arc::new(
        macaca_sdk::ServiceBackedToolClient::new(Arc::clone(&generic_service_client)),
    );
    let store_client: Arc<dyn macaca_sdk::SystemStoreClient> = Arc::new(
        macaca_sdk::ServiceBackedStoreClient::new(Arc::clone(&generic_service_client)),
    );
    let entitlement_client: Arc<dyn macaca_sdk::SystemEntitlementClient> = Arc::new(
        macaca_sdk::ServiceBackedEntitlementClient::new(Arc::clone(&generic_service_client)),
    );
    let payment_client: Arc<dyn macaca_sdk::SystemPaymentClient> = Arc::new(
        macaca_sdk::ServiceBackedPaymentClient::new(Arc::clone(&generic_service_client)),
    );
    let scheduler_client: Arc<dyn macaca_sdk::SystemSchedulerClient> = Arc::new(
        macaca_sdk::ServiceBackedSchedulerClient::new(Arc::clone(&generic_service_client)),
    );
    let scheduled_agent_task_client: Arc<dyn macaca_sdk::SystemScheduledAgentTaskClient> =
        Arc::new(macaca_sdk::ServiceBackedScheduledAgentTaskClient::new(Arc::clone(
            &generic_service_client,
        )));
    let heartbeat_client: Arc<dyn macaca_sdk::SystemHeartbeatClient> = Arc::new(
        macaca_sdk::ServiceBackedHeartbeatClient::new(Arc::clone(&generic_service_client)),
    );
    let web3_client: Arc<dyn macaca_sdk::SystemWeb3Client> = Arc::new(
        macaca_sdk::ServiceBackedWeb3Client::new(Arc::clone(&generic_service_client)),
    );
    let evm_client: Arc<dyn macaca_sdk::SystemEvmClient> = Arc::new(
        macaca_sdk::ServiceBackedEvmClient::new(Arc::clone(&generic_service_client)),
    );
    let plugin_control_client: Arc<dyn macaca_sdk::SystemPluginControlClient> = Arc::new(
        macaca_sdk::ServiceBackedPluginControlClient::new(Arc::clone(&generic_service_client)),
    );
    let plugin_capability_client: Arc<dyn macaca_sdk::SystemPluginCapabilityClient> = Arc::new(
        macaca_sdk::ServiceBackedPluginCapabilityClient::new(Arc::clone(&generic_service_client)),
    );
    let plugin_hook_client: Arc<dyn macaca_sdk::SystemPluginHookClient> = Arc::new(
        macaca_sdk::ServiceBackedPluginHookClient::new(Arc::clone(&generic_service_client)),
    );
    let application_execution_client: Arc<dyn macaca_sdk::SystemApplicationExecutionClient> =
        Arc::new(macaca_sdk::ServiceBackedApplicationExecutionClient::new(
            Arc::clone(&generic_service_client),
        ));
    let system_facade = WebSystemFacadeBundle::new(
        Arc::clone(&generic_service_client),
        Arc::clone(&web3_client),
        Arc::clone(&evm_client),
        Arc::clone(&plugin_control_client),
        Arc::clone(&plugin_capability_client),
        Arc::clone(&plugin_hook_client),
        Arc::clone(&application_execution_client),
    );

    ctx.llm_client = Some(llm_client);
    ctx.context_client = Some(context_client);
    ctx.driver_client = Some(driver_client);
    ctx.skill_client = Some(skill_client);
    ctx.mcp_client = Some(mcp_client);
    ctx.tool_client = Some(tool_client);
    ctx.store_client = Some(store_client);
    ctx.entitlement_client = Some(entitlement_client);
    ctx.payment_client = Some(payment_client);
    ctx.scheduler_client = Some(scheduler_client);
    ctx.scheduled_agent_task_client = Some(scheduled_agent_task_client);
    ctx.heartbeat_client = Some(heartbeat_client);
    ctx.web3_client = Some(web3_client);
    ctx.evm_client = Some(evm_client);
    ctx.plugin_control_client = Some(plugin_control_client);
    ctx.plugin_capability_client = Some(plugin_capability_client);
    ctx.plugin_hook_client = Some(plugin_hook_client);
    ctx.application_execution_client = Some(application_execution_client);
    ctx.system_facade = Some(system_facade);
    Ok(())
}
