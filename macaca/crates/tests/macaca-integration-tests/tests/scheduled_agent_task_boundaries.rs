//! Scheduled Agent Task executable boundary gates for prompt isolation,
//! serviceized shell routes, and generic OS-layer code.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    AgentExecutionCommand, AgentExecutionIntent, AgentExecutionResult, ApplicationId,
    AutonomyScope, CreateScheduledAgentTaskCommand, KernelServiceId, MacacaResult,
    ScheduledAgentTaskCommandResult, ScheduledAgentTaskSchedule, SchedulerCommandResult,
    SchedulerDeleteJobCommand, SchedulerGetJobCommand, SchedulerJobCommand, SchedulerJobDefinition,
    SchedulerJobId, SchedulerJobLifecycleState, SchedulerLifecycleJobCommand,
    SchedulerListJobsCommand, SchedulerQueryCommand, SchedulerRegisterJobCommand,
    SchedulerRunState, SchedulerRunSummary, SchedulerServiceSnapshot, SchedulerTargetCommand,
    SchedulerUpdateJobCommand, ServiceBusSource, ServiceDescriptor, ServiceHealth, TraceContext,
    SCHEDULED_AGENT_TASK_SERVICE_ID,
};
use macaca_runtime_host::{
    agent_execution_service_descriptor, autonomy_dispatch::AutonomyDispatchStrategies,
    AgentExecutionBackend, AgentExecutionSystemServiceProvider,
    ScheduledAgentTaskSystemServiceProvider, ServiceProviderFactoryContext,
    ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
};
use macaca_scheduled_agent_task::{LocalScheduledAgentTaskProvider, ScheduledAgentTaskService};
use macaca_scheduler::{SchedulerService, UnavailableSchedulerProvider};

/// Locate the Macaca Rust workspace root from the integration-test crate.
///
/// The test crate can be executed from different working directories in CI, so
/// relying on `current_dir` would make the gate fragile.  Walking ancestors from
/// `CARGO_MANIFEST_DIR` keeps the scanner deterministic and auditable.
fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from CARGO_MANIFEST_DIR")
}

/// Return the repository root that contains both `macaca/` and `frontend/`.
fn repository_root() -> PathBuf {
    workspace_root()
        .parent()
        .expect("Macaca workspace should live under the repository root")
        .to_path_buf()
}

/// Recursively collect files with a matching extension while skipping generated
/// and fixture-only directories.  This keeps each gate scoped to production
/// source and prevents target artifacts from changing test behavior.
fn collect_files_with_extension(dir: &Path, extension: &str, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if matches!(name, "target" | "tests" | "fixtures" | "examples" | ".git") {
            continue;
        }
        if path.is_dir() {
            collect_files_with_extension(&path, extension, files);
            continue;
        }
        if path.extension().and_then(OsStr::to_str) == Some(extension) {
            files.push(path);
        }
    }
}

/// Comments may mention forbidden concepts as design rationale.  The gate only
/// rejects executable source lines so architectural documentation remains
/// readable without weakening production enforcement.
fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("/*")
}

