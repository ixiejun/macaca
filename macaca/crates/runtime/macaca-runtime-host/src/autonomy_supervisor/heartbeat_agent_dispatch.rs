//! Manifest-declared heartbeat agent dispatch strategy.
//!
//! Runtime-host is the approved composition root for this bridge: Heartbeat
//! owns wake acceptance, Application Service owns manifest projection, and
//! Agent Execution owns the actual model/tool run. This Strategy connects
//! those services with typed commands and sanitized logs without making
//! Scheduler, Web routes, or filesystem scanning own heartbeat execution.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, ApplicationHeartbeatAgentView,
    ApplicationHeartbeatAgentsQueryCommand, ApplicationHeartbeatAgentsResult, ApplicationId,
    HeartbeatCommandResult, KernelServiceId, MacacaError, MacacaResult, ServiceBusSource,
    TraceContext, AGENT_EXECUTE_COMMAND, AGENT_EXECUTION_SERVICE_ID, APPLICATION_SERVICE_ID,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::ServiceRuntime;

/// Bounded dispatch summary recorded by HeartbeatLane logs and tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HeartbeatAgentDispatchSummary {
    pub queried: usize,
    pub enabled: usize,
    pub dispatched: usize,
    pub skipped: usize,
    pub failed: usize,
}

/// Replaceable Strategy for dispatching accepted heartbeat wakes to agents.
pub(crate) struct HeartbeatAgentDispatchStrategy {
    runtime: Arc<ServiceRuntime>,
}

impl HeartbeatAgentDispatchStrategy {
    /// Create a runtime-host strategy backed by the service runtime facade.
    pub(crate) fn new(runtime: Arc<ServiceRuntime>) -> Self {
        Self { runtime }
    }

    /// Dispatch enabled manifest declarations for one accepted Heartbeat wake.
    ///
    /// The wake result contains only sanitized metadata emitted by Heartbeat.
    /// If the wake is not app-scoped, or services are unavailable, the strategy
    /// returns structured skip/failure counts and leaves the Heartbeat lane
    /// alive. No branch in this method depends on app names, agent roles,
    /// providers, models, workflows, or business domains.
    pub(crate) async fn dispatch_after_accepted_wake(
        &self,
        wake: &HeartbeatCommandResult,
    ) -> MacacaResult<HeartbeatAgentDispatchSummary> {
        if !wake.accepted {
            return Ok(HeartbeatAgentDispatchSummary::default());
        }
        let Some(application_id) = application_id_from_wake(wake) else {
            info!(
                trace_id = wake.trace.trace_id.as_str(),
                "heartbeat agent dispatch skipped because accepted wake is not application-scoped"
            );
            return Ok(HeartbeatAgentDispatchSummary {
                skipped: 1,
                ..HeartbeatAgentDispatchSummary::default()
            });
        };

        let declarations = match self
            .query_declarations(wake.trace.clone(), application_id)
            .await
        {
            Ok(declarations) => declarations,
            Err(error) => {
                warn!(
                    trace_id = wake.trace.trace_id.as_str(),
                    app_id = %application_id,
                    error = %error,
                    "heartbeat agent dispatch declaration query failed"
                );
                return Ok(HeartbeatAgentDispatchSummary {
                    failed: 1,
                    ..HeartbeatAgentDispatchSummary::default()
                });
            }
        };
        let mut summary = HeartbeatAgentDispatchSummary {
            queried: declarations.len(),
            ..HeartbeatAgentDispatchSummary::default()
        };
        for declaration in declarations {
            if !declaration.enabled || !declaration.diagnostics.is_empty() {
                summary.skipped += 1;
                continue;
            }
            summary.enabled += 1;
            match self.dispatch_agent(wake, &declaration).await {
                Ok(()) => summary.dispatched += 1,
                Err(error) => {
                    summary.failed += 1;
                    warn!(
                        trace_id = wake.trace.trace_id.as_str(),
                        app_id = %application_id,
                        agent_name = %declaration.agent_name,
                        error = %error,
                        "heartbeat agent dispatch request failed"
                    );
                }
            }
        }
        info!(
            trace_id = wake.trace.trace_id.as_str(),
            app_id = %application_id,
            queried = summary.queried,
            enabled = summary.enabled,
            dispatched = summary.dispatched,
            skipped = summary.skipped,
            failed = summary.failed,
            "heartbeat agent dispatch completed"
        );
        Ok(summary)
    }

