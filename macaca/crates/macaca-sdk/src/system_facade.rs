//! Shell-facing system facade for Web and CLI thin-shell migration.
//!
//! `SystemFacade` is the SDK-owned Facade that upper presentation shells call
//! instead of reaching into provider crates or Web internals. The facade is
//! deliberately composed from small Strategy clients so each capability can be
//! migrated to `ServiceRuntime` independently in later Route C phases.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::MacacaResult;

pub use crate::package_client::{
    EmptySystemPackageClient, PackageInspectionCommand, PackageInspectionResult,
    SystemPackageClient,
};
pub use crate::service_client::{
    ServiceCallCommand, ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    SystemServiceClient, UnavailableSystemServiceClient,
};
pub use crate::status_client::{
    kernel_status_snapshot, StaticSystemStatusDataSource, SystemStatusClient,
    SystemStatusDataSource, SystemStatusSnapshot,
};
pub use crate::task_client::{
    SystemTaskClient, TaskBoardDataSource, TaskBoardQueryCommand, TaskBoardQueryResult,
    TodoStoreTaskBoardDataSource,
};
pub use crate::trace_client::{
    EmptySystemTraceClient, SessionEventQueryCommand, SystemTraceClient, TraceQueryResult,
    TraceTailCommand,
};

/// Policy-ready approval command produced by Web/CLI confirmation surfaces.
///
/// Approval processing still belongs to policy/runtime services. S3 keeps this
/// command in the SDK so presentation shells can produce an auditable decision
/// object without becoming the policy engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecisionCommand {
    pub decision_id: String,
    pub approved: bool,
    pub decided_at: DateTime<Utc>,
}

/// SDK system facade consumed by Web/CLI thin shells.
///
/// The type parameters are intentionally capability-scoped Strategy clients.
/// Existing two-client call sites keep working through default service, trace,
/// and package clients that return structured empty/unavailable responses until
/// later Route C phases attach concrete runtime-backed implementations.
pub struct SystemFacade<
    T,
    S,
    SV = UnavailableSystemServiceClient,
    TR = EmptySystemTraceClient,
    P = EmptySystemPackageClient,
> {
    task_board: T,
    status: S,
    service: SV,
    trace: TR,
    package: P,
}

impl<T, S> SystemFacade<T, S>
where
    T: SystemTaskClient,
    S: SystemStatusClient,
{
    /// Create a facade from the current compatibility task and status clients.
    ///
    /// This constructor preserves the pre-S3 Web/CLI call shape. It installs
    /// explicit empty/unavailable clients for capabilities whose service-backed
    /// implementations are intentionally deferred to later Route C phases.
    pub fn new(task_board: T, status: S) -> Self {
        Self {
            task_board,
            status,
            service: UnavailableSystemServiceClient,
            trace: EmptySystemTraceClient,
            package: EmptySystemPackageClient,
        }
    }
}

