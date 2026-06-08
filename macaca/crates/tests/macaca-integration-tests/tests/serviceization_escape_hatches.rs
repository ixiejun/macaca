//! Static serviceization escape-hatch gate.
//!
//! This integration test is an executable specification for the freeze-first
//! refactor plan. It prevents new production Rust code from growing direct
//! runtime/provider access while owners migrate callers to service clients.
//!
//! Two scan modes (Strategy):
//! - **Freeze mode** (`honor_migration_surfaces = true`): blocks new violations
//!   outside approved migration surfaces (P0–P4 default).
//! - **Debt inventory mode** (`honor_migration_surfaces = false`): counts every
//!   raw hit for baseline tracking toward P5 §6.2 terminal zero-debt state.

#[path = "serviceization_escape_hatches/migration_debt_baseline.rs"]
mod migration_debt_baseline;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// A source token that must not appear in production Rust code unless the file
/// is an approved migration surface, test, fixture, or service provider bridge.
struct ForbiddenToken {
    family: &'static str,
    token: &'static str,
    rationale: &'static str,
}

/// Controls whether approved migration surfaces suppress token hits during scan.
#[derive(Debug, Clone, Copy)]
struct ScanOptions {
    /// When true, hits inside `is_approved_migration_surface` are ignored (freeze gate).
    /// When false, every production hit is recorded (debt inventory gate).
    honor_migration_surfaces: bool,
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

/// Return the repository root that contains both `macaca/` and optional `frontend/`.
///
/// The Macaca Cargo workspace lives under `macaca/`, while the Next.js shell is a
/// sibling checkout. Partial worktrees (backend-only CI agents) may omit `frontend/`;
/// gates that reference presentation sources must tolerate that absence.
fn repository_root() -> PathBuf {
    workspace_root()
        .parent()
        .expect("Macaca workspace should live under the repository root")
        .to_path_buf()
}

/// Read a presentation source file when the checkout includes it.
///
/// Returns `None` when the path is absent so contract tests can enforce backend
/// invariants in every environment while still validating frontend adapters when
/// the full monorepo is present.
fn read_optional_presentation_source(path: &Path) -> Option<String> {
    if !path.exists() {
        eprintln!(
            "serviceization_escape_hatches event=skip_missing_presentation_source path={}",
            path.display()
        );
        return None;
    }
    Some(
        std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display())),
    )
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
            rationale: "local scheduler engines must remain replaceable service providers (retired name; use InProcessSchedulerProvider in approved surfaces)",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "LocalHeartbeatProvider",
            rationale: "local heartbeat engines must remain replaceable service providers (retired name; use InProcessHeartbeatProvider in approved surfaces)",
        },
        ForbiddenToken {
            family: "autonomy-service-boundary",
            token: "AutonomySupervisor",
            rationale: "autonomy loops must remain lifecycle-managed runtime-host infrastructure (retired name; use AutonomyLifecycleCoordinator)",
        },
        // Service id string literals (`"service.scheduler"` / `"service.heartbeat"`) were
        // removed from this family in iteration 50. Proto owns canonical constants;
        // growth outside proto is guarded by `autonomy_service_id_literals_absent_outside_proto`.
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

/// Returns true when a Rust path is test/fixture code even if it lives under `src/`.
///
/// Inline `#[cfg(test)]` modules are handled separately inside `scan_file`; this helper
/// covers standalone contract-test sources (`*_tests.rs`, `tests.rs`) that must not be
/// counted as production for terminal reconciliation-marker assertions.
fn is_non_production_rust_source(relative: &str) -> bool {
    relative.contains("/tests/")
        || relative.ends_with("_tests.rs")
        || relative.ends_with("tests.rs")
        || relative.ends_with("/serviceization_escape_hatches.rs")
}

