//! Web-shell traced agent factory (`TracedAgentFactory` implementation).

use super::agent_factory_build::WebTracedAgentFactory;
use super::build_mode::FrameworkRunnerBuildMode;
use async_trait::async_trait;
use macaca_host_composition::framework::agent::HookedAgent;
use macaca_host_composition::framework::construction::{AgentBuildRequest, TracedAgentFactory};
use macaca_host_composition::framework::react_agent::ReActAgent;
use std::sync::Arc;

#[async_trait]
impl TracedAgentFactory for WebTracedAgentFactory {
    type Output = HookedAgent<ReActAgent>;

    async fn build(&self, request: AgentBuildRequest) -> Result<Self::Output, String> {
        match &self.build_mode {
            FrameworkRunnerBuildMode::Executor { executor } => {
                self.build_executor_agent(request, Arc::clone(executor))
                    .await
            }
            FrameworkRunnerBuildMode::Runtime {
                event_tx,
                execution_control,
                max_iters,
                tool_choice,
            } => {
                self.materialize_runtime_agent(
                    request,
                    event_tx.clone(),
                    execution_control.clone(),
                    *max_iters,
                    tool_choice.clone(),
                )
                .await
            }
            FrameworkRunnerBuildMode::Coordinator { .. } => {
                Err("Coordinator construction requires owned channels".into())
            }
        }
    }
}