    async fn query_declarations(
        &self,
        trace: TraceContext,
        application_id: ApplicationId,
    ) -> MacacaResult<ApplicationHeartbeatAgentsResult> {
        let command =
            ApplicationHeartbeatAgentsQueryCommand::application(trace.clone(), application_id)?
                .into_service_command()?;
        let reply = self
            .runtime
            .call(
                &KernelServiceId::new(APPLICATION_SERVICE_ID),
                ServiceBusSource::new("runtime.heartbeat_agent_dispatch"),
                command,
            )
            .await
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        let output = reply.output.ok_or_else(|| {
            MacacaError::Config("application heartbeat declaration query returned no output".into())
        })?;
        serde_json::from_value(output).map_err(|error| MacacaError::Config(error.to_string()))
    }

    async fn dispatch_agent(
        &self,
        wake: &HeartbeatCommandResult,
        declaration: &ApplicationHeartbeatAgentView,
    ) -> MacacaResult<()> {
        let mut command = AgentExecutionCommand::new(
            declaration.application_id,
            session_id_from_wake(wake, declaration),
            declaration.agent_name.clone(),
            AgentExecutionIntent::Heartbeat,
            "Execute manifest-declared HEARTBEAT.md instructions from trusted agent context.",
            wake.trace.clone(),
        )?;
        command.metadata = dispatch_metadata(wake, declaration);
        command.delegated_context = serde_json::json!({
            "heartbeat": {
                "run_id": wake.run_id.as_ref().map(|run_id| run_id.as_str()),
                "audit_id": wake.audit_id,
                "profile_id": declaration.profile_id,
            }
        });
        let service_command = command.into_service_command()?;
        let reply = self
            .runtime
            .call(
                &KernelServiceId::new(AGENT_EXECUTION_SERVICE_ID),
                ServiceBusSource::new("runtime.heartbeat_agent_dispatch"),
                service_command,
            )
            .await
            .map_err(|error| MacacaError::Config(error.to_string()))?;
        if reply.success {
            info!(
                trace_id = wake.trace.trace_id.as_str(),
                command = AGENT_EXECUTE_COMMAND,
                status = %reply.status,
                "heartbeat agent execution command accepted"
            );
            Ok(())
        } else {
            Err(MacacaError::Config(format!(
                "agent execution returned {}",
                reply.status
            )))
        }
    }
}

fn application_id_from_wake(wake: &HeartbeatCommandResult) -> Option<ApplicationId> {
    wake.metadata
        .get("application_id")
        .and_then(|value| Uuid::parse_str(value).ok())
        .map(ApplicationId)
}

fn session_id_from_wake(
    wake: &HeartbeatCommandResult,
    declaration: &ApplicationHeartbeatAgentView,
) -> String {
    wake.metadata
        .get("session_id")
        .cloned()
        .unwrap_or_else(|| format!("heartbeat:{}", declaration.application_id))
}