/// Read one file and panic with a precise diagnostic if it is unavailable.
fn read_source(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// Scheduler test double that records the job registered across the public
/// service boundary instead of inspecting provider internals.
#[derive(Default)]
struct RecordingSchedulerService {
    jobs: Mutex<Vec<SchedulerJobDefinition>>,
}

#[async_trait]
impl SchedulerService for RecordingSchedulerService {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = UnavailableSchedulerProvider::default().descriptor();
        descriptor.health = ServiceHealth::Healthy;
        descriptor
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<SchedulerServiceSnapshot> {
        UnavailableSchedulerProvider::default().health(trace).await
    }

    async fn snapshot(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerServiceSnapshot> {
        UnavailableSchedulerProvider::default()
            .snapshot(command)
            .await
    }

    async fn register_job(
        &self,
        command: SchedulerRegisterJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        self.jobs.lock().unwrap().push(command.definition);
        Ok(SchedulerCommandResult {
            job_id: Some(SchedulerJobId::new("scheduled-agent-integration-job").unwrap()),
            run_id: None,
            lifecycle: Some(SchedulerJobLifecycleState::Active),
            run_state: Some(SchedulerRunState::Queued),
            accepted: true,
            error: None,
            trace: command.trace,
            audit_id: Some("audit.scheduler.integration.registered".into()),
            metadata: Default::default(),
        })
    }

    async fn update_job(
        &self,
        command: SchedulerUpdateJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .update_job(command)
            .await
    }

    async fn pause_job(
        &self,
        command: SchedulerLifecycleJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .pause_job(command)
            .await
    }

    async fn resume_job(
        &self,
        command: SchedulerLifecycleJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .resume_job(command)
            .await
    }

    async fn delete_job(
        &self,
        command: SchedulerDeleteJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .delete_job(command)
            .await
    }

    async fn trigger_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .trigger_job(command)
            .await
    }

    async fn get_job(
        &self,
        command: SchedulerGetJobCommand,
    ) -> MacacaResult<Option<macaca_proto::SchedulerJobSummary>> {
        UnavailableSchedulerProvider::default()
            .get_job(command)
            .await
    }

    async fn list_jobs(
        &self,
        command: SchedulerListJobsCommand,
    ) -> MacacaResult<Vec<macaca_proto::SchedulerJobSummary>> {
        UnavailableSchedulerProvider::default()
            .list_jobs(command)
            .await
    }

    async fn get_run(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        UnavailableSchedulerProvider::default()
            .get_run(command)
            .await
    }

    async fn list_runs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerRunSummary>> {
        UnavailableSchedulerProvider::default()
            .list_runs(command)
            .await
    }
}

/// Agent Execution backend double that records the service command and returns a
/// sanitized completed result without executing tools or model calls.
///
/// The integration fixture emits a provider-neutral evidence reference so the
/// Runtime Host dispatch gate can distinguish a real completed result envelope
/// from a bare `completed` status. The evidence value is synthetic but stable,
/// and it intentionally does not encode application, workflow, provider, or
/// filesystem-specific semantics.
#[derive(Default)]
struct RecordingAgentExecutionBackend {
    commands: Mutex<Vec<AgentExecutionCommand>>,
}

#[async_trait]
impl AgentExecutionBackend for RecordingAgentExecutionBackend {
    async fn execute(
        &self,
        command: AgentExecutionCommand,
    ) -> macaca_proto::ServiceResult<AgentExecutionResult> {
        self.commands.lock().unwrap().push(command.clone());
        let mut result = AgentExecutionResult::completed(
            &command,
            serde_json::json!({"integration": "scheduled_agent_task"}),
        );
        result.metadata.insert(
            "result_evidence_ref".into(),
            "event/integration-scheduled-agent-result/1".into(),
        );
        Ok(result)
    }
}

/// Register and start one provider so calls pass through ServiceRuntime
/// decorators, trace validation, and typed service commands.
async fn register_static_service(
    runtime: &ServiceRuntime,
    descriptor: ServiceDescriptor,
    service: Arc<dyn macaca_kernel::SystemService>,
) {
    let service_id = descriptor.id.clone();
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .unwrap();
    runtime
        .start(
            &service_id,
            TraceContext::new(format!("trace-start-{}", service_id.as_str())),
        )
        .await
        .unwrap();
}

/// Assert that Scheduler-owned code carries only payload references. Raw prompts
/// stay in `service.scheduled_agent_task` until Runtime Host resolves them.
#[test]
fn scheduler_boundaries_reject_raw_scheduled_agent_task_payload_fields() {
    let root = workspace_root();
    let scan_roots = [
        root.join("crates/services/macaca-scheduler/src"),
        root.join("crates/foundation/macaca-proto/src/scheduler_service.rs"),
    ];
    let forbidden_tokens = ["user_prompt", "task_prompt", "delegated_context"];
    let mut violations = Vec::new();

    for scan_root in scan_roots {
        let mut files = Vec::new();
        if scan_root.is_file() {
            files.push(scan_root);
        } else {
            collect_files_with_extension(&scan_root, "rs", &mut files);
        }
        for file in files {
            let content = read_source(&file);
            for (index, line) in content.lines().enumerate() {
                if is_comment_only_line(line) {
                    continue;
                }
                for token in forbidden_tokens {
                    if line.contains(token) {
                        violations.push(format!(
                            "{}:{} contains raw payload token `{}`",
                            file.strip_prefix(&root).unwrap_or(&file).display(),
                            index + 1,
                            token
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Scheduler scheduled-agent-task boundary violations:\n{}",
        violations.join("\n")
    );
}

/// Assert that scheduled-agent-task UI/API creation uses the focused autonomy
/// namespace and not the legacy schedule namespace.
#[test]
fn scheduled_agent_task_shell_paths_use_focused_autonomy_namespace() {
    let repo = repository_root();
    let frontend_facade = read_source(&repo.join("frontend/lib/autonomy.ts"));
    let scheduled_task_section = frontend_facade
        .split("export function fetchScheduledAgentTasks")
        .nth(1)
        .expect("scheduled-agent-task frontend helpers should exist");
    assert!(
        scheduled_task_section.contains("'/scheduled-agent-tasks'")
            || scheduled_task_section.contains("`/scheduled-agent-tasks/"),
        "frontend scheduled-agent-task helpers must call /autonomy/scheduled-agent-tasks"
    );
    assert!(
        !scheduled_task_section.contains("'/schedules'")
            && !scheduled_task_section.contains("`/schedules/"),
        "frontend scheduled-agent-task helpers must not call legacy schedule endpoints"
    );

    let bootstrap =
        read_source(&workspace_root().join("crates/shells/macaca-web/src/bootstrap.rs"));
    assert!(
        bootstrap.contains("/api/apps/{app_id}/autonomy/scheduled-agent-tasks"),
        "Web must expose scheduled-agent-task routes under the serviceized autonomy namespace"
    );
}

/// Assert that the OS-layer implementation stays provider-neutral and
/// application-neutral.
///
/// This is a deliberately conservative source gate for new scheduled-task code.
/// The forbidden strings are examples of application domains, provider/model
/// names, gateway names, chain/payment names, and workflow-template branches
/// that must not appear in generic Macaca OS scheduled-task infrastructure.
#[test]
fn scheduled_agent_task_os_layer_has_no_business_or_provider_literals() {
    let root = workspace_root();
    let scan_roots = [
        root.join("crates/foundation/macaca-proto/src/scheduled_agent_task_service.rs"),
        root.join("crates/facade/macaca-sdk/src/scheduled_agent_task_client.rs"),
        root.join("crates/services/macaca-scheduled-agent-task/src"),
        root.join("crates/runtime/macaca-runtime-host/src/autonomy_dispatch.rs"),
        root.join("crates/shells/macaca-web/src/scheduled_agent_task_tool.rs"),
    ];
    let forbidden_tokens = [
        "\"openai",
        "\"anthropic",
        "\"claude-",
        "\"gpt-",
        "\"qwen",
        "\"deepseek",
        "\"stripe",
        "\"paypal",
        "\"ethereum",
        "\"solana",
        "\"bitcoin",
        "\"crypto",
        "\"finance",
        "\"trading",
        "\"slack",
        "\"telegram",
        "\"discord",
        "\"workflow.",
        "\"template.",
    ];
    let mut violations = Vec::new();

    for scan_root in scan_roots {
        let mut files = Vec::new();
        if scan_root.is_file() {
            files.push(scan_root);
        } else {
            collect_files_with_extension(&scan_root, "rs", &mut files);
        }
        for file in files {
            let content = read_source(&file);
            for (index, line) in content.lines().enumerate() {
                if is_comment_only_line(line) {
                    continue;
                }
                for token in forbidden_tokens {
                    if line.contains(token) {
                        violations.push(format!(
                            "{}:{} contains non-generic literal `{}`",
                            file.strip_prefix(&root).unwrap_or(&file).display(),
                            index + 1,
                            token
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Scheduled-agent-task OS-layer genericity violations:\n{}",
        violations.join("\n")
    );
}

/// Prove the generic create -> Scheduler target -> Runtime Host dispatch ->
/// Agent Execution service chain without leaking raw prompts to Scheduler.
#[tokio::test]
async fn scheduled_agent_task_create_to_dispatch_chain_preserves_audit_and_redaction() {
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig::default());
    let scheduler = Arc::new(RecordingSchedulerService::default());
    let scheduled_provider: Arc<dyn ScheduledAgentTaskService> =
        Arc::new(LocalScheduledAgentTaskProvider::new(scheduler.clone()));
    let scheduled_service = Arc::new(ScheduledAgentTaskSystemServiceProvider::new(
        scheduled_provider,
    ));
    let agent_execution_backend = Arc::new(RecordingAgentExecutionBackend::default());
    let agent_execution_service = Arc::new(AgentExecutionSystemServiceProvider::new(
        agent_execution_backend.clone(),
    ));

    register_static_service(&runtime, scheduled_service.descriptor(), scheduled_service).await;
    register_static_service(
        &runtime,
        agent_execution_service_descriptor(),
        agent_execution_service,
    )
    .await;

    let raw_prompt = "RAW_PROMPT_INTEGRATION_SHOULD_NOT_LEAK";
    let create = CreateScheduledAgentTaskCommand::new(
        TraceContext::new("trace-scheduled-agent-integration-create"),
        AutonomyScope::application(ApplicationId::from_name("scheduled-agent-integration-app")),
        ScheduledAgentTaskSchedule::Every {
            interval_ms: 60_000,
        },
        "worker",
        raw_prompt,
    )
    .unwrap();
    let reply = runtime
        .call(
            &KernelServiceId::new(SCHEDULED_AGENT_TASK_SERVICE_ID),
            ServiceBusSource::new("integration.scheduled_agent_task"),
            create.into_service_command().unwrap(),
        )
        .await
        .unwrap();
    assert!(reply.success);
    let result: ScheduledAgentTaskCommandResult =
        serde_json::from_value(reply.output.unwrap()).unwrap();
    assert!(result.accepted);
    let audit_id = result
        .audit_id
        .clone()
        .expect("scheduled task creation should return audit correlation");
    assert!(audit_id.starts_with("audit.scheduled_agent_task."));
    assert!(!serde_json::to_string(&result).unwrap().contains(raw_prompt));

    let target = {
        let jobs = scheduler.jobs.lock().unwrap();
        assert_eq!(jobs.len(), 1);
        let encoded_job = serde_json::to_string(&jobs[0]).unwrap();
        assert!(!encoded_job.contains(raw_prompt));
        match &jobs[0].target {
            SchedulerTargetCommand::AgentExecution(target) => target.clone(),
            other => panic!("expected AgentExecution target, got {other:?}"),
        }
    };

    let dispatcher = AutonomyDispatchStrategies::new(&runtime, 1_000);
    let outcome = dispatcher
        .dispatch(
            TraceContext::new("trace-scheduled-agent-integration-dispatch"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-agent-integration-app")),
            SchedulerTargetCommand::AgentExecution(target),
        )
        .await
        .unwrap();
    assert!(outcome.succeeded);

    let commands = agent_execution_backend.commands.lock().unwrap();
    assert_eq!(commands.len(), 1);
    let command = &commands[0];
    assert_eq!(command.execution_intent, AgentExecutionIntent::TaskWorker);
    assert_eq!(command.user_prompt, raw_prompt);
    assert_eq!(command.metadata["scheduled_agent_task_audit_id"], audit_id);
    assert_eq!(
        command.metadata["scheduler_run_source"],
        "service.scheduler"
    );
    assert!(command.metadata.contains_key("payload_digest"));
}
