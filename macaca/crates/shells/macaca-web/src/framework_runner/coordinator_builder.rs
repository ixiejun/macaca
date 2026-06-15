//! Coordinator agent builder with SSE bridging and execution-control barriers.

use super::agent_factory_build::WebTracedAgentFactory;
use super::agent_factory_coordinator::build_coordinator_agent;
use super::build_mode::FrameworkRunnerBuildMode;
use super::request_composition;
use super::runtime_execution_control::RuntimeExecutionControl;
use super::FrameworkRunner;
use crate::state::AppState;
use axum::response::sse::Event;
use macaca_host_composition::framework::agent::HookedAgent;
use macaca_host_composition::framework::construction::{AgentBuildIntent, AgentToolConfig};
use macaca_host_composition::framework::react_agent::ReActAgent;
use macaca_proto::ApplicationId;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::mpsc;

impl FrameworkRunner {
    /// Build a coordinator `ReActAgent` wrapped with `HookedAgent` for SSE bridging
    /// and policy-driven execution-control barriers.
    ///
    /// Returns `(HookedAgent<ReActAgent>, CancellationToken)`.
    pub(crate) async fn build_coordinator(
        state: &Arc<AppState>,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<String>,
        sse_tx: mpsc::Sender<Result<Event, Infallible>>,
        execution_control: RuntimeExecutionControl,
    ) -> Result<(HookedAgent<ReActAgent>, tokio_util::sync::CancellationToken), String> {
        let request = request_composition::build_request(
            state,
            app_id,
            agent_name,
            session_id.clone(),
            macaca_proto::TaskId::new(),
            None,
            AgentBuildIntent::CoordinatorChat,
            AgentToolConfig::default(),
        )
        .await?;
        let factory = WebTracedAgentFactory {
            state: Arc::clone(state),
            build_mode: FrameworkRunnerBuildMode::Coordinator {
                sse_tx,
                execution_control,
            },
        };

        build_coordinator_agent(factory, request).await
    }
}
