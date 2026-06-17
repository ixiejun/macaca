//! Provider-neutral claim diagnostics for Task Board projections.
//!
//! The diagnostic logic works only on `TodoItem` DTOs, so it belongs with the
//! protocol data model instead of the concrete task service runtime. Shells can
//! render these projections without importing task-service implementation
//! crates or owning WorkerLoop scheduling semantics.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{TaskId, TodoItem, TodoStatus};

/// Snapshot of one dependency used in a claim gate explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySnap {
    pub task_id: String,
    pub status: String,
    pub title: String,
    pub assigned_agent: String,
}

/// First non-terminal task that decides whether a worker can claim work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimGate {
    pub task_id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
    /// `claimable` | `blocked` | `sequential_wait`
    pub gate_kind: String,
    /// For `blocked`: dependencies not in `Completed` state.
    ///
    /// Older and empty claimable gates omit this field from JSON to keep the
    /// shell payload compact. Deserialization still defaults it to an empty
    /// vector so service-call round trips remain forward compatible.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub incomplete_dependencies: Vec<DependencySnap>,
}

/// Per-agent claimability report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentClaimReport {
    pub agent: String,
    /// True iff the first non-terminal task on this board is `Pending`.
    pub worker_would_claim_now: bool,
    pub summary: String,
    pub pending_count: usize,
    pub blocked_count: usize,
    pub first_gate: Option<ClaimGate>,
}

/// Session-wide claim diagnostics grouped by agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaimDiagnostics {
    pub agents: Vec<AgentClaimReport>,
    /// Agents that have `Pending` tasks but cannot claim because an earlier
    /// non-terminal task gates the worker board.
    pub agents_pending_but_not_claiming: Vec<String>,
}

fn status_label(status: TodoStatus) -> &'static str {
    match status {
        TodoStatus::Pending => "Pending",
        TodoStatus::Assigned => "Assigned",
        TodoStatus::InProgress => "InProgress",
        TodoStatus::PendingReview => "PendingReview",
        TodoStatus::NeedsOptimization => "NeedsOptimization",
        TodoStatus::Completed => "Completed",
        TodoStatus::Blocked => "Blocked",
        TodoStatus::Cancelled => "Cancelled",
        TodoStatus::Failed => "Failed",
    }
}

fn build_index(todos: &[TodoItem]) -> HashMap<TaskId, &TodoItem> {
    todos.iter().map(|todo| (todo.id, todo)).collect()
}

fn dep_snapshot(index: &HashMap<TaskId, &TodoItem>, dep: &TaskId) -> DependencySnap {
    match index.get(dep) {
        Some(todo) => DependencySnap {
            task_id: dep.to_string(),
            status: status_label(todo.status).to_string(),
            title: todo.title.clone(),
            assigned_agent: todo.assigned_agent.clone(),
        },
        None => DependencySnap {
            task_id: dep.to_string(),
            status: "Missing".to_string(),
            title: String::new(),
            assigned_agent: String::new(),
        },
    }
}