fn is_approved_migration_surface(relative: &str, token: &ForbiddenToken) -> bool {
    if relative.contains("/tests/")
        || relative.ends_with("_tests.rs")
        || relative.ends_with("tests.rs")
    {
        return true;
    }

    match token.family {
        // `application-runtime-direct-start` family retired in iteration 43 — no migration surfaces remain.
        // `web-direct-runtime-field` family retired in iteration 45 — no migration surfaces remain.
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
                    // route_chat_v2 / framework_runner SSE adapters retired in iteration 51.
                    | "crates/shells/macaca-web/src/framework_toolkit/mod.rs"
                    | "crates/shells/macaca-web/src/framework_toolkit/builder.rs"
                    | "crates/shells/macaca-web/src/loop_manager/mod.rs"
                    | "crates/shells/macaca-web/src/orchestration_tools.rs"
                    | "crates/shells/macaca-web/src/session/mod.rs"
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
        // `autonomy-service-boundary` family retired in iteration 50 — no migration surfaces remain.
        // `autonomy-loop-boundary` family retired in iteration 44 — no migration surfaces remain.
        // `provider-compat-construction` family retired in iteration 47 — no migration surfaces remain.
        // `direct-runtime-catalog-read` family retired in iteration 46 — no migration surfaces remain.
        "kernel-non-kernel-module" => relative.starts_with("crates/kernel/macaca-kernel/src/"),
        // `multi-path-coordination-patch` family retired in iteration 42 — no migration surfaces remain.
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

fn violation_fingerprint(violation: &Violation) -> String {
    format!(
        "{}|{}|{}",
        violation.family,
        violation.path.display(),
        violation.token
    )
}

/// Scan all production Rust files under `crates/` and return sorted violations.
fn collect_production_violations(options: ScanOptions) -> Vec<Violation> {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let tokens = forbidden_tokens();
    let mut files = Vec::new();

    eprintln!(
        "serviceization_escape_hatches event=scan_start honor_migration_surfaces={} root={}",
        options.honor_migration_surfaces,
        crates_root.display()
    );
    collect_rust_files(&crates_root, &mut files);
    files.sort();

    let mut violations = Vec::new();
    for file in &files {
        violations.extend(scan_file(
            &root,
            file,
            &tokens,
            options.honor_migration_surfaces,
        ));
    }
    violations.sort();
    eprintln!(
        "serviceization_escape_hatches event=scan_complete files={} violations={}",
        files.len(),
        violations.len()
    );
    violations
}

fn scan_file(
    root: &Path,
    path: &Path,
    tokens: &[ForbiddenToken],
    honor_migration_surfaces: bool,
) -> Vec<Violation> {
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
                || (honor_migration_surfaces && is_approved_migration_surface(&relative, token))
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
    let violations = collect_production_violations(ScanOptions {
        honor_migration_surfaces: true,
    });

    assert!(
        violations.is_empty(),
        "Serviceization escape-hatch freeze violations were found:{}",
        render_violations(&violations)
    );
}

/// Ignored helper — run with `cargo test -p macaca-integration-tests dump_escape_hatch_raw_fingerprints -- --ignored --nocapture`
/// when `migration_debt_baseline.rs` must be regenerated after deliberate surface retirement.
#[test]
#[ignore = "baseline regeneration helper only"]
fn dump_escape_hatch_raw_fingerprints() {
    let violations = collect_production_violations(ScanOptions {
        honor_migration_surfaces: false,
    });
    eprintln!(
        "serviceization_escape_hatches event=dump_raw_inventory count={}",
        violations.len()
    );
    for violation in &violations {
        eprintln!("{}", violation_fingerprint(violation));
    }
}

#[test]
fn serviceization_escape_hatches_migration_debt_inventory_matches_baseline() {
    let violations = collect_production_violations(ScanOptions {
        honor_migration_surfaces: false,
    });

    assert_eq!(
        violations.len(),
        migration_debt_baseline::EXPECTED_RAW_VIOLATION_COUNT,
        "Raw escape-hatch violation count changed (honor_migration_surfaces=false). \
         Update migration_debt_baseline.rs after OpenSpec-approved surface retirement.{}",
        render_violations(&violations)
    );

    let mut observed_by_family: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();
    for violation in &violations {
        *observed_by_family.entry(violation.family).or_insert(0) += 1;
    }
    for (family, expected_count) in migration_debt_baseline::EXPECTED_RAW_VIOLATION_BY_FAMILY {
        let observed = observed_by_family.get(family).copied().unwrap_or(0);
        assert_eq!(
            observed, *expected_count,
            "Raw escape-hatch family debt changed for {family}: observed={observed} expected={expected_count}. \
             Update migration_debt_baseline.rs after OpenSpec-approved retirement."
        );
    }
}

/// Hard assertion helper for retired escape-hatch families (Strangler Fig terminal state).
///
/// Retired families keep forbidden-token definitions for freeze-mode growth detection,
/// but production `src/` must not contain the legacy literals even when migration surfaces
/// Assert specific retired forbidden tokens are absent from all production code.
///
/// Used when a family is retired in sub-phases: only a subset of tokens is renamed
/// while the family migration surfaces remain for other tokens.
fn assert_retired_escape_hatch_tokens_absent_in_production(tokens: &[&str]) {
    assert_retired_escape_hatch_tokens_absent_in_production_with_allowed_paths(tokens, &[]);
}

/// Scans production Rust sources for exact substring literals outside approved
/// path prefixes.
///
/// Used when retired literals are no longer listed in [`forbidden_tokens`] (e.g.
/// service id strings owned exclusively by proto constants) but must still be
/// prevented from reappearing in dispatch/runtime layers.
fn assert_production_literal_tokens_absent_outside_allowed_paths(
    literals: &[&str],
    allowed_path_prefixes: &[&str],
) {
    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let mut hits = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&root)
            .expect("scanned file should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        if is_non_production_rust_source(&relative) {
            continue;
        }
        if allowed_path_prefixes
            .iter()
            .any(|prefix| relative.starts_with(prefix))
        {
            continue;
        }
        let content = std::fs::read_to_string(file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for (index, line) in content.lines().enumerate() {
            for literal in literals {
                if line.contains(literal) {
                    hits.push(format!(
                        "\nfamily=literal-guard\nfile={}:{}\ntoken={}\n",
                        relative,
                        index + 1,
                        literal
                    ));
                }
            }
        }
    }
    hits.sort();

    assert!(
        hits.is_empty(),
        "Production literal tokens {:?} must be absent outside {:?}:{}",
        literals,
        allowed_path_prefixes,
        hits.join("")
    );
}

/// Assert specific production source paths contain no forbidden literal tokens.
///
/// Used for sub-phase retirement of `hardcoded-agent-role` debt where only a
/// handful of modules were cleaned while the family migration surface remains.
fn assert_production_paths_literal_tokens_absent(relative_paths: &[&str], literals: &[&str]) {
    let root = workspace_root();
    let mut hits = Vec::new();

    for relative in relative_paths {
        let path = root.join(relative);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for (index, line) in content.lines().enumerate() {
            for literal in literals {
                if line.contains(literal) {
                    hits.push(format!(
                        "\nfamily=literal-guard\nfile={}:{}\ntoken={}\n",
                        relative,
                        index + 1,
                        literal
                    ));
                }
            }
        }
    }
    hits.sort();

    assert!(
        hits.is_empty(),
        "Production paths {:?} must not contain literal tokens {:?}:{}",
        relative_paths,
        literals,
        hits.join("")
    );
}

/// Like [`assert_retired_escape_hatch_tokens_absent_in_production`], but permits
/// literal hits under approved path prefixes (e.g. proto constant definitions).
fn assert_retired_escape_hatch_tokens_absent_in_production_with_allowed_paths(
    tokens: &[&str],
    allowed_path_prefixes: &[&str],
) {
    let retired_tokens = forbidden_tokens()
        .into_iter()
        .filter(|entry| tokens.contains(&entry.token))
        .collect::<Vec<_>>();
    assert!(
        !retired_tokens.is_empty(),
        "retired token list must match at least one forbidden token: {tokens:?}"
    );

    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let mut violations = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&root)
            .expect("scanned file should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        if is_non_production_rust_source(&relative) {
            continue;
        }
        if allowed_path_prefixes
            .iter()
            .any(|prefix| relative.starts_with(prefix))
        {
            continue;
        }
        violations.extend(scan_file(&root, file, &retired_tokens, false));
    }
    violations.sort();

    assert!(
        violations.is_empty(),
        "Retired escape-hatch tokens {:?} must be absent from production code outside {:?} \
         (no migration exemption):{}",
        tokens,
        allowed_path_prefixes,
        render_violations(&violations)
    );
}

