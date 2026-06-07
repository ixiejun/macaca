//! Static serviceization escape-hatch gate.
//!
//! This integration test is an executable specification for the freeze-first
//! refactor plan. It does not remove existing behavior; instead it prevents new
//! production Rust code from growing direct runtime/provider access while the
//! owners migrate callers to service clients and facades.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// A source token that must not appear in production Rust code unless the file
/// is an approved migration surface, test, fixture, or service provider bridge.
struct ForbiddenToken {
    family: &'static str,
    token: &'static str,
    rationale: &'static str,
}

/// A deterministic violation rendered in sorted order for stable CI output.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Violation {
    family: &'static str,
    path: PathBuf,
    line: usize,
    token: &'static str,
    rationale: &'static str,
}

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

fn forbidden_tokens() -> Vec<ForbiddenToken> {
    vec![
        ForbiddenToken {
            family: "application-runtime-direct-start",
            token: "AppRuntime::start_app",
            rationale: "application lifecycle must enter through Application Service commands",
        },
        ForbiddenToken {
            family: "application-runtime-direct-start",
            token: "start_app_from_file",
            rationale: "file-backed application starts must pass through traced service commands",
        },
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.driver_runtime",
            rationale: "Web must use the driver service client instead of runtime internals",
        },
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.mcp_runtime",
            rationale: "Web must use the MCP service catalog/snapshot command",
        },
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.runtime",
            rationale: "Web must use application service clients instead of AppRuntime anchors",
        },
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.registry",
            rationale: "Web must use service-backed application metadata views",
        },
        ForbiddenToken {
            family: "hardcoded-agent-role",
            token: "\"coordinator\"",
            rationale: "production OS layers must receive agent names from manifests or service descriptors",
        },
        ForbiddenToken {
            family: "hardcoded-agent-role",
            token: "\"planner\"",
            rationale: "production OS layers must receive agent names from manifests or service descriptors",
        },
        ForbiddenToken {
            family: "hardcoded-agent-role",
            token: "\"worker\"",
            rationale: "production OS layers must receive agent names from manifests or service descriptors",
        },
        ForbiddenToken {
            family: "hardcoded-agent-role",
            token: "\"backend\"",
            rationale: "production OS layers must receive agent names from manifests or service descriptors",
        },
        ForbiddenToken {
            family: "hardcoded-agent-role",
            token: "\"frontend\"",
            rationale: "production OS layers must receive agent names from manifests or service descriptors",
        },
        ForbiddenToken {
            family: "hardcoded-agent-role",
            token: "\"architect\"",
            rationale: "production OS layers must receive agent names from manifests or service descriptors",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"openai\"",
            rationale: "provider/model routing names must stay inside LLM service descriptors",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"anthropic\"",
            rationale: "provider/model routing names must stay inside LLM service descriptors",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"dashscope\"",
            rationale: "provider/model routing names must stay inside LLM service descriptors",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"deepseek\"",
            rationale: "provider/model routing names must stay inside LLM service descriptors",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"minimax\"",
            rationale: "provider/model routing names must stay inside LLM service descriptors",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"openrouter\"",
            rationale: "provider/model routing names must stay inside LLM service descriptors",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"gpt-",
            rationale: "model-family routing prefixes must be descriptor data inside the LLM service",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"claude-",
            rationale: "model-family routing prefixes must be descriptor data inside the LLM service",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"qwen",
            rationale: "model-family routing prefixes must be descriptor data inside the LLM service",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"deepseek-",
            rationale: "model-family routing prefixes must be descriptor data inside the LLM service",
        },
        ForbiddenToken {
            family: "provider-model-routing-name",
            token: "\"minimax-",
            rationale: "model-family routing prefixes must be descriptor data inside the LLM service",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "SchedulerSystemServiceProvider",
            rationale: "scheduler providers must be constructed only by runtime-host autonomy composition",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "HeartbeatSystemServiceProvider",
            rationale: "heartbeat providers must be constructed only by runtime-host autonomy composition",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "LocalSchedulerProvider",
            rationale: "local scheduler engines must remain replaceable service providers",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "LocalHeartbeatProvider",
            rationale: "local heartbeat engines must remain replaceable service providers",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "AutonomySupervisor",
            rationale: "autonomy loops must remain lifecycle-managed runtime-host infrastructure",
        },
        ForbiddenToken {
            family: "autonomy-loop-boundary",
            token: "run_scheduler_tick_once",
            rationale: "scheduler ticks must be owned by runtime-host autonomy supervisor",
        },
        ForbiddenToken {
            family: "autonomy-loop-boundary",
            token: "run_heartbeat_tick_once",
            rationale: "heartbeat ticks must be owned by runtime-host autonomy supervisor",
        },
        ForbiddenToken {
            family: "autonomy-loop-boundary",
            token: "run_recovery_wake_once",
            rationale: "recovery wake loops must be owned by runtime-host autonomy supervisor",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "\"service.scheduler\"",
            rationale: "scheduler service ids must flow through protocol DTOs, SDK clients, or runtime-host service registration",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "\"service.heartbeat\"",
            rationale: "heartbeat service ids must flow through protocol DTOs, SDK clients, or runtime-host service registration",
        },
        // P0 freeze: provider-compat and legacy execution adapters (tasks 1.1.1).
        ForbiddenToken {
            family: "provider-compat-construction",
            token: "KernelProviderCompat",
            rationale: "kernel provider bundles must be replaced by service-client AgentExecutionPort wiring",
        },
        ForbiddenToken {
            family: "provider-compat-construction",
            token: "LegacyLlmProvider",
            rationale: "LLM access must flow through service.agent_execution or LLM service clients",
        },
        ForbiddenToken {
            family: "provider-compat-construction",
            token: "LegacyToolCatalog",
            rationale: "tool catalogs must flow through driver/skill/MCP service snapshot commands",
        },
        ForbiddenToken {
            family: "provider-compat-construction",
            token: "LegacyAgentExecutionAdapter",
            rationale: "agent execution must use ServiceClientAgentExecutionAdapter against service.agent_execution",
        },
        // P0 freeze: additional deprecated AppState direct fields (tasks 1.1.2).
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.llm",
            rationale: "Web must use the LLM service client instead of direct provider handles",
        },
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.router",
            rationale: "Web must use service-backed model routing instead of direct router handles",
        },
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.memory_runtime",
            rationale: "Web must use memory/context service clients instead of runtime internals",
        },
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.driver_registry",
            rationale: "Web must use the driver service client instead of registry internals",
        },
        ForbiddenToken {
            family: "web-direct-runtime-field",
            token: "state.llm_client",
            rationale: "Web must use focused SDK LLM clients instead of shell-owned provider bridges",
        },
        // P0 freeze: direct driver/MCP runtime catalog reads (tasks 1.1.4).
        ForbiddenToken {
            family: "direct-runtime-catalog-read",
            token: "collect_tools()",
            rationale: "tool catalogs must be fetched through driver service snapshot commands",
        },
        ForbiddenToken {
            family: "direct-runtime-catalog-read",
            token: ".definitions().await",
            rationale: "MCP definitions must be fetched through MCP service snapshot commands",
        },
        // P0 freeze: kernel non-kernel modules before P2 eviction (tasks 1.1.6).
        ForbiddenToken {
            family: "kernel-non-kernel-module",
            token: "macaca_kernel::web3",
            rationale: "Web3 must be accessed through optional module or web3 service providers",
        },
        ForbiddenToken {
            family: "kernel-non-kernel-module",
            token: "macaca_kernel::evm",
            rationale: "EVM must be accessed through optional module or EVM service providers",
        },
        ForbiddenToken {
            family: "kernel-non-kernel-module",
            token: "macaca_kernel::a2a",
            rationale: "A2A must be accessed through payment/A2A service providers",
        },
        ForbiddenToken {
            family: "kernel-non-kernel-module",
            token: "macaca_kernel::payment_policy",
            rationale: "payment policy must be owned by payment service providers",
        },
        ForbiddenToken {
            family: "kernel-non-kernel-module",
            token: "kernel::web3",
            rationale: "kernel must not grow new Web3 module references",
        },
        ForbiddenToken {
            family: "kernel-non-kernel-module",
            token: "kernel::evm",
            rationale: "kernel must not grow new EVM module references",
        },
        ForbiddenToken {
            family: "kernel-non-kernel-module",
            token: "kernel::a2a",
            rationale: "kernel must not grow new A2A module references",
        },
        ForbiddenToken {
            family: "kernel-non-kernel-module",
            token: "kernel::payment_policy",
            rationale: "kernel must not grow new payment policy references",
        },
        // P0 freeze: multi-path coordination patches (tasks 1.1.7).
        ForbiddenToken {
            family: "multi-path-coordination-patch",
            token: "suppress_executor_lifecycle",
            rationale: "single execution owner makes lifecycle suppression patches unnecessary",
        },
        ForbiddenToken {
            family: "multi-path-coordination-patch",
            token: "legacy_chat_main_thread_goal_pause",
            rationale: "execution-control policy must come from manifest projection, not shell patches",
        },
        ForbiddenToken {
            family: "multi-path-coordination-patch",
            token: "legacy_unmarked",
            rationale: "hosted execution must not grow additional legacy authority markers",
        },
        ForbiddenToken {
            family: "multi-path-coordination-patch",
            token: "non_authoritative",
            rationale: "hosted execution must not grow non-authoritative bypass branches",
        },
        ForbiddenToken {
            family: "multi-path-coordination-patch",
            token: "TaskGraphOwner::TaskServiceCompatibility",
            rationale: "task graph ownership must converge on application_execution authority only",
        },
        ForbiddenToken {
            family: "multi-path-coordination-patch",
            token: "TaskGraphOwner::DiagnosticOnly",
            rationale: "diagnostic-only graph owners must not become new execution bypasses",
        },
    ]
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));

    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
        if should_skip_dir(name) {
            continue;
        }
        if path.is_dir() {
            collect_rust_files(&path, files);
            continue;
        }
        if path.extension().and_then(OsStr::to_str) == Some("rs") {
            files.push(path);
        }
    }
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        "target" | "tests" | "fixtures" | "examples" | ".git" | ".playwright-mcp"
    )
}