fn dispatch_metadata(
    wake: &HeartbeatCommandResult,
    declaration: &ApplicationHeartbeatAgentView,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("source".into(), "service.heartbeat".into());
    metadata.insert("execution_intent".into(), "heartbeat".into());
    metadata.insert("profile_id".into(), declaration.profile_id.clone());
    if let Some(run_id) = wake.run_id.as_ref() {
        metadata.insert("heartbeat_run_id".into(), run_id.as_str().to_string());
    }
    if let Some(audit_id) = wake.audit_id.as_ref() {
        metadata.insert("heartbeat_audit_id".into(), audit_id.clone());
    }
    metadata
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use macaca_app::application_service_descriptor;
    use macaca_kernel::SystemService;
    use macaca_proto::{
        AgentExecutionResult, CleanupPolicy, HeartbeatRunId, HeartbeatWakeDisposition,
        ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
        ServiceResult, APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND,
    };

    use super::*;
    use crate::{
        agent_execution_service_descriptor, AgentExecutionBackend,
        AgentExecutionSystemServiceProvider, ServiceProviderFactoryContext,
        ServiceProviderInstance, ServiceRuntimeConfig, StaticServiceProviderFactory,
    };

    /// Application Service test double that returns prebuilt manifest projections.
    ///
    /// The fake deliberately implements only the provider-neutral
    /// `SystemService` contract. Tests therefore exercise the same ServiceRuntime
    /// bus, command decoding, and output shaping used by production dispatch
    /// without reaching into concrete application registries or app-specific
    /// fixtures.
    struct FakeApplicationHeartbeatService {
        descriptor: ServiceDescriptor,
        declarations: ApplicationHeartbeatAgentsResult,
    }

    impl FakeApplicationHeartbeatService {
        fn new(declarations: ApplicationHeartbeatAgentsResult) -> Self {
            Self {
                descriptor: application_service_descriptor(),
                declarations,
            }
        }
    }

    #[async_trait]
    impl SystemService for FakeApplicationHeartbeatService {
        fn descriptor(&self) -> ServiceDescriptor {
            self.descriptor.clone()
        }

        async fn start(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
            let trace = command
                .trace
                .clone()
                .ok_or(ServiceError::MissingTraceContext)?;
            if command.name.as_str() != APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND {
                return Err(ServiceError::UnsupportedCommand(format!(
                    "unsupported fake application command {}",
                    command.name
                )));
            }
            Ok(ServiceCallResult {
                status: "ok".into(),
                output: serde_json::to_value(&self.declarations)
                    .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
                trace,
                metadata: Default::default(),
                cleanup_hint: Some(CleanupPolicy::None),
            })
        }

        async fn stop(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn cleanup(&self) -> ServiceResult<()> {
            Ok(())
        }

        async fn health(&self) -> ServiceResult<ServiceHealth> {
            Ok(ServiceHealth::Healthy)
        }
    }

    /// Agent Execution backend that records typed commands for assertions.
    ///
    /// The Strategy under test only sees the service boundary. Capturing the
    /// decoded command here proves the dispatch metadata, intent, and target
    /// agent survive the ServiceRuntime hop without exposing test-only branches
    /// in production code.
    #[derive(Default)]
    struct RecordingExecutionBackend {
        commands: Mutex<Vec<AgentExecutionCommand>>,
    }

    #[async_trait]
    impl AgentExecutionBackend for RecordingExecutionBackend {
        async fn execute(
            &self,
            command: AgentExecutionCommand,
        ) -> ServiceResult<AgentExecutionResult> {
            self.commands.lock().unwrap().push(command.clone());
            Ok(AgentExecutionResult::completed(
                &command,
                serde_json::json!({"accepted": true}),
            ))
        }
    }

    async fn register_static_service(
        runtime: &ServiceRuntime,
        descriptor: ServiceDescriptor,
        service: Arc<dyn SystemService>,
    ) {
        let factory =
            StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service));
        runtime
            .register_provider(&factory, ServiceProviderFactoryContext::new())
            .await
            .unwrap();
    }

    fn accepted_app_wake(application_id: ApplicationId) -> HeartbeatCommandResult {
        let mut metadata = BTreeMap::new();
        metadata.insert("application_id".into(), application_id.to_string());
        metadata.insert("session_id".into(), "session-heartbeat".into());
        HeartbeatCommandResult {
            run_id: Some(HeartbeatRunId::new("run-heartbeat").unwrap()),
            state: None,
            disposition: HeartbeatWakeDisposition::Accepted,
            gates: Vec::new(),
            accepted: true,
            error: None,
            trace: TraceContext::new("trace-heartbeat-dispatch"),
            audit_id: Some("audit-heartbeat".into()),
            metadata,
        }
    }

    #[tokio::test]
    async fn declaration_driven_dispatch_calls_agent_execution() {
        let application_id = ApplicationId::from_name("generic-app");
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let backend = Arc::new(RecordingExecutionBackend::default());
        let declaration = ApplicationHeartbeatAgentView {
            application_id,
            agent_name: "operator".into(),
            enabled: true,
            profile_id: "default".into(),
            metadata: BTreeMap::new(),
            diagnostics: Vec::new(),
        };

        register_static_service(
            &runtime,
            application_service_descriptor(),
            Arc::new(FakeApplicationHeartbeatService::new(vec![declaration])),
        )
        .await;
        register_static_service(
            &runtime,
            agent_execution_service_descriptor(),
            Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
        )
        .await;

        let summary = HeartbeatAgentDispatchStrategy::new(runtime)
            .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
            .await
            .unwrap();

        assert_eq!(
            summary,
            HeartbeatAgentDispatchSummary {
                queried: 1,
                enabled: 1,
                dispatched: 1,
                skipped: 0,
                failed: 0,
            }
        );
        let commands = backend.commands.lock().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].execution_intent,
            AgentExecutionIntent::Heartbeat
        );
        assert_eq!(commands[0].target_agent, "operator");
        assert_eq!(commands[0].metadata["source"], "service.heartbeat");
        assert_eq!(
            commands[0].metadata["heartbeat_audit_id"],
            "audit-heartbeat"
        );
    }

    #[tokio::test]
    async fn absent_declarations_return_empty_structured_summary() {
        let application_id = ApplicationId::from_name("generic-app");
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        register_static_service(
            &runtime,
            application_service_descriptor(),
            Arc::new(FakeApplicationHeartbeatService::new(Vec::new())),
        )
        .await;

        let summary = HeartbeatAgentDispatchStrategy::new(runtime)
            .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
            .await
            .unwrap();

        assert_eq!(summary, HeartbeatAgentDispatchSummary::default());
    }

    #[tokio::test]
    async fn unavailable_application_service_returns_failure_evidence() {
        let application_id = ApplicationId::from_name("generic-app");
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));

        let summary = HeartbeatAgentDispatchStrategy::new(runtime)
            .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
            .await
            .unwrap();

        assert_eq!(
            summary,
            HeartbeatAgentDispatchSummary {
                failed: 1,
                ..HeartbeatAgentDispatchSummary::default()
            }
        );
    }

    #[tokio::test]
    async fn unavailable_agent_execution_service_returns_failure_evidence() {
        let application_id = ApplicationId::from_name("generic-app");
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let declaration = ApplicationHeartbeatAgentView {
            application_id,
            agent_name: "operator".into(),
            enabled: true,
            profile_id: "default".into(),
            metadata: BTreeMap::new(),
            diagnostics: Vec::new(),
        };

        register_static_service(
            &runtime,
            application_service_descriptor(),
            Arc::new(FakeApplicationHeartbeatService::new(vec![declaration])),
        )
        .await;

        let summary = HeartbeatAgentDispatchStrategy::new(runtime)
            .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
            .await
            .unwrap();

        assert_eq!(
            summary,
            HeartbeatAgentDispatchSummary {
                queried: 1,
                enabled: 1,
                failed: 1,
                ..HeartbeatAgentDispatchSummary::default()
            }
        );
    }

    #[tokio::test]
    async fn invalid_declarations_are_skipped_without_dispatch() {
        let application_id = ApplicationId::from_name("generic-app");
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
        let backend = Arc::new(RecordingExecutionBackend::default());
        let declaration = ApplicationHeartbeatAgentView {
            application_id,
            agent_name: "operator".into(),
            enabled: true,
            profile_id: "default".into(),
            metadata: BTreeMap::new(),
            diagnostics: vec!["heartbeat_agent_unknown".into()],
        };

        register_static_service(
            &runtime,
            application_service_descriptor(),
            Arc::new(FakeApplicationHeartbeatService::new(vec![declaration])),
        )
        .await;
        register_static_service(
            &runtime,
            agent_execution_service_descriptor(),
            Arc::new(AgentExecutionSystemServiceProvider::new(backend.clone())),
        )
        .await;

        let summary = HeartbeatAgentDispatchStrategy::new(runtime)
            .dispatch_after_accepted_wake(&accepted_app_wake(application_id))
            .await
            .unwrap();

        assert_eq!(
            summary,
            HeartbeatAgentDispatchSummary {
                queried: 1,
                skipped: 1,
                ..HeartbeatAgentDispatchSummary::default()
            }
        );
        assert!(backend.commands.lock().unwrap().is_empty());
    }
}