/// Assert every forbidden token in a retired family is absent from production code.
///
/// Retired families keep token definitions for freeze-mode growth detection, but
/// production `src/` must not contain legacy literals even when migration surfaces
/// were removed — otherwise debt would silently reappear outside runtime-host ownership.
fn assert_retired_escape_hatch_family_absent_in_production(family: &str) {
    let retired_tokens = forbidden_tokens()
        .into_iter()
        .filter(|token| token.family == family)
        .collect::<Vec<_>>();

    let root = workspace_root();
    let crates_root = root.join("crates");
    let mut files = Vec::new();
    collect_rust_files(&crates_root, &mut files);

    let mut violations = Vec::new();
    for file in &files {
        let relative = file
            .strip_prefix(&root)
            .expect("scanned file should be under workspace root")
            .to_string_lossy()
            .replace('\\', "/");
        if is_non_production_rust_source(&relative) {
            continue;
        }
        violations.extend(scan_file(&root, file, &retired_tokens, false));
    }
    violations.sort();

    assert!(
        violations.is_empty(),
        "Retired escape-hatch family `{family}` must be absent from all production code \
         (no migration exemption):{}",
        render_violations(&violations)
    );
}

#[test]
fn serviceization_escape_hatches_reconciliation_markers_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("multi-path-coordination-patch");
}

#[test]
fn serviceization_escape_hatches_autonomy_loop_boundary_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("autonomy-loop-boundary");
}