fn is_approved_migration_surface(relative: &str, token: &ForbiddenToken) -> bool {
    if relative.contains("/tests/")
        || relative.ends_with("_tests.rs")
        || relative.ends_with("tests.rs")
    {
        return true;
    }

    match token.family {
        "application-runtime-direct-start" => {
            relative == "crates/application/macaca-app/src/runtime.rs"
                || relative
                    == "crates/runtime/macaca-runtime-host/src/application_service_provider.rs"
        }
        "web-direct-runtime-field" => {
            // P3 §4.1: deprecated AppState provider fields moved into
            // `WebShellCompositionBundle`; only bootstrap + approved shell
            // adapters may touch legacy anchors or SDK LLM clients.
            relative == "crates/shells/macaca-web/src/lib.rs"
                || relative == "crates/shells/macaca-web/src/state.rs"
                || relative == "crates/shells/macaca-web/src/shell_composition_bundle.rs"
                || relative == "crates/shells/macaca-web/src/application_shell_adapter.rs"
                || relative == "crates/shells/macaca-web/src/llm_route_shell_adapter.rs"
                || relative == "crates/shells/macaca-web/src/mcp_shell_adapter.rs"
                || relative.starts_with("crates/shells/macaca-web/src/framework_runner/")
        }
        "hardcoded-agent-role" => {
            // Existing role-name literals are migration debt recorded by the
            // serviceization audit. The allow rule is intentionally file-level,
            // not directory-level, so a new production module cannot add another
            // role branch without updating OpenSpec and this executable gate.
            matches!(
                relative,
                "crates/application/macaca-app/src/consumption.rs"
                    | "crates/application/macaca-app/src/service_projection.rs"
                    | "crates/application/macaca-app/src/workflow.rs"
                    | "crates/facade/macaca-sdk/src/system_facade.rs"
                    | "crates/foundation/macaca-proto/src/agent_execution_service.rs"
                    | "crates/foundation/macaca-proto/src/orchestration.rs"
                    | "crates/foundation/macaca-proto/src/types.rs"
                    | "crates/runtime/macaca-runtime-host/src/executor/app_executor.rs"
                    | "crates/runtime/macaca-runtime-host/src/executor/bus.rs"
                    | "crates/runtime/macaca-runtime-host/src/executor/callback.rs"
                    | "crates/runtime/macaca-runtime-host/src/executor/event_factory.rs"
                    | "crates/runtime/macaca-runtime-host/src/executor/fork_manager.rs"
                    | "crates/runtime/macaca-runtime-host/src/executor/mod.rs"
                    | "crates/runtime/macaca-runtime-host/src/executor/queue.rs"
                    | "crates/runtime/macaca-runtime-host/src/executor/router.rs"
                    | "crates/kernel/macaca-kernel/src/orchestrator.rs"
                    | "crates/runtime/macaca-framework/src/construction.rs"
                    | "crates/runtime/macaca-runtime-host/src/agent_context_service_provider.rs"
                    | "crates/runtime/macaca-runtime-host/src/agent_execution_service_provider.rs"
                    | "crates/services/macaca-memory/src/core/tests.rs"
                    | "crates/services/macaca-task/src/claim_diagnostics.rs"
                    | "crates/services/macaca-task/src/decompose.rs"
                    | "crates/services/macaca-task/src/dependency.rs"
                    | "crates/services/macaca-task/src/lifecycle.rs"
                    | "crates/services/macaca-task/src/plan_loop.rs"
                    | "crates/services/macaca-task/src/scheduler.rs"
                    | "crates/services/macaca-task/src/todo_board.rs"
                    | "crates/services/macaca-task/src/todo_store.rs"
                    | "crates/services/macaca-tools/src/todo.rs"
                    | "crates/shells/macaca-web/src/capability_catalog.rs"
                    | "crates/shells/macaca-web/src/chat_orchestrator.rs"
                    | "crates/shells/macaca-web/src/framework_runner/build_mode.rs"
                    | "crates/shells/macaca-web/src/framework_runner/sse_emitter_adapter.rs"
                    | "crates/shells/macaca-web/src/framework_toolkit.rs"
                    | "crates/shells/macaca-web/src/loop_manager/mod.rs"
                    | "crates/shells/macaca-web/src/orchestration_tools.rs"
                    | "crates/shells/macaca-web/src/session.rs"
                    | "crates/shells/macaca-web/src/workspace.rs"
                    | "crates/shells/macaca-web/src/workspace_knowledge_digest_capability.rs"
            )
        }
        "provider-model-routing-name" => {
            // Provider/model names are allowed only in the LLM service family,
            // where resolver descriptors, provider adapters, and LLM tests own
            // the routing strategy. This rule is intentionally scoped to the
            // layers called out by the refactor plan: Kernel, Web, and CLI must
            // consume provider-neutral model-selection DTOs instead of adding
            // provider-name or model-prefix branches. Existing application
            // framework and foundation contract literals are handled by later
            // baseline-alignment work, not by this shell/kernel freeze.
            !(relative.starts_with("crates/kernel/")
                || relative.starts_with("crates/shells/macaca-web/")
                || relative.starts_with("crates/shells/macaca-cli/"))
                || relative.starts_with("crates/services/macaca-llm/src/")
        }
        "autonomy-service-boundary" => {
            relative.starts_with("crates/foundation/macaca-proto/src/")
                || relative.starts_with("crates/services/macaca-scheduler/src/")
                || relative.starts_with("crates/services/macaca-heartbeat/src/")
                || relative == "crates/runtime/macaca-runtime-host/src/autonomy_dispatch.rs"
                || relative == "crates/runtime/macaca-runtime-host/src/autonomy_runtime_config.rs"
                || relative == "crates/runtime/macaca-runtime-host/src/autonomy_service_provider.rs"
                || relative == "crates/runtime/macaca-runtime-host/src/autonomy_supervisor.rs"
                || relative
                    .starts_with("crates/runtime/macaca-runtime-host/src/autonomy_supervisor/")
                || relative == "crates/runtime/macaca-runtime-host/src/lib.rs"
                || relative.starts_with("crates/facade/macaca-sdk/src/")
        }
        "autonomy-loop-boundary" => {
            relative == "crates/runtime/macaca-runtime-host/src/autonomy_service_provider.rs"
                || relative == "crates/runtime/macaca-runtime-host/src/autonomy_supervisor.rs"
                || relative
                    .starts_with("crates/runtime/macaca-runtime-host/src/autonomy_supervisor/")
        }
        "provider-compat-construction" => {
            // P2 §3.5–3.6: kernel provider_compat deleted; legacy in-process execution
            // adapters remain only in macaca-agent (definition) and approved migration surfaces.
            relative == "crates/application/macaca-agent/src/execution.rs"
                || relative == "crates/application/macaca-agent/src/lib.rs"
                || relative == "crates/application/macaca-app/src/runtime.rs"
                || relative == "crates/application/macaca-app/src/workflow.rs"
                || relative == "crates/facade/macaca-sdk/src/facade.rs"
                || relative == "crates/facade/macaca-sdk/src/registry_api.rs"
                || relative == "crates/facade/macaca-sdk/src/legacy_kernel_registration.rs"
        }
        "direct-runtime-catalog-read" => {
            relative.starts_with("crates/services/macaca-driver/src/")
                || relative == "crates/runtime/macaca-runtime-host/src/driver_service_provider.rs"
                || relative == "crates/runtime/macaca-runtime-host/src/mcp_service_provider.rs"
                || relative.starts_with("crates/runtime/macaca-runtime-host/src/mcp_runtime.rs")
                || relative.starts_with("crates/services/macaca-tools/src/")
                || relative == "crates/shells/macaca-web/src/mcp_shell_adapter.rs"
        }
        "kernel-non-kernel-module" => relative.starts_with("crates/kernel/macaca-kernel/src/"),
        "multi-path-coordination-patch" => matches!(
            relative,
            "crates/runtime/macaca-runtime-host/src/application_execution_hosted.rs"
                | "crates/runtime/macaca-runtime-host/src/application_execution_hosted_tests.rs"
                | "crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs"
                // P1.2 composed execution backend — patches removed in task 2.6 after audit replay gate.
                | "crates/runtime/macaca-runtime-host/src/agent_execution_orchestration.rs"
                | "crates/shells/macaca-web/src/web_agent_execution_adapters.rs"
                | "crates/shells/macaca-web/src/agent_execution_backend.rs"
                | "crates/shells/macaca-web/src/agent_execution_backend/tests.rs"
                | "crates/shells/macaca-web/src/agent_runner.rs"
                | "crates/shells/macaca-web/src/loop_manager/mod.rs"
                | "crates/services/macaca-task/src/runtime.rs"
                | "crates/services/macaca-task/src/events.rs"
                | "crates/foundation/macaca-proto/src/types.rs"
                | "crates/foundation/macaca-proto/src/application_execution_tests.rs"
        ),
        _ => false,
    }
}

