// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! Harness skill repository and prompt adapter contracts.
//!
//! The framework does not curate, mutate, install, decrypt, or execute skills.
//! Those responsibilities belong to Macaca skill/plugin services. Harness only
//! models the AgentScope 2.0 dynamic-skill prompt surface and a replaceable
//! repository port for service adapters.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::agent::AgentResult;
use crate::runtime_context::RuntimeContext;

/// Command for querying authorized skills from a service-owned repository.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessSkillSearchCommand {
    pub runtime: RuntimeContext,
    pub query: Option<String>,
    pub limit: usize,
}

/// Command for loading one skill body through the skill service boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessSkillLoadCommand {
    pub runtime: RuntimeContext,
    pub skill_id: String,
}

/// Bounded runtime skill entry. Raw skill package bytes and secrets are never
/// stored in this DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSkillRuntimeEntry {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub prompt: Option<String>,
    pub evidence_ref: Option<String>,
}

/// Replaceable skill repository port.
#[async_trait]
pub trait HarnessSkillRepositoryPort: Send + Sync {
    fn name(&self) -> &str;
    async fn search(
        &self,
        command: HarnessSkillSearchCommand,
    ) -> AgentResult<Vec<HarnessSkillRuntimeEntry>>;
    async fn load(
        &self,
        command: HarnessSkillLoadCommand,
    ) -> AgentResult<Option<HarnessSkillRuntimeEntry>>;
}

/// Null Object skill repository for absent optional skill services.
pub struct UnavailableHarnessSkillRepository {
    reason: String,
}

impl UnavailableHarnessSkillRepository {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl HarnessSkillRepositoryPort for UnavailableHarnessSkillRepository {
    fn name(&self) -> &str {
        "unavailable_harness_skill_repository"
    }

    async fn search(
        &self,
        command: HarnessSkillSearchCommand,
    ) -> AgentResult<Vec<HarnessSkillRuntimeEntry>> {
        warn!(
            trace_id = %command.runtime.trace_id,
            reason = %self.reason,
            "harness skill search unavailable"
        );
        Ok(Vec::new())
    }

    async fn load(
        &self,
        command: HarnessSkillLoadCommand,
    ) -> AgentResult<Option<HarnessSkillRuntimeEntry>> {
        warn!(
            trace_id = %command.runtime.trace_id,
            skill_id = %command.skill_id,
            reason = %self.reason,
            "harness skill load unavailable"
        );
        Ok(None)
    }
}

/// Prompt builder for dynamic skill middleware.
pub struct HarnessSkillPromptBuilder;

impl HarnessSkillPromptBuilder {
    pub fn build(entries: &[HarnessSkillRuntimeEntry]) -> String {
        if entries.is_empty() {
            return String::new();
        }
        debug!(
            skill_count = entries.len(),
            "building harness skill prompt section"
        );
        let mut prompt = String::from("## Available Skills\n");
        for entry in entries {
            prompt.push_str("- ");
            prompt.push_str(&entry.name);
            prompt.push_str(" (`");
            prompt.push_str(&entry.skill_id);
            prompt.push_str("`): ");
            prompt.push_str(&entry.description);
            prompt.push('\n');
            if let Some(skill_prompt) = &entry.prompt {
                prompt.push_str(skill_prompt.trim());
                prompt.push('\n');
            }
        }
        prompt
    }
}
