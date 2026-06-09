// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Harness plan mode state transitions.
//!
//! Plan mode is agent-local scratch planning state. It can guide the agent's
//! next model calls, but it must not create, claim, assign, or mutate Macaca
//! TaskBoard work. TaskBoard operations remain behind task service commands.

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::agent::AgentResult;
use crate::runtime_context::{AgentState, RuntimeContext};

/// Plan-mode operation requested by Harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessPlanModeOperation {
    Enter { plan_id: Option<String> },
    AddNote { note: String },
    Exit,
}

/// Command for one local plan-mode state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessPlanModeCommand {
    pub runtime: RuntimeContext,
    pub state: AgentState,
    pub operation: HarnessPlanModeOperation,
}

/// Result of a local plan-mode state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessPlanModeResult {
    pub state: AgentState,
    pub task_board_boundary_preserved: bool,
    pub evidence_ref: String,
}

pub struct HarnessPlanModeManager;

impl HarnessPlanModeManager {
    pub fn apply(command: HarnessPlanModeCommand) -> AgentResult<HarnessPlanModeResult> {
        let mut state = command.state;
        match command.operation {
            HarnessPlanModeOperation::Enter { plan_id } => {
                info!(
                    trace_id = %command.runtime.trace_id,
                    plan_id = ?plan_id,
                    "harness plan mode entered"
                );
                state.plan_mode.enabled = true;
                state.plan_mode.plan_id = plan_id;
            }
            HarnessPlanModeOperation::AddNote { note } => {
                debug!(
                    trace_id = %command.runtime.trace_id,
                    "harness plan mode note appended"
                );
                state.plan_mode.notes.push(note);
            }
            HarnessPlanModeOperation::Exit => {
                info!(
                    trace_id = %command.runtime.trace_id,
                    "harness plan mode exited"
                );
                state.plan_mode.enabled = false;
            }
        }
        Ok(HarnessPlanModeResult {
            state,
            task_board_boundary_preserved: true,
            evidence_ref: format!("harness.plan_mode://{}", command.runtime.trace_id),
        })
    }
}