fn is_comment_only_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("*") || trimmed.starts_with("/*")
}

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn scan_file(root: &Path, path: &Path, tokens: &[ForbiddenToken]) -> Vec<Violation> {
    let relative = path
        .strip_prefix(root)
        .expect("scanned file should be under workspace root")
        .to_string_lossy()
        .replace('\\', "/");
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let mut violations = Vec::new();
    let mut pending_cfg_test = false;
    let mut test_module_depth: Option<i32> = None;

    for (index, line) in content.lines().enumerate() {
        // Source files often contain unit-test modules that are not production
        // code even though they live under `src/`. The scanner treats a
        // `#[cfg(test)] mod tests { ... }` block as fixture code so production
        // role freezes do not fail on existing test literals.
        if let Some(depth) = test_module_depth {
            let next_depth = depth + brace_delta(line);
            if next_depth <= 0 {
                test_module_depth = None;
            } else {
                test_module_depth = Some(next_depth);
            }
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("#[cfg(test)]") {
            pending_cfg_test = true;
            continue;
        }
        if pending_cfg_test {
            if trimmed.starts_with("mod tests") || trimmed.starts_with("pub mod tests") {
                let depth = brace_delta(line);
                if depth > 0 {
                    test_module_depth = Some(depth);
                }
                pending_cfg_test = false;
                continue;
            }
            if !trimmed.starts_with("#[") && !trimmed.is_empty() {
                pending_cfg_test = false;
            }
        }

        for token in tokens {
            if !line.contains(token.token)
                || (token.family == "hardcoded-agent-role" && is_comment_only_line(line))
                || is_approved_migration_surface(&relative, token)
            {
                continue;
            }
            violations.push(Violation {
                family: token.family,
                path: PathBuf::from(&relative),
                line: index + 1,
                token: token.token,
                rationale: token.rationale,
            });
        }
    }

    violations
}

fn render_violations(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(|violation| {
            format!(
                "\nfamily={}\nfile={}:{}\ntoken={}\nrationale={}\nprocess=Move the caller behind a service client/facade, or register a time-boxed migration surface through OpenSpec.\n",
                violation.family,
                violation.path.display(),
                violation.line,
                violation.token,
                violation.rationale
            )
        })
        .collect::<Vec<_>>()
        .join("")
}

#[test]
fn serviceization_escape_hatches_reject_new_production_references() {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let tokens = forbidden_tokens();
    let mut files = Vec::new();

    eprintln!(
        "serviceization_escape_hatches event=scan_start root={}",
        crates_root.display()
    );
    collect_rust_files(&crates_root, &mut files);
    files.sort();

    let mut violations = Vec::new();
    for file in &files {
        violations.extend(scan_file(&root, file, &tokens));
    }
    violations.sort();
    eprintln!(
        "serviceization_escape_hatches event=scan_complete files={} violations={}",
        files.len(),
        violations.len()
    );

    assert!(
        violations.is_empty(),
        "Serviceization escape-hatch freeze violations were found:{}",
        render_violations(&violations)
    );
}

#[test]
fn autonomy_schedule_management_uses_serviceized_paths_only() {
    let root = workspace_root();
    let frontend_facade = root
        .parent()
        .expect("workspace has repository parent")
        .join("frontend/lib/autonomy.ts");
    let facade = std::fs::read_to_string(&frontend_facade)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", frontend_facade.display()));
    assert!(
        facade.contains("/autonomy"),
        "frontend autonomy facade must call the serviceized /autonomy namespace"
    );
    assert!(
        !facade.contains("/api/apps/${encodeURIComponent(appId)}/schedules"),
        "frontend autonomy facade must not call the legacy direct schedule namespace"
    );
    assert!(
        !facade.contains("heartbeat_wake"),
        "frontend schedule mutations must not expose heartbeat native cadence as a Scheduler target"
    );
    let schedule_editor = std::fs::read_to_string(
        root.parent()
            .expect("workspace has repository parent")
            .join("frontend/components/autonomy/ScheduleEditorDrawer.tsx"),
    )
    .expect("schedule editor should be readable");
    assert!(
        !schedule_editor.contains("Heartbeat wake")
            && !schedule_editor.contains("wake_scope_key")
            && !schedule_editor.contains("wake_reason_code"),
        "application schedule editor must not expose heartbeat native cadence fields"
    );

    let routes = std::fs::read_to_string(root.join("crates/shells/macaca-web/src/routes.rs"))
        .expect("routes.rs should be readable");
    let serviceized_section = routes
        .split("// Serviceized application autonomy schedule routes")
        .nth(1)
        .and_then(|tail| tail.split("// Event Log API").next())
        .expect("serviceized autonomy schedule section should exist");
    assert!(
        !serviceized_section.contains("macaca_task::TaskScheduler"),
        "serviceized autonomy routes must use Scheduler service clients, not legacy TaskScheduler construction"
    );
}