/// Diagnose why each agent would or would not claim pending work.
///
/// The algorithm mirrors the worker-board sequential rule: the first
/// non-terminal task in `sequence_number` order gates every later task on the
/// same agent board.
pub fn diagnose_session_claims(todos: &[TodoItem]) -> SessionClaimDiagnostics {
    let index = build_index(todos);
    let completed: HashSet<TaskId> = todos
        .iter()
        .filter(|todo| todo.status == TodoStatus::Completed)
        .map(|todo| todo.id)
        .collect();
    let mut agents: Vec<String> = todos
        .iter()
        .map(|todo| todo.assigned_agent.clone())
        .collect();
    agents.sort();
    agents.dedup();

    let mut reports = Vec::new();
    let mut pending_but_not_claiming = Vec::new();

    for agent in agents {
        let mut mine: Vec<&TodoItem> = todos
            .iter()
            .filter(|todo| todo.assigned_agent == agent)
            .collect();
        mine.sort_by_key(|todo| todo.sequence_number);

        let pending_count = mine
            .iter()
            .filter(|todo| todo.status == TodoStatus::Pending)
            .count();
        let blocked_count = mine
            .iter()
            .filter(|todo| todo.status == TodoStatus::Blocked)
            .count();

        let mut first_gate: Option<ClaimGate> = None;
        let mut worker_would_claim_now = false;
        let mut summary = String::new();

        for task in &mine {
            match task.status {
                TodoStatus::Completed | TodoStatus::Cancelled | TodoStatus::Failed => continue,
                TodoStatus::Pending => {
                    worker_would_claim_now = true;
                    first_gate = Some(ClaimGate {
                        task_id: task.id.to_string(),
                        sequence_number: task.sequence_number,
                        title: task.title.clone(),
                        status: status_label(task.status).to_string(),
                        gate_kind: "claimable".to_string(),
                        incomplete_dependencies: Vec::new(),
                    });
                    summary = format!(
                        "WorkerLoop can claim next: seq={} pending task '{}'",
                        task.sequence_number, task.title
                    );
                    break;
                }
                TodoStatus::Blocked => {
                    let incomplete: Vec<DependencySnap> = task
                        .depends_on
                        .iter()
                        .filter(|dep| !completed.contains(*dep))
                        .map(|dep| dep_snapshot(&index, dep))
                        .collect();
                    first_gate = Some(ClaimGate {
                        task_id: task.id.to_string(),
                        sequence_number: task.sequence_number,
                        title: task.title.clone(),
                        status: status_label(task.status).to_string(),
                        gate_kind: "blocked".to_string(),
                        incomplete_dependencies: incomplete.clone(),
                    });
                    let dep_msg = if incomplete.is_empty() {
                        "blocked but no incomplete deps listed (data issue?)".to_string()
                    } else {
                        incomplete
                            .iter()
                            .map(|dep| {
                                format!(
                                    "{} [{}] {} ({})",
                                    dep.task_id, dep.status, dep.title, dep.assigned_agent
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("; ")
                    };
                    summary = format!(
                        "Not claiming: first open task seq={} is Blocked - waiting on: {}",
                        task.sequence_number, dep_msg
                    );
                    break;
                }
                TodoStatus::Assigned
                | TodoStatus::InProgress
                | TodoStatus::PendingReview
                | TodoStatus::NeedsOptimization => {
                    first_gate = Some(ClaimGate {
                        task_id: task.id.to_string(),
                        sequence_number: task.sequence_number,
                        title: task.title.clone(),
                        status: status_label(task.status).to_string(),
                        gate_kind: "sequential_wait".to_string(),
                        incomplete_dependencies: Vec::new(),
                    });
                    summary = format!(
                        "Not claiming: seq={} is {} ('{}') - WorkerLoop will not skip to later Pending tasks",
                        task.sequence_number,
                        status_label(task.status),
                        task.title
                    );
                    break;
                }
            }
        }

        if first_gate.is_none() {
            summary = if mine.is_empty() {
                "No tasks on this agent board".to_string()
            } else {
                "All tasks terminal (or board empty after filter)".to_string()
            };
        }

        if pending_count > 0 && !worker_would_claim_now {
            pending_but_not_claiming.push(agent.clone());
        }

        reports.push(AgentClaimReport {
            agent,
            worker_would_claim_now,
            summary,
            pending_count,
            blocked_count,
            first_gate,
        });
    }

    SessionClaimDiagnostics {
        agents: reports,
        agents_pending_but_not_claiming: pending_but_not_claiming,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::ApplicationId;

    fn item(agent: &str, seq: u32, status: TodoStatus, title: &str, deps: Vec<TaskId>) -> TodoItem {
        let mut item = TodoItem::new(
            ApplicationId::new(),
            Some("session.test".into()),
            agent,
            "planner",
            title,
            "diagnostic fixture",
            5,
        );
        item.sequence_number = seq;
        item.status = status;
        item.depends_on = deps;
        item.created_at = Utc::now();
        item.updated_at = Utc::now();
        item
    }

    #[test]
    fn pending_task_is_claimable_when_first_open_task() {
        let task = item("worker", 1, TodoStatus::Pending, "ready", Vec::new());
        let diagnostics = diagnose_session_claims(&[task]);

        assert!(diagnostics.agents[0].worker_would_claim_now);
        assert!(diagnostics.agents_pending_but_not_claiming.is_empty());
    }

    #[test]
    fn blocked_task_reports_incomplete_dependency() {
        let dependency = item("planner", 1, TodoStatus::Pending, "dependency", Vec::new());
        let blocked = item(
            "worker",
            1,
            TodoStatus::Blocked,
            "blocked",
            vec![dependency.id],
        );

        let diagnostics = diagnose_session_claims(&[dependency, blocked]);
        let worker = diagnostics
            .agents
            .iter()
            .find(|report| report.agent == "worker")
            .expect("worker report");

        assert!(!worker.worker_would_claim_now);
        assert_eq!(worker.first_gate.as_ref().unwrap().gate_kind, "blocked");
        assert_eq!(
            worker
                .first_gate
                .as_ref()
                .unwrap()
                .incomplete_dependencies
                .len(),
            1
        );
    }

    #[test]
    fn claimable_gate_round_trips_when_dependency_list_is_omitted() {
        let value = serde_json::json!({
            "task_id": TaskId::new().to_string(),
            "sequence_number": 1,
            "title": "ready",
            "status": "Pending",
            "gate_kind": "claimable"
        });

        let gate: ClaimGate =
            serde_json::from_value(value).expect("missing dependency list should default");

        assert!(gate.incomplete_dependencies.is_empty());
    }
}
