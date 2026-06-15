//! protocol microkernel 阶段 0 基线测试。
//!
//! 这些测试不依赖真实 LLM、浏览器、前端服务或外部网络。它们验证两件事：
//! 1. 当前 no-network autonomous pipeline 仍然可跑。
//! 2. protocol microkernel 治理文档没有丢失后续阶段必须保护的边界和回归场景。

use std::path::{Path, PathBuf};

use macaca_integration_tests::pipeline_dry_run;

fn repo_root() -> PathBuf {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("[workspace]") {
                return ancestor
                    .parent()
                    .expect("workspace root should have a repository parent")
                    .to_path_buf();
            }
        }
    }
    panic!("failed to locate repository root from CARGO_MANIFEST_DIR")
}

fn read_repo_file(relative_path: &str) -> String {
    let path = repo_root().join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn assert_contains_all(document_name: &str, content: &str, required: &[&str]) {
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|needle| !content.contains(needle))
        .collect();
    assert!(
        missing.is_empty(),
        "{document_name} is missing required protocol microkernel baseline markers: {missing:?}"
    );
}

#[tokio::test]
async fn protocol_service_baseline_no_network_pipeline_still_passes() {
    let report = pipeline_dry_run::run_full_pipeline_dry_run().await;
    report.assert_all_ok();
}

#[test]
fn protocol_service_governance_docs_cover_required_boundaries() {
    let boundaries = read_repo_file("macaca/docs/macaca-os-microkernel-boundaries.md");
    assert_contains_all(
        "macaca-os-microkernel-boundaries.md",
        &boundaries,
        &[
            "Identity",
            "Scheduler primitive",
            "Capability registry",
            "Service registry",
            "IPC and service-call facade",
            "Policy facade",
            "Trace and audit bus",
            "Resource manager facade",
            "Session primitive",
            "Task primitive",
            "Package runtime guard",
            "LLM providers",
            "Driver runtimes",
            "Skill runtimes",
            "MCP protocol runtimes",
            "Gateway adapters",
            "Store",
            "Payment",
            "Web3, wallets, chain RPC, EVM, DApp execution",
            "Application-specific",
            "thin shells",
        ],
    );

    let governance = read_repo_file("macaca/docs/macaca-os-architecture-governance.md");
    assert_contains_all(
        "macaca-os-architecture-governance.md",
        &governance,
        &[
            "Microkernel + Service Runtime + Application ABI + Plugin/Module Ecosystem",
            "Required Design Patterns",
            "Acceptance Gates",
            "Service Rules",
            "SDK/SystemFacade Rules",
            "Runtime Host Rules",
            "Application Rules",
            "Shell Rules",
            "Security And Audit Rules",
            "Trace must exist",
            "Policy must run",
        ],
    );
}

#[test]
fn protocol_service_regression_matrix_covers_existing_user_visible_flows() {
    let matrix = read_repo_file("macaca/docs/route-c-regression-matrix.md");
    assert_contains_all(
        "route-c-regression-matrix.md",
        &matrix,
        &[
            "YAML application 加载",
            "/api/chat/v2",
            "goal -> planner -> task -> worker -> review -> coordinator resume",
            "trace 实时推送",
            "trace 历史恢复",
            "task board session-scoped fetch",
            "driver execution trace",
            "skill/MCP runtime smoke path",
            "frontend/backend 重启后 session 恢复",
            "无网络 LLM pipeline dry run",
        ],
    );
}

#[test]
fn protocol_service_phase_template_enforces_required_workflow() {
    let template = read_repo_file("macaca/docs/route-c-phase-template.md");
    assert_contains_all(
        "route-c-phase-template.md",
        &template,
        &[
            "Superpowers brainstorm",
            "OpenSpec proposal/design/tasks/spec",
            "GitNexus impact",
            "additive-first",
            "targeted tests",
            "integration smoke",
            "detect_changes",
            "commit",
        ],
    );
}
