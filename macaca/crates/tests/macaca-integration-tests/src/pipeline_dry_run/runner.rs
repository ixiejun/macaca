//! Orchestrates the five pipeline dry-run stages in sequence.
//!
//! **Design pattern:** Template Method — `run_full_pipeline_dry_run_with_config` defines
//! the stage skeleton; individual stage modules supply step implementations.

use std::sync::Arc;

use macaca_proto::types::ApplicationId;
use macaca_task::TodoStore;
use tracing::info;

use super::config::PipelineDryRunConfig;
use super::report::{ok, record, PipelineReport};
use super::stages::{
    agentic_create, agentic_worker_review, depends_on_review, session_claim,
};
use super::store::open_temp_store;
use super::trace::{trace, trace_stage, trace_stage_end};

/// Run all stages with default (non-verbose) configuration.
pub async fn run_full_pipeline_dry_run() -> PipelineReport {
    run_full_pipeline_dry_run_with_config(PipelineDryRunConfig::default()).await
}

/// Run all stages sequentially; failures are recorded but later stages still execute
/// so a single run can surface multiple breakage points.
pub async fn run_full_pipeline_dry_run_with_config(config: PipelineDryRunConfig) -> PipelineReport {
    let mut report = PipelineReport { stages: vec![] };

    if config.verbose {
        eprintln!();
        eprintln!("========== pipeline dry-run (no real LLM) ==========");
    }
    info!(target: "pipeline_dry_run", verbose = config.verbose, "pipeline_start");

    // Stage 1: open ephemeral redb + TodoStore
    let (dir, store) = match open_temp_store() {
        Ok(x) => x,
        Err(e) => {
            record("redb_todo_store_open", Err(e.clone()), &mut report);
            trace_stage_end(config, false, Some(&e));
            info!(target: "pipeline_dry_run", error = %e, "pipeline_abort_store_open");
            return report;
        }
    };
    trace_stage(config, "1/5 redb + TodoStore");
    trace(
        config,
        format!(
            "  db_path = {}",
            dir.path().join("pipeline_dry_run.redb").display()
        ),
    );
    record("redb_todo_store_open", ok(), &mut report);
    trace_stage_end(config, true, Some("opened embedded redb, TodoStore ready"));
    let _keep = dir;

    let app_id = ApplicationId::new();
    trace(
        config,
        format!("  application_id = {app_id} (all stages share this app)"),
    );
    let store: Arc<TodoStore> = store;

    session_claim::run(config, app_id, store.clone(), &mut report).await;
    depends_on_review::run(config, app_id, store.clone(), &mut report).await;
    agentic_create::run(config, app_id, store.clone(), &mut report).await;
    agentic_worker_review::run(config, app_id, store, &mut report).await;

    if config.verbose {
        eprintln!();
        eprintln!("========== stage summary ==========");
        eprintln!(
            "{}",
            PipelineReport {
                stages: report.stages.clone()
            }
            .summary()
        );
        eprintln!("====================================");
    }

    info!(
        target: "pipeline_dry_run",
        stage_count = report.stages.len(),
        all_ok = report.stages.iter().all(|(_, r)| r.is_ok()),
        "pipeline_end"
    );

    report
}
