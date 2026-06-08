//! Task graph admission specification for session-scoped assignments.
//!
//! Enforces at most one authoritative application-execution graph per session while
//! allowing compatibility/diagnostic graph entries for audit evidence.

use tracing::{info, warn};

use crate::commands::CreateTaskAssignmentCommand;

use super::TaskServiceRuntime;

impl<S> TaskServiceRuntime<S>
where
    S: super::TaskServiceExecutionStrategy + 'static,
{
    /// Normalize an assignment graph id before admission.
    ///
    /// Application-execution task assignments form one authoritative graph per
    /// application execution session.  If an adapter does not provide an opaque
    /// graph id, the Task Service derives a deterministic service-owned id from
    /// the session.  The derived value is not an application name, workflow name,
    /// provider name, or business-domain value; it is only a replay/audit key for
    /// grouping tasks that belong to the same execution graph.
    pub(crate) fn normalize_assignment_graph_id(
        command: &CreateTaskAssignmentCommand,
    ) -> Option<String> {
        let requested = command
            .graph_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        requested.or_else(|| {
            command
                .graph_owner
                .is_application_execution_authoritative()
                .then(|| format!("application_execution:{}", command.session_id.trim()))
        })
    }

    /// Admit one task into the session's task graph according to ownership.
    ///
    /// The rule intentionally models graph admission instead of agent workflow
    /// semantics: many tasks may join the same authoritative graph, while a
    /// second authoritative graph id in the same session is rejected.  Compatibility
    /// and diagnostic graph entries remain admissible because they are audit
    /// evidence, not application-execution terminal facts.
    pub(crate) async fn admit_assignment_graph(
        &self,
        command: &CreateTaskAssignmentCommand,
        normalized_graph_id: Option<&str>,
    ) -> Result<(), String> {
        let existing = self
            .store
            .list_all_todos_for_session(&command.app_id, &command.session_id)
            .await;
        let authoritative_conflict = command
            .graph_owner
            .is_application_execution_authoritative()
            .then(|| {
                existing.into_iter().find(|task| {
                    task.graph_owner.is_application_execution_authoritative()
                        && task.graph_id.as_deref().or_else(|| normalized_graph_id)
                            != normalized_graph_id
                })
            })
            .flatten();

        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            graph_owner = %command.graph_owner.as_str(),
            graph_id = normalized_graph_id.unwrap_or("none"),
            trace_id = command
                .trace
                .as_ref()
                .map(|trace| trace.trace_id.as_str())
                .unwrap_or("none"),
            admitted = authoritative_conflict.is_none(),
            "task graph admission evaluated"
        );

        if let Some(conflicting_task) = authoritative_conflict {
            warn!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                requested_graph_id = normalized_graph_id.unwrap_or("none"),
                existing_graph_id = conflicting_task.graph_id.as_deref().unwrap_or("legacy_application_execution"),
                existing_task_id = %conflicting_task.id,
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "task graph admission rejected"
            );
            return Err(format!(
                "task service rejected a second authoritative graph for session {}",
                command.session_id
            ));
        }

        if command.graph_owner.is_application_execution_authoritative() {
            info!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                graph_id = normalized_graph_id.unwrap_or("none"),
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "authoritative task graph admitted"
            );
        } else {
            info!(
                app_id = %command.app_id.0,
                session_id = %command.session_id,
                graph_owner = %command.graph_owner.as_str(),
                graph_id = normalized_graph_id.unwrap_or("none"),
                trace_id = command
                    .trace
                    .as_ref()
                    .map(|trace| trace.trace_id.as_str())
                    .unwrap_or("none"),
                "compatibility task graph admitted"
            );
        }

        Ok(())
    }
}
