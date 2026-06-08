//! Stage 3: depends_on + review unblocks dependents.
//!
//! Exercises blocked-task semantics: a dependent stays `Blocked` until its predecessor
//! completes review, then becomes claimable.

use std::sync::Arc;

use macaca_proto::types::{ApplicationId, TodoReviewResult, TodoStatus};
use macaca_task::{TaskBoard, TaskSpace, TodoStore};
use tracing::info;

use crate::pipeline_dry_run::config::PipelineDryRunConfig;
use crate::pipeline_dry_run::fixtures::{ASSIGNEE_AGENT, COORDINATOR_AGENT};
use crate::pipeline_dry_run::report::{err_msg, ok, record, PipelineReport};
use crate::pipeline_dry_run::trace::{trace, trace_stage, trace_stage_end};

/// Run depends-on + review unblock flow and record the outcome.
pub async fn run(
    config: PipelineDryRunConfig,
    app_id: ApplicationId,
    store: Arc<TodoStore>,
    report: &mut PipelineReport,
) {
    trace_stage(config, "3/5 depends_on + review_unblocks_next");
    info!(target: "pipeline_dry_run", "stage_depends_on_review_start");

    let res = async {
        let space = TaskSpace::for_session(app_id, Some("sess-dep".into()), store.clone());
        let t1 = space
            .create_task_assignment(
                ASSIGNEE_AGENT,
                COORDINATOR_AGENT,
                "first",
                "d1",
                vec![],
                5,
                vec![],
                None,
            )
            .await;
        let t2 = space
            .create_task_assignment(
                ASSIGNEE_AGENT,
                COORDINATOR_AGENT,
                "second",
                "d2",
                vec![],
                5,
                vec![t1.id],
                None,
            )
            .await;
        trace(
            config,
            format!(
                "  created t1 id={} status={:?} | t2 id={} status={:?} depends_on=[{}]",
                t1.id, t1.status, t2.id, t2.status, t1.id
            ),
        );
        if t2.status != TodoStatus::Blocked {
            return Err(format!("expected second task Blocked, got {:?}", t2.status));
        }
        trace(config, "  (t2 correctly Blocked until t1 completes)");

        let board =
            TaskBoard::for_agent(app_id, ASSIGNEE_AGENT, Some("sess-dep".into()), store.clone());
        let c = board
            .claim_next_task()
            .await
            .ok_or_else(|| "claim first task".to_string())?;
        trace(
            config,
            format!("  claim_next -> {} status={:?}", c.id, c.status),
        );
        if c.id != t1.id {
            return err_msg("claimed task should be t1");
        }
        if !board.mark_task_in_progress(&t1.id).await {
            return err_msg("start_task t1");
        }
        trace(config, "  start_task(t1) -> InProgress");
        if !board
            .submit_task_for_review(&t1.id, "done summary".into())
            .await
        {
            return err_msg("submit_for_review t1");
        }
        trace(config, "  submit_for_review(t1) -> PendingReview");

        let ok_review = space
            .apply_review_result(
                &t1.id,
                ASSIGNEE_AGENT,
                TodoReviewResult {
                    passed: true,
                    feedback: "ok".into(),
                    verified_criteria: vec![],
                },
            )
            .await;
        if !ok_review {
            return err_msg("review_task returned false");
        }
        trace(
            config,
            "  review_task(passed=true) -> t1 Completed, dependents unblocked",
        );

        let t2_after = store
            .get_todo(&app_id, &Some("sess-dep".into()), ASSIGNEE_AGENT, &t2.id)
            .await
            .ok_or_else(|| "reload t2".to_string())?;
        trace(
            config,
            format!("  reload t2 -> status={:?}", t2_after.status),
        );
        if t2_after.status != TodoStatus::Pending {
            return Err(format!(
                "after t1 completed, t2 should be Pending, got {:?}",
                t2_after.status
            ));
        }

        let c2 = board
            .claim_next_task()
            .await
            .ok_or_else(|| "claim second task after unblock".to_string())?;
        trace(config, format!("  claim_next -> {} (expect t2)", c2.id));
        if c2.id != t2.id {
            return err_msg("second claim should be t2");
        }
        ok()
    }
    .await;

    trace_stage_end(config, res.is_ok(), None);
    record("depends_on_review_unblocks_next", res, report);
    info!(target: "pipeline_dry_run", "stage_depends_on_review_end");
}