#[test]
fn serviceization_escape_hatches_web_direct_runtime_field_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("web-direct-runtime-field");
}

#[test]
fn serviceization_escape_hatches_direct_runtime_catalog_read_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("direct-runtime-catalog-read");
}

#[test]
fn serviceization_escape_hatches_provider_compat_construction_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("provider-compat-construction");
}

/// Sub-phase of `autonomy-service-boundary` (iteration 48): legacy `Local*Provider`
/// symbols were renamed to provider-neutral `InProcess*Provider`. The old names must
/// not reappear outside migration surfaces.
#[test]
fn serviceization_escape_hatches_local_autonomy_providers_absent_in_production() {
    assert_retired_escape_hatch_tokens_absent_in_production(&[
        "LocalSchedulerProvider",
        "LocalHeartbeatProvider",
    ]);
}

/// Sub-phase of `autonomy-service-boundary` (iteration 49): legacy host adapter
/// symbols were renamed to provider-neutral `Host*ServiceAdapter`. The old names
/// must not reappear outside migration surfaces.
#[test]
fn serviceization_escape_hatches_autonomy_host_adapters_absent_in_production() {
    assert_retired_escape_hatch_tokens_absent_in_production(&[
        "SchedulerSystemServiceProvider",
        "HeartbeatSystemServiceProvider",
    ]);
}

/// Sub-phase of `autonomy-service-boundary` (iteration 49): `AutonomySupervisor`
/// was renamed to lifecycle-neutral `AutonomyLifecycleCoordinator`.
#[test]
fn serviceization_escape_hatches_autonomy_supervisor_absent_in_production() {
    assert_retired_escape_hatch_tokens_absent_in_production(&["AutonomySupervisor"]);
}

/// Sub-phase of `autonomy-service-boundary` (iteration 49): scheduler/heartbeat
/// service id literals must flow through proto constants, not ad-hoc strings.
#[test]
fn serviceization_escape_hatches_autonomy_service_id_literals_absent_outside_proto() {
    // Proto owns the canonical service id string constants; all other production
    // callers must reference SCHEDULER_SERVICE_ID / HEARTBEAT_SERVICE_ID instead.
    // These literals were removed from `forbidden_tokens` in iteration 50, so the
    // guard uses direct literal scanning instead of the retired-token helper.
    assert_production_literal_tokens_absent_outside_allowed_paths(
        &["\"service.scheduler\"", "\"service.heartbeat\""],
        &["crates/foundation/macaca-proto/src/"],
    );
}

/// Terminal state for `autonomy-service-boundary` (iteration 50): all legacy
/// autonomy composition symbols and service id literals are retired; production
/// must use `Host*ServiceAdapter`, `InProcess*Provider`, `AutonomyLifecycleCoordinator`,
/// and proto `*_SERVICE_ID` constants only.
#[test]
fn serviceization_escape_hatches_autonomy_service_boundary_absent_in_production() {
    assert_retired_escape_hatch_family_absent_in_production("autonomy-service-boundary");
}

/// Sub-phase of `hardcoded-agent-role` (iteration 51): web framework chat/SSE
/// paths must resolve entry agents from manifest/service views — never shell
/// defaults like `"coordinator"`.
#[test]
fn serviceization_escape_hatches_web_framework_runner_coordinator_literal_absent() {
    assert_production_paths_literal_tokens_absent(
        &[
            "crates/shells/macaca-web/src/chat_orchestrator/route_chat_v2.rs",
            "crates/shells/macaca-web/src/framework_runner/build_mode.rs",
            "crates/shells/macaca-web/src/framework_runner/sse_emitter_adapter.rs",
        ],
        &["\"coordinator\""],
    );
}

#[test]
fn autonomy_schedule_management_uses_serviceized_paths_only() {
    let root = workspace_root();
    let repo = repository_root();
    let frontend_facade = repo.join("frontend/lib/autonomy.ts");
    if let Some(facade) = read_optional_presentation_source(&frontend_facade) {
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
    }

    let schedule_editor_path =
        repo.join("frontend/components/autonomy/ScheduleEditorDrawer.tsx");
    if let Some(schedule_editor) = read_optional_presentation_source(&schedule_editor_path) {
        assert!(
            !schedule_editor.contains("Heartbeat wake")
                && !schedule_editor.contains("wake_scope_key")
                && !schedule_editor.contains("wake_reason_code"),
            "application schedule editor must not expose heartbeat native cadence fields"
        );
    }

    let routes = std::fs::read_to_string(
        root.join("crates/shells/macaca-web/src/routes/autonomy_schedules.rs"),
    )
    .expect("routes/autonomy_schedules.rs should be readable");
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
