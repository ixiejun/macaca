//! Service-owned fallback decomposition planning.
//!
//! This module intentionally contains the conservative fallback Strategy that
//! used to live in the Web shell.  The Strategy consumes provider-neutral
//! capability descriptors and produces task assignment specs; it does not call
//! LLM providers, mutate task stores, inspect application-specific role names,
//! or create task-board records.  Callers must persist returned assignments via
//! the traced `task.create_assignment` command so every side effect remains
//! auditable.

use std::collections::HashSet;

use macaca_proto::{
    BuildFallbackDecompositionCommand, FallbackDecompositionPlan, FallbackTaskAssignmentSpec,
    FallbackTaskPhase,
};
use tracing::info;

use super::TaskServiceRuntime;

impl<S> TaskServiceRuntime<S>
where
    S: super::TaskServiceExecutionStrategy + 'static,
{
    /// Build a fallback decomposition plan from declarative worker capability profiles.
    ///
    /// The command is a pure query-style service command. It exists so Web and
    /// CLI can stay thin adapters even when planner delegation fails. The
    /// returned plan is deterministic: workers are ordered by phase and then by
    /// stable agent name, truncated to five assignments, and chained by
    /// dependency in that order.
    pub async fn build_fallback_decomposition(
        &self,
        command: BuildFallbackDecompositionCommand,
    ) -> Result<FallbackDecompositionPlan, String> {
        let mut ordered = command.workers;
        ordered.sort_by_key(|worker| {
            (
                fallback_phase_for(&worker.capabilities),
                worker.name.clone(),
            )
        });
        ordered.truncate(5);

        let mut next_dependency = command.initial_dependency;
        let mut assignments = Vec::new();
        let mut seen_agents = HashSet::new();
        for (index, worker) in ordered.into_iter().enumerate() {
            if worker.name.trim().is_empty() || !seen_agents.insert(worker.name.clone()) {
                continue;
            }
            let phase = fallback_phase_for(&worker.capabilities);
            let (title, description, acceptance_criteria) =
                fallback_task_template(phase, &worker.name, &command.goal_description);
            let depends_on = next_dependency.into_iter().collect::<Vec<_>>();
            assignments.push(FallbackTaskAssignmentSpec {
                agent_name: worker.name,
                title,
                description,
                acceptance_criteria,
                priority: 8u8.saturating_sub(index.min(3) as u8),
                depends_on,
                phase,
            });
            // Only the first plan entry can reference a pre-existing task id.
            // Subsequent task-to-task dependencies are assigned by the caller
            // after each audited `task.create_assignment` command returns a
            // concrete task id.
            next_dependency = None;
        }

        info!(
            app_id = %command.app_id.0,
            session_id = ?command.session_id,
            goal_id = %command.goal_id,
            assignment_count = assignments.len(),
            trace_id = command
                .trace
                .as_ref()
                .map(|trace| trace.trace_id.as_str())
                .unwrap_or("none"),
            "task service built fallback decomposition plan"
        );

        Ok(FallbackDecompositionPlan { assignments })
    }
}

/// Classify a worker from declarative capability descriptors.
///
/// This matcher is intentionally generic and descriptor-driven. It never checks
/// for application names or concrete role names; it only groups broad capability
/// vocabulary so fallback chains can proceed in a conservative order.
pub fn fallback_phase_for(capabilities: &[String]) -> FallbackTaskPhase {
    let text = capabilities.join(" ").to_lowercase();
    if any_capability_marker(&text, &["research", "source", "web", "discovery"]) {
        FallbackTaskPhase::Research
    } else if any_capability_marker(
        &text,
        &[
            "write",
            "draft",
            "implement",
            "build",
            "code",
            "artifact",
            "deliverable",
            "scaffold",
            "component",
        ],
    ) {
        FallbackTaskPhase::Produce
    } else if any_capability_marker(
        &text,
        &[
            "analysis",
            "analyze",
            "architecture",
            "design",
            "spec",
            "planning",
            "interface",
        ],
    ) {
        FallbackTaskPhase::Analyze
    } else if any_capability_marker(&text, &["fact", "verify", "validation", "quality"]) {
        FallbackTaskPhase::Validate
    } else if any_capability_marker(
        &text,
        &["edit", "review", "package", "polish", "test", "qa"],
    ) {
        FallbackTaskPhase::Finalize
    } else {
        FallbackTaskPhase::Execute
    }
}

