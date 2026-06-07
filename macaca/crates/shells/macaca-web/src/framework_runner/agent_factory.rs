//! Web-shell traced agent factory (`TracedAgentFactory` implementation).

use std::sync::Arc;
use async_trait::async_trait;
use macaca_framework::agent::HookedAgent;
use macaca_framework::construction::{AgentBuildRequest, TracedAgentFactory};
use macaca_framework::react_agent::ReActAgent;
use super::agent_factory_build::WebTracedAgentFactory;
use super::build_mode::FrameworkRunnerBuildMode;

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
                self.build_runtime_agent(
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
