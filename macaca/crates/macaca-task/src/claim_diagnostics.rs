//! Explain why [`WorkerLoop::claim_next`](crate::worker_loop::WorkerLoop) may not pick up `Pending` work.
//!
//! Mirrors the strict sequential rule in [`TaskBoard::claim_next`](crate::todo_board::TaskBoard):
//! the first non-terminal task in `sequence_number` order gates all later tasks on that agent board.

use std::collections::HashMap;

use macaca_proto::{TaskId, TodoItem, TodoStatus};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DependencySnap {
    pub task_id: String,
    pub status: String,
    pub title: String,
    pub assigned_agent: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaimGate {
    pub task_id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
    /// `claimable` | `blocked` | `sequential_wait`
    pub gate_kind: String,
    /// For `blocked`: dependencies not in `Completed` state.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub incomplete_dependencies: Vec<DependencySnap>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentClaimReport {
    pub agent: String,
    /// True iff the first non-terminal task on this board is `Pending`.
    pub worker_would_claim_now: bool,
    pub summary: String,
    pub pending_count: usize,
    pub blocked_count: usize,
    pub first_gate: Option<ClaimGate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionClaimDiagnostics {
    pub agents: Vec<AgentClaimReport>,
    /// Agents that have `Pending` tasks but `worker_would_claim_now == false` (usual stall signal).
    pub agents_pending_but_not_claiming: Vec<String>,
}

fn status_label(s: TodoStatus) -> &'static str {
    match s {
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
    todos.iter().map(|t| (t.id, t)).collect()
}

fn dep_snapshot(index: &HashMap<TaskId, &TodoItem>, dep: &TaskId) -> DependencySnap {
    match index.get(dep) {
        Some(t) => DependencySnap {
            task_id: dep.to_string(),
            status: status_label(t.status).to_string(),
            title: t.title.clone(),
            assigned_agent: t.assigned_agent.clone(),
        },
        None => DependencySnap {
            task_id: dep.to_string(),
            status: "Missing".to_string(),
            title: String::new(),
            assigned_agent: String::new(),
        },
    }
}

/// Per-agent diagnosis for the same session todo set used by `TaskSpace` / `list_all_todos_for_session`.
pub fn diagnose_session_claims(todos: &[TodoItem]) -> SessionClaimDiagnostics {
    let index = build_index(todos);
    let mut agents: Vec<String> = todos.iter().map(|t| t.assigned_agent.clone()).collect();
    agents.sort();
    agents.dedup();

    let mut reports = Vec::new();
    let mut pending_but_not_claiming = Vec::new();

    for agent in agents {
        let mut mine: Vec<&TodoItem> = todos.iter().filter(|t| t.assigned_agent == agent).collect();
        mine.sort_by_key(|t| t.sequence_number);

        let pending_count = mine
            .iter()
            .filter(|t| t.status == TodoStatus::Pending)
            .count();
        let blocked_count = mine
            .iter()
            .filter(|t| t.status == TodoStatus::Blocked)
            .count();

        let mut first_gate: Option<ClaimGate> = None;
        let mut worker_would_claim_now = false;
        let mut summary = String::new();

        for t in &mine {
            match t.status {
                TodoStatus::Completed | TodoStatus::Cancelled | TodoStatus::Failed => continue,
                TodoStatus::Pending => {
                    worker_would_claim_now = true;
                    first_gate = Some(ClaimGate {
                        task_id: t.id.to_string(),
                        sequence_number: t.sequence_number,
                        title: t.title.clone(),
                        status: status_label(t.status).to_string(),
                        gate_kind: "claimable".to_string(),
                        incomplete_dependencies: vec![],
                    });
                    summary = format!(
                        "WorkerLoop can claim next: seq={} pending task '{}'",
                        t.sequence_number, t.title
                    );
                    break;
                }
                TodoStatus::Blocked => {
                    let completed: std::collections::HashSet<TaskId> = todos
                        .iter()
                        .filter(|x| x.status == TodoStatus::Completed)
                        .map(|x| x.id)
                        .collect();
                    let incomplete: Vec<DependencySnap> = t
                        .depends_on
                        .iter()
                        .filter(|d| !completed.contains(*d))
                        .map(|d| dep_snapshot(&index, d))
                        .collect();
                    first_gate = Some(ClaimGate {
                        task_id: t.id.to_string(),
                        sequence_number: t.sequence_number,
                        title: t.title.clone(),
                        status: status_label(t.status).to_string(),
                        gate_kind: "blocked".to_string(),
                        incomplete_dependencies: incomplete.clone(),
                    });
                    let dep_msg = if incomplete.is_empty() {
                        "blocked but no incomplete deps listed (data issue?)".to_string()
                    } else {
                        incomplete
                            .iter()
                            .map(|d| {
                                format!(
                                    "{} [{}] {} ({})",
                                    d.task_id, d.status, d.title, d.assigned_agent
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("; ")
                    };
                    summary = format!(
                        "Not claiming: first open task seq={} is Blocked — waiting on: {}",
                        t.sequence_number, dep_msg
                    );
                    break;
                }
                TodoStatus::Assigned
                | TodoStatus::InProgress
                | TodoStatus::PendingReview
                | TodoStatus::NeedsOptimization => {
                    first_gate = Some(ClaimGate {
                        task_id: t.id.to_string(),
                        sequence_number: t.sequence_number,
                        title: t.title.clone(),
                        status: status_label(t.status).to_string(),
                        gate_kind: "sequential_wait".to_string(),
                        incomplete_dependencies: vec![],
                    });
                    summary = format!(
                        "Not claiming: seq={} is {} ('{}') — WorkerLoop will not skip to later Pending tasks",
                        t.sequence_number,
                        status_label(t.status),
                        t.title
                    );
                    break;
                }
            }
        }

        if first_gate.is_none() {
            if mine.is_empty() {
                summary = "No tasks on this agent board".to_string();
            } else {
                summary = "All tasks terminal (or board empty after filter)".to_string();
            }
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
    use super::*;
    use chrono::Utc;
    use macaca_proto::ApplicationId;

    fn base_item(
        agent: &str,
        seq: u32,
        status: TodoStatus,
        title: &str,
        deps: Vec<TaskId>,
    ) -> TodoItem {
        let mut t = TodoItem::new(
            ApplicationId::new(),
            Some("sess".into()),
            agent,
            "planner",
            title,
            "d",
            5,
        );
        t.sequence_number = seq;
        t.status = status;
        t.depends_on = deps;
        t.created_at = Utc::now();
        t.updated_at = Utc::now();
        t
    }

    #[test]
    fn pending_claimable() {
        let t = base_item("backend", 1, TodoStatus::Pending, "api", vec![]);
        let d = diagnose_session_claims(std::slice::from_ref(&t));
        assert!(d.agents[0].worker_would_claim_now);
        assert!(d.agents_pending_but_not_claiming.is_empty());
    }

    #[test]
    fn blocked_waits_dep() {
        let arch = base_item("architect", 1, TodoStatus::Pending, "design", vec![]);
        let arch_id = arch.id;
        let be = base_item("backend", 1, TodoStatus::Blocked, "api", vec![arch_id]);
        let d = diagnose_session_claims(&[arch, be]);
        let backend = d.agents.iter().find(|a| a.agent == "backend").unwrap();
        assert!(!backend.worker_would_claim_now);
        assert_eq!(backend.pending_count, 0);
        assert!(backend.summary.contains("Blocked"));
    }
}
