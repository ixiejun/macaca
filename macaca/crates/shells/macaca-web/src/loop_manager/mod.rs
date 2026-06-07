//! PlanLoop and WorkerLoop lifecycle management (thin orchestrator module).
//!
//! This module follows the Facade + Adapter pattern: each subdirectory adapter owns
//! one integration boundary (execution control, agent execution service, SSE,
//! task-event consumption) while `loop_orchestrator` wires macaca-task controllers.

mod agent_execution_adapter;
mod decomposition_adapter;
mod execution_control_adapter;
mod goal_route;
mod loop_orchestrator;
mod plan_event_consumer;
mod plan_event_context;
mod plan_event_goal_lifecycle;
mod plan_event_goal_ready;
mod plan_event_handlers;
mod plan_loop_orchestrator;
mod worker_loop_orchestrator;
mod planner_helpers;
mod sse_channel_adapter;
mod worker_execution_adapter;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub(crate) use goal_route::create_goal;
pub(crate) use loop_orchestrator::ensure_plan_and_worker_loops;
