//! Kernel microkernel purity gate (P5 §6.1.5).
//!
//! The Macaca kernel must remain a **constitutional** layer: system invariants,
//! scheduling primitives, and protocol-facing facades only. Replaceable provider
//! implementations (LLM, memory, task, driver, gateway, etc.) belong in
//! service-runtime / runtime-host composition roots, not as direct `Cargo.toml`
//! edges on `macaca-kernel`.
//!
//! This gate audits **workspace** direct dependencies only — external crates
//! (`tokio`, `serde`, …) are expected infrastructure. The invariant matches
//! task §3.6.6: `macaca-kernel` may depend on `macaca-proto` and `macaca-ipc`
//! alone among workspace members.
//!
//! Design pattern: **Specification by Example** — `cargo metadata` is the
//! executable contract; failures name the forbidden workspace crate and the
//! service-client replacement boundary.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Workspace crate names permitted as direct production dependencies of `macaca-kernel`.
const PERMITTED_KERNEL_WORKSPACE_DEPS: &[&str] = &["macaca-proto", "macaca-ipc"];

/// External crates that would make the kernel own network transport.
const FORBIDDEN_KERNEL_TRANSPORT_DEPS: &[&str] = &[
    "reqwest",
    "hyper",
    "ureq",
    "surf",
    "isahc",
    "awc",
    "tonic",
    "tokio-tungstenite",
];

/// Source tokens that indicate concrete network transport leaked into kernel production code.
const FORBIDDEN_KERNEL_TRANSPORT_TOKENS: &[&str] = &[
    "WebhookAlertChannel",
    "reqwest::",
    "hyper::",
    "ureq::",
    ".post(",
    "TcpStream",
    "UdpSocket",
    "tokio::net",
    "std::net",
];

/// Source tokens that indicate agent/task orchestration leaked into the kernel.
const FORBIDDEN_KERNEL_ORCHESTRATION_TOKENS: &[&str] = &[
    "AgentOrchestrator",
    "OrchestratorBuilder",
    "OrchestrationCommand",
    "OrchestrationEvent",
    "DelegatedTask",
    "DelegatedTaskResult",
    "AgentRouting",
    "RoutingDecision",
    "delegate_task",
    "aggregate_results",
    "report_to_coordinator",
    "find_best_agent",
    "parse_command",
];

fn workspace_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    panic!("failed to locate Macaca workspace root from integration-tests manifest");
}

