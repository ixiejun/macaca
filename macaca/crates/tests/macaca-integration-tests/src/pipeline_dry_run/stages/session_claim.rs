//! Stage 2: session-scoped claim isolation.
//!
//! Verifies that two [`TaskSpace`] instances with different session keys each claim
//! only their own pending tasks on a shared [`TodoStore`].

use std::sync::Arc;

use macaca_proto::types::ApplicationId;
use macaca_task::{TaskBoard, TaskSpace, TodoStore};
use tracing::info;

use crate::pipeline_dry_run::config::PipelineDryRunConfig;
use crate::pipeline_dry_run::fixtures::{ASSIGNEE_AGENT, COORDINATOR_AGENT};
use crate::pipeline_dry_run::report::{ok, record};
use crate::pipeline_dry_run::report::PipelineReport;
use crate::pipeline_dry_run::trace::{trace, trace_stage, trace_stage_end};

/// Run session-scoped claim isolation and record the outcome.
pub async fn run(
    config: PipelineDryRunConfig,
    app_id: ApplicationId,
    store: Arc<TodoStore>,
    report: &mut PipelineReport,
) {
    trace_stage(config, "2/5 session_scoped_claim_isolation");
    info!(target: "pipeline_dry_run", "stage_session_claim_start");

    let res = async {
        let sa = TaskSpace::for_session(app_id, Some("sess-a".into()), store.clone());
        let sb = TaskSpace::for_session(app_id, Some("sess-b".into()), store.clone());
        let ta = sa
            .create_task_assignment(
                ASSIGNEE_AGENT,
                COORDINATOR_AGENT,
                "A",
                "da",
                vec![],
                5,
                vec![],
                None,
            )
            .await;
        let tb = sb
            .create_task_assignment(
                ASSIGNEE_AGENT,
                COORDINATOR_AGENT,
                "B",
                "db",
                vec![],
                5,
                vec![],
                None,
            )
            .await;
        trace(
            config,
            format!(
                "  TaskSpace.create_and_assign | sess-a | task_id={} title=A status={:?}",
                ta.id, ta.status
            ),
        );
        trace(
            config,
            format!(
                "  TaskSpace.create_and_assign | sess-b | task_id={} title=B status={:?}",
                tb.id, tb.status
            ),
        );

        let board_a =
            TaskBoard::for_agent(app_id, ASSIGNEE_AGENT, Some("sess-a".into()), store.clone());
        let board_b =
            TaskBoard::for_agent(app_id, ASSIGNEE_AGENT, Some("sess-b".into()), store.clone());

        let ca = board_a
            .claim_next_task()
            .await
            .ok_or_else(|| "session A: expected claim".to_string())?;
        let cb = board_b
            .claim_next_task()
            .await
            .ok_or_else(|| "session B: expected claim".to_string())?;
        trace(
            config,
            format!(
                "  TaskBoard.claim_next(sess-a) -> task_id={} status_after={:?}",
                ca.id, ca.status
            ),
        );
        trace(
            config,
            format!(
                "  TaskBoard.claim_next(sess-b) -> task_id={} status_after={:?}",
                cb.id, cb.status
            ),
        );
        if ca.id != ta.id {
            return Err(format!(
                "session A claimed wrong task: {:?} vs {:?}",
                ca.id, ta.id
            ));
        }
        if cb.id != tb.id {
            return Err(format!(
                "session B claimed wrong task: {:?} vs {:?}",
                cb.id, tb.id
            ));
        }
        ok()
    }
    .await;

    trace_stage_end(config, res.is_ok(), None);
    record("session_scoped_claim_isolation", res, report);
    info!(target: "pipeline_dry_run", "stage_session_claim_end");
}