fn any_capability_marker(text: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| text.contains(marker))
}

fn fallback_task_template(
    phase: FallbackTaskPhase,
    agent_name: &str,
    goal_description: &str,
) -> (String, String, Vec<String>) {
    let clipped_goal = goal_description.chars().take(500).collect::<String>();
    let title = match phase {
        FallbackTaskPhase::Research => "Collect source material and context",
        FallbackTaskPhase::Validate => "Validate facts, assumptions, and constraints",
        FallbackTaskPhase::Analyze => "Produce specialist analysis and handoff brief",
        FallbackTaskPhase::Produce => "Create the primary specialist deliverable",
        FallbackTaskPhase::Finalize => "Review, polish, and package final deliverables",
        FallbackTaskPhase::Execute => "Complete specialist contribution",
    };
    let description = format!(
        "Planner LLM timed out before creating todos. Fallback task for `{agent_name}`.\n\n\
         Goal:\n{clipped_goal}\n\n\
         Use your declared capabilities and allowed tools to produce the durable deliverable \
         expected for this stage. Save outputs under the shared workspace when files are needed, \
         then submit a concise review summary with exact paths."
    );
    let criteria = match phase {
        FallbackTaskPhase::Research => vec![
            "Collect reliable source material or project context relevant to the goal".to_string(),
            "Separate confirmed facts from assumptions or unresolved questions".to_string(),
            "Save a durable handoff artifact in the shared workspace when applicable".to_string(),
        ],
        FallbackTaskPhase::Validate => vec![
            "Validate important claims, assumptions, dependencies, or constraints".to_string(),
            "Flag unsupported, risky, contradictory, or incomplete information".to_string(),
            "Save findings and safe wording or implementation guidance when applicable".to_string(),
        ],
        FallbackTaskPhase::Analyze => vec![
            "Create a clear specialist brief, plan, architecture, or analysis for downstream work"
                .to_string(),
            "Identify dependencies, risks, and handoff requirements".to_string(),
            "Save the brief in the shared workspace when applicable".to_string(),
        ],
        FallbackTaskPhase::Produce => vec![
            "Produce the primary deliverable requested by the goal for this specialist area"
                .to_string(),
            "Use upstream handoff artifacts if they exist".to_string(),
            "Save durable output files or a detailed completion summary".to_string(),
        ],
        FallbackTaskPhase::Finalize => vec![
            "Review upstream deliverables for completeness, correctness, and user fit".to_string(),
            "Polish or package final outputs without hiding uncertainty".to_string(),
            "Submit exact output paths and remaining caveats".to_string(),
        ],
        FallbackTaskPhase::Execute => vec![
            "Complete a useful specialist contribution toward the goal".to_string(),
            "Create durable output or a clear handoff summary".to_string(),
            "Submit exact paths, decisions, and unresolved blockers".to_string(),
        ],
    };
    (title.to_string(), description, criteria)
}

#[cfg(test)]
mod tests {
    use super::fallback_phase_for;
    use macaca_proto::FallbackTaskPhase;

    #[test]
    fn fallback_phase_orders_primary_production_before_validation_or_review() {
        let producer = vec![
            "code_change_planning".to_string(),
            "build_artifact".to_string(),
        ];
        let reviewer = vec!["quality_review".to_string(), "qa_validation".to_string()];

        assert_eq!(fallback_phase_for(&producer), FallbackTaskPhase::Produce);
        assert_eq!(fallback_phase_for(&reviewer), FallbackTaskPhase::Validate);
        assert!(
            fallback_phase_for(&producer) < fallback_phase_for(&reviewer),
            "synthetic fallback chains must produce the primary artifact before validation/review"
        );
    }
}