fn run_cargo_metadata() -> Value {
    eprintln!(
        "kernel_purity_gate event=metadata_start workspace={}",
        workspace_root().display()
    );
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute cargo metadata for kernel purity gate");
    assert!(
        output.status.success(),
        "cargo metadata failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    eprintln!(
        "kernel_purity_gate event=metadata_complete bytes={}",
        output.stdout.len()
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata should emit valid JSON")
}

/// Returns sorted workspace `macaca-*` crate names that `macaca-kernel` depends on
/// through normal or build production edges (dev-dependencies are ignored).
fn kernel_workspace_dependency_names(metadata: &Value) -> BTreeSet<String> {
    let workspace_members: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members should be an array")
        .iter()
        .map(|id| id.as_str().expect("workspace member id should be a string"))
        .collect();

    let packages = metadata["packages"]
        .as_array()
        .expect("packages should be an array");

    let workspace_names: BTreeSet<&str> = packages
        .iter()
        .filter(|package| workspace_members.contains(package["id"].as_str().unwrap_or_default()))
        .map(|package| {
            package["name"]
                .as_str()
                .expect("package name should be a string")
        })
        .collect();

    let kernel_package = packages
        .iter()
        .find(|package| package["name"].as_str() == Some("macaca-kernel"))
        .expect("macaca-kernel must exist in workspace metadata");

    let mut deps = BTreeSet::new();
    for dependency in kernel_package["dependencies"]
        .as_array()
        .expect("kernel dependencies should be an array")
    {
        let kind = dependency["kind"].as_str();
        if matches!(kind, Some("dev")) {
            continue;
        }
        let name = dependency["name"]
            .as_str()
            .expect("dependency name should be a string");
        if workspace_names.contains(name) {
            deps.insert(name.to_string());
        }
    }
    deps
}

/// Returns sorted forbidden external transport dependency names from `macaca-kernel`.
fn kernel_forbidden_transport_dependency_names(metadata: &Value) -> BTreeSet<String> {
    let packages = metadata["packages"]
        .as_array()
        .expect("packages should be an array");
    let kernel_package = packages
        .iter()
        .find(|package| package["name"].as_str() == Some("macaca-kernel"))
        .expect("macaca-kernel must exist in workspace metadata");

    let forbidden: BTreeSet<&str> = FORBIDDEN_KERNEL_TRANSPORT_DEPS.iter().copied().collect();
    let mut deps = BTreeSet::new();
    for dependency in kernel_package["dependencies"]
        .as_array()
        .expect("kernel dependencies should be an array")
    {
        if matches!(dependency["kind"].as_str(), Some("dev")) {
            continue;
        }
        let name = dependency["name"]
            .as_str()
            .expect("dependency name should be a string");
        if forbidden.contains(name) {
            deps.insert(name.to_string());
        }
    }
    deps
}

/// Scans production kernel source for concrete network transport markers.
///
/// This is intentionally token-based rather than syntax-aware. The gate is a
/// constitutional tripwire: kernel production code must not construct transport
/// clients or embed HTTP/socket senders even if the code is otherwise unused.
fn kernel_forbidden_transport_source_hits() -> Vec<String> {
    kernel_source_hits(FORBIDDEN_KERNEL_TRANSPORT_TOKENS)
}

/// Scans production kernel source for concrete agent/task orchestration markers.
fn kernel_forbidden_orchestration_source_hits() -> Vec<String> {
    kernel_source_hits(FORBIDDEN_KERNEL_ORCHESTRATION_TOKENS)
}

/// Shared source scanner used by constitutional token gates.
fn kernel_source_hits(tokens: &[&str]) -> Vec<String> {
    let kernel_src = workspace_root()
        .join("crates")
        .join("kernel")
        .join("macaca-kernel")
        .join("src");
    let mut stack = vec![kernel_src];
    let mut hits = Vec::new();

    while let Some(path) = stack.pop() {
        let entries = std::fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for entry in entries {
            let entry = entry.expect("kernel source directory entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            for (line_index, line) in content.lines().enumerate() {
                for token in tokens {
                    if line.contains(token) {
                        hits.push(format!(
                            "{}:{} contains forbidden kernel token `{}`",
                            path.strip_prefix(workspace_root())
                                .unwrap_or(&path)
                                .display(),
                            line_index + 1,
                            token
                        ));
                    }
                }
            }
        }
    }
    hits.sort();
    hits
}

/// Main gate entry — panics with replacement guidance when kernel depends on
/// disallowed workspace crates.
pub fn assert_kernel_workspace_dependency_purity() {
    let metadata = run_cargo_metadata();
    let observed = kernel_workspace_dependency_names(&metadata);
    let permitted: BTreeSet<&str> = PERMITTED_KERNEL_WORKSPACE_DEPS.iter().copied().collect();

    eprintln!(
        "kernel_purity_gate event=workspace_deps observed={:?} permitted={:?}",
        observed, permitted
    );

    let mut forbidden: Vec<&str> = observed
        .iter()
        .map(|name| name.as_str())
        .filter(|name| !permitted.contains(name))
        .collect();
    forbidden.sort();

    if forbidden.is_empty() {
        eprintln!("kernel_purity_gate event=pass reason=kernel_workspace_deps_pure");
        return;
    }

    panic!(
        "Kernel purity gate failed: macaca-kernel has forbidden workspace dependencies: {forbidden:?}\n\
         Terminal invariant: kernel may depend only on {PERMITTED_KERNEL_WORKSPACE_DEPS:?}.\n\
         Move provider/runtime coupling behind ServiceRuntime, runtime-host providers, or SDK clients."
    );
}

/// Main network-transport gate — panics when the kernel owns HTTP/socket delivery.
pub fn assert_kernel_has_no_network_transport() {
    let metadata = run_cargo_metadata();
    let forbidden_deps = kernel_forbidden_transport_dependency_names(&metadata);
    let forbidden_source_hits = kernel_forbidden_transport_source_hits();

    eprintln!(
        "kernel_purity_gate event=network_transport observed_deps={:?} source_hits={}",
        forbidden_deps,
        forbidden_source_hits.len()
    );

    if forbidden_deps.is_empty() && forbidden_source_hits.is_empty() {
        eprintln!("kernel_purity_gate event=pass reason=kernel_network_transport_absent");
        return;
    }

    panic!(
        "Kernel network transport gate failed.\n\
         Forbidden dependency hits: {forbidden_deps:?}\n\
         Forbidden source hits:\n{}\n\
         Terminal invariant: kernel may define provider-neutral alert ports, but concrete HTTP/socket delivery must live in runtime-host service providers.",
        forbidden_source_hits.join("\n")
    );
}

/// Main orchestration-semantics gate — panics when kernel owns agent/task routing.
pub fn assert_kernel_has_no_agent_orchestration_semantics() {
    let forbidden_source_hits = kernel_forbidden_orchestration_source_hits();

    eprintln!(
        "kernel_purity_gate event=agent_orchestration source_hits={}",
        forbidden_source_hits.len()
    );

    if forbidden_source_hits.is_empty() {
        eprintln!("kernel_purity_gate event=pass reason=kernel_agent_orchestration_absent");
        return;
    }

    panic!(
        "Kernel agent orchestration gate failed.\n\
         Forbidden source hits:\n{}\n\
         Terminal invariant: kernel may expose service-call primitives and protocol DTOs, but task delegation, agent routing, result aggregation, and tool-command parsing must live in services or runtime-host providers.",
        forbidden_source_hits.join("\n")
    );
}