impl<T, S, SV, TR, P> SystemFacade<T, S, SV, TR, P>
where
    T: SystemTaskClient,
    S: SystemStatusClient,
    SV: SystemServiceClient,
    TR: SystemTraceClient,
    P: SystemPackageClient,
{
    /// Create a facade from explicit capability clients.
    ///
    /// This constructor is the preferred composition path for tests and future
    /// serviceized runtimes because it makes every SDK boundary dependency
    /// visible at construction time.
    pub fn with_clients(task_board: T, status: S, service: SV, trace: TR, package: P) -> Self {
        Self {
            task_board,
            status,
            service,
            trace,
            package,
        }
    }

    /// Query a session-scoped task board through the facade boundary.
    pub async fn query_task_board(
        &self,
        command: TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            "system facade task board query started"
        );
        let result = self.task_board.query_task_board(&command).await?;
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            count = result.count,
            "system facade task board query completed"
        );
        Ok(result)
    }

    /// Return a shell-facing status snapshot without exposing presentation internals.
    pub async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot> {
        info!("system facade status snapshot requested");
        self.status.status_snapshot().await
    }

    /// Inspect service availability through the SDK service client boundary.
    pub async fn inspect_services(
        &self,
        command: ServiceInspectionCommand,
    ) -> MacacaResult<ServiceInspectionResult> {
        info!(
            scope = %command.scope,
            "system facade service inspection started"
        );
        self.service.inspect_services(&command).await
    }

    /// Dispatch a service command through the replaceable SDK service client.
    pub async fn call_service(
        &self,
        command: ServiceCallCommand,
    ) -> MacacaResult<ServiceCallResult> {
        info!(
            service_id = %command.service_id,
            command = %command.command_name,
            "system facade service call started"
        );
        self.service.call_service(&command).await
    }

    /// Replay session events through the replaceable trace client.
    pub async fn replay_events(
        &self,
        command: SessionEventQueryCommand,
    ) -> MacacaResult<TraceQueryResult> {
        info!(
            session_id = %command.session_id,
            since = ?command.since,
            limit = ?command.limit,
            "system facade trace replay started"
        );
        self.trace.replay_events(&command).await
    }

    /// Tail session trace events through the replaceable trace client.
    pub async fn tail_trace(&self, command: TraceTailCommand) -> MacacaResult<TraceQueryResult> {
        info!(
            session_id = %command.session_id,
            since = ?command.since,
            "system facade trace tail started"
        );
        self.trace.tail_trace(&command).await
    }

    /// Inspect packages through the replaceable package client.
    pub async fn inspect_packages(
        &self,
        command: PackageInspectionCommand,
    ) -> MacacaResult<PackageInspectionResult> {
        info!(
            package_ref = ?command.package_ref,
            "system facade package inspection started"
        );
        self.package.inspect_packages(&command).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_proto::{ApplicationId, TodoItem, TodoStatus};

    struct MockTaskBoardClient {
        todos: Vec<TodoItem>,
    }

    #[async_trait]
    impl SystemTaskClient for MockTaskBoardClient {
        async fn query_task_board(
            &self,
            _command: &TaskBoardQueryCommand,
        ) -> MacacaResult<TaskBoardQueryResult> {
            let mut todos = self.todos.clone();
            todos.sort_by_key(|todo| todo.sequence_number);
            let count = todos.len();
            Ok(TaskBoardQueryResult { todos, count })
        }
    }

    fn todo(sequence_number: u32) -> TodoItem {
        let mut item = TodoItem::new(
            ApplicationId(uuid::Uuid::new_v4()),
            Some("session-a".into()),
            "agent",
            "planner",
            "title",
            "description",
            1,
        );
        item.session_id = Some("session-a".into());
        item.status = TodoStatus::Pending;
        item.sequence_number = sequence_number;
        item.created_at = Utc::now();
        item.updated_at = Utc::now();
        item
    }

    #[tokio::test]
    async fn system_facade_returns_sorted_task_board_without_web_dependency() {
        let facade = SystemFacade::new(
            MockTaskBoardClient {
                todos: vec![todo(2), todo(1)],
            },
            StaticSystemStatusDataSource::new(SystemStatusSnapshot {
                version: "test".into(),
                agent_count: 0,
                loaded_apps: 0,
                max_agents: 8,
                llm_provider: "stub".into(),
                app_runtime: "macaca-app/AppRuntime".into(),
                gateway_enabled: false,
            }),
        );
        let result = facade
            .query_task_board(
                TaskBoardQueryCommand::new(ApplicationId(uuid::Uuid::new_v4()), "session-a")
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.count, 2);
        assert_eq!(result.todos[0].sequence_number, 1);
    }

    #[tokio::test]
    async fn default_service_client_returns_structured_unavailable() {
        let facade = SystemFacade::new(
            MockTaskBoardClient { todos: Vec::new() },
            StaticSystemStatusDataSource::new(SystemStatusSnapshot {
                version: "test".into(),
                agent_count: 0,
                loaded_apps: 0,
                max_agents: 8,
                llm_provider: "stub".into(),
                app_runtime: "macaca-app/AppRuntime".into(),
                gateway_enabled: false,
            }),
        );
        let command =
            ServiceCallCommand::new("service-a", "command-a", serde_json::json!({})).unwrap();
        let error = facade.call_service(command).await.unwrap_err();
        assert!(error.to_string().contains("unavailable"));
    }

    #[test]
    fn task_board_command_rejects_blank_session_scope() {
        let error = TaskBoardQueryCommand::new(ApplicationId(uuid::Uuid::new_v4()), "  ");
        assert!(error.is_err());
    }
}
