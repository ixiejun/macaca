//! Observer bridge from Agent Execution completions into Skill self-evolution.
//!
//! The Web shell does not own self-evolution semantics. It observes the generic
//! `service.agent_execution` result boundary, converts a verified terminal
//! success into bounded evidence references, and forwards a typed command through
//! the SDK Skill facade. The Skill service remains the owner of proposal
//! validation, policy, storage, curation, promotion, and rollback behavior.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chrono::Utc;
use macaca_kernel::executor::TaskResult;
use macaca_proto::{
    AgentExecutionResult, AgentExecutionStatus, ApplicationId, TaskId, TraceContext,
};
use macaca_skill::{
    SkillEvolutionCandidateClassification, SkillEvolutionProposalAction, SkillExperienceCandidate,
    SkillExperienceCandidateDestination, SkillExperienceEvidenceGateStatus,
    SkillExperienceProposalCommand, SkillServiceScope,
};

use crate::state::AppState;

/// Maximum number of artifact references carried in metadata.
///
/// Artifact paths can be useful for audit replay, but the observer must keep
/// proposal metadata bounded because the Skill operations surface can be shown
/// in shells and reports.
const MAX_ARTIFACT_REFS: usize = 8;

/// Maximum number of characters copied from one artifact reference.
const MAX_ARTIFACT_REF_CHARS: usize = 256;

/// Maximum number of semantic trigger phrases used for an autonomous Skill name.
///
/// The frontmatter `name` is a model-facing selector, not a provenance id.  A
/// short phrase budget keeps generated Skills easy to trigger while avoiding
/// overfitted task transcripts or application-specific wording.
const MAX_SEMANTIC_TRIGGER_PHRASES: usize = 4;

/// Bounded observer result written to EventLog by the decorator.
///
/// This is intentionally small and stringly for audit display only.  The Skill
/// service result remains the typed source of truth; the shell event lets live
/// black-box tests prove that the Agent Execution boundary observer actually ran.
#[derive(Debug, Clone)]
pub(crate) struct SkillSelfEvolutionObservation {
    pub status: &'static str,
    pub task_id: Option<String>,
    pub proposal_id: Option<String>,
    pub reason: Option<String>,
}

/// Observe a completed Agent Execution service result.
///
/// The Web chat main thread and delegated worker path complete through
/// `service.agent_execution`. This adapter treats that service result as the
/// production completion boundary, converts only bounded output metadata into a
/// task result view, and forwards the command to the Skill service. Keeping this
/// bridge in the observer module prevents chat orchestration from owning Skill
/// curation semantics.
pub(crate) async fn observe_agent_execution_result_for_skill_self_evolution(
    state: &Arc<AppState>,
    result: &AgentExecutionResult,
) -> Option<SkillSelfEvolutionObservation> {
    if result.status != AgentExecutionStatus::Completed {
        return Some(SkillSelfEvolutionObservation {
            status: "skipped_non_completed_agent_execution",
            task_id: result.task_id.map(|task_id| task_id.to_string()),
            proposal_id: None,
            reason: Some(format!(
                "agent execution status was {}",
                result.status.as_str()
            )),
        });
    }

    let task_result = task_result_from_agent_execution_result(result);
    let command = match build_skill_experience_proposal_command(
        &result.application_id,
        &result.session_id,
        &result.target_agent,
        &task_result,
        &result.trace.trace_id,
    ) {
        Some(command) => command,
        None => {
            return Some(SkillSelfEvolutionObservation {
                status: "skipped_missing_evidence",
                task_id: Some(task_result.task_id.to_string()),
                proposal_id: None,
                reason: Some("task completion had no bounded replay evidence".into()),
            });
        }
    };

    forward_skill_experience_proposal_command(state, command, &task_result, result).await
}

/// Build the task-result projection used by the Skill proposal command.
///
/// Agent Execution returns a structured service result instead of a kernel
/// executor event. The projection preserves only the stable task id, success
/// status, bounded output text, and completion time required to generate counts
/// and references; raw prompts and context snapshots never enter the proposal.
fn task_result_from_agent_execution_result(result: &AgentExecutionResult) -> TaskResult {
    let task_id = result.task_id.unwrap_or_else(TaskId::new);
    let output = agent_execution_output_text(&result.output);
    let artifacts = artifact_refs_from_agent_execution_result(result);
    if !artifacts.is_empty() {
        tracing::info!(
            trace_id = %result.trace.trace_id,
            task_id = %task_id,
            artifact_refs = artifacts.len(),
            "Skill self-evolution observer preserved bounded artifact evidence from Agent Execution metadata"
        );
    }
    TaskResult {
        task_id,
        success: true,
        output,
        error: None,
        artifacts,
        completed_at: Utc::now(),
        tokens_used: None,
    }
}

/// Forward a sanitized Skill proposal command and return bounded audit state.
///
/// The observer is best-effort: the original Agent Execution result has already
/// been produced, so missing Skill service providers, policy rejections, or store
/// failures are logged and surfaced as observer status without changing the task
/// success/failure result.
async fn forward_skill_experience_proposal_command(
    state: &Arc<AppState>,
    command: SkillExperienceProposalCommand,
    task_result: &TaskResult,
    source_result: &AgentExecutionResult,
) -> Option<SkillSelfEvolutionObservation> {
    tracing::info!(
        trace_id = %command.trace.trace_id,
        task_id = %command.candidate.task_id,
        session_id = %source_result.session_id,
        agent = %source_result.target_agent,
        artifact_count = task_result.artifacts.len(),
        "Forwarding verified Agent Execution completion to Skill self-evolution service"
    );

    match state.skill_client.propose_skill_experience(command).await {
        Ok(result) => {
            let proposal_id = result.proposal.proposal_id.clone();
            tracing::info!(
                proposal_id = %result.proposal.proposal_id,
                mutated = result.mutated,
                destination = ?result.proposal.destination,
                route_status = %result.destination_route.status.as_str(),
                "Skill self-evolution proposal recorded from Agent Execution completion"
            );
            Some(SkillSelfEvolutionObservation {
                status: "proposal_created",
                task_id: Some(result.proposal.task_id),
                proposal_id: Some(proposal_id),
                reason: None,
            })
        }
        Err(error) => {
            tracing::warn!(
                task_id = %task_result.task_id,
                session_id = %source_result.session_id,
                error = %error,
                "Skill self-evolution proposal forwarding failed"
            );
            Some(SkillSelfEvolutionObservation {
                status: "proposal_failed",
                task_id: Some(task_result.task_id.to_string()),
                proposal_id: None,
                reason: Some("skill service rejected or failed the proposal command".into()),
            })
        }
    }
}

/// Build the provider-neutral command sent to the Skill service.
///
/// The command uses only stable identifiers, bounded counts, and event-log
/// references.  It deliberately avoids raw prompts, raw model payloads, raw task
/// output, and full artifact contents so the proposal remains safe to display,
/// score, and replay through governance tooling.
fn build_skill_experience_proposal_command(
    app_id: &ApplicationId,
    session_id: &str,
    agent: &str,
    result: &TaskResult,
    agent_execution_trace_id: &str,
) -> Option<SkillExperienceProposalCommand> {
    let task_id = result.task_id.to_string();

    if result.output.trim().is_empty() && result.artifacts.is_empty() {
        tracing::debug!(
            task_id = %result.task_id,
            "Skipping skill self-evolution proposal without output or artifact evidence"
        );
        return None;
    }

    let trace = proposal_trace(app_id, session_id, agent, result);
    let scope = SkillServiceScope {
        application_id: Some(*app_id),
        session_id: Some(session_id.to_string()),
        tenant_id: None,
        agent_name: Some(agent.to_string()),
    };
    let semantic_signal = SemanticSkillCreatorSignal::from_task_result(result);

    let evidence_ids = vec![
        format!(
            "eventlog://sessions/{}/skill_self_evolution_observer/agent_execution_completed_seen/{}",
            session_id, result.task_id
        ),
        format!("trace://service.agent_execution/{}", agent_execution_trace_id),
    ];

    let mut metadata = BTreeMap::new();
    metadata.insert("origin".into(), "agent_execution_result_observer".into());
    metadata.insert("evidence_ref.event_log".into(), evidence_ids[0].clone());
    metadata.insert(
        "evidence_ref.agent_execution_trace".into(),
        evidence_ids[1].clone(),
    );
    metadata.insert(
        "output_char_count".into(),
        result.output.chars().count().to_string(),
    );
    metadata.insert("artifact_count".into(), result.artifacts.len().to_string());
    metadata.insert("completed_at".into(), result.completed_at.to_rfc3339());
    if let Some(tokens) = &result.tokens_used {
        metadata.insert("total_tokens".into(), tokens.total_tokens.to_string());
    }
    for (index, artifact) in result.artifacts.iter().take(MAX_ARTIFACT_REFS).enumerate() {
        metadata.insert(
            format!("evidence_ref.artifact_{}", index),
            bounded_artifact_ref(artifact),
        );
    }

    tracing::info!(
        task_id = %result.task_id,
        semantic_target = semantic_signal.target_skill_name.as_deref().unwrap_or(""),
        semantic_phrase_count = semantic_signal.trigger_phrases.len(),
        semantic_signal_fallback = semantic_signal.target_skill_name.is_none(),
        "Skill self-evolution observer built Skill Creator-compatible semantic trigger signal"
    );

    Some(SkillExperienceProposalCommand {
        trace,
        scope,
        candidate: SkillExperienceCandidate {
            task_id,
            session_id: Some(session_id.to_string()),
            application_id: Some(*app_id),
            agent_name: Some(agent.to_string()),
            verified_terminal_success: true,
            evidence_gate: SkillExperienceEvidenceGateStatus::Accepted,
            bounded_summary: semantic_signal.bounded_summary(format!(
                "Verified terminal task completion observed through service.agent_execution; output_chars={}, artifact_count={}, token_total={}.",
                result.output.chars().count(),
                result.artifacts.len(),
                result
                    .tokens_used
                    .as_ref()
                    .map(|tokens| tokens.total_tokens.to_string())
                    .unwrap_or_else(|| "unavailable".to_string())
            )),
            trace_digest: Some(format!(
                "session={},task={},agent={},agent_execution_trace={},completed_at={}",
                session_id,
                result.task_id,
                agent,
                agent_execution_trace_id,
                result.completed_at.to_rfc3339()
            )),
            memory_digest_refs: Vec::new(),
            reusable_procedure: semantic_signal.reusable_procedure(),
            classification: SkillEvolutionCandidateClassification::ReusableProcedure,
            destination: SkillExperienceCandidateDestination::NewSkillDraft,
            recommended_action: SkillEvolutionProposalAction::CreateDraft,
            target_skill_id: None,
            target_skill_name: semantic_signal.target_skill_name,
            evidence_ids,
            metadata,
        },
    })
}

/// Create the trace context attached to the Skill service command.
fn proposal_trace(
    app_id: &ApplicationId,
    session_id: &str,
    agent: &str,
    result: &TaskResult,
) -> TraceContext {
    let mut trace = TraceContext::new(format!(
        "skill-self-evolution:{}:{}:{}:{}",
        app_id.0, session_id, agent, result.task_id
    ));
    trace.session_id = Some(session_id.to_string());
    trace.task_id = Some(result.task_id.to_string());
    trace.agent = Some(agent.to_string());
    trace
}

/// Return a bounded artifact reference suitable for proposal metadata.
fn bounded_artifact_ref(artifact: &str) -> String {
    artifact.chars().take(MAX_ARTIFACT_REF_CHARS).collect()
}

/// Extract bounded artifact evidence that Web already collected for the
/// service.agent_execution result.
///
/// Agent Execution exposes durable write evidence as sanitized metadata instead
/// of raw file paths or file bodies.  The self-evolution observer must preserve
/// those refs when building the Skill proposal; otherwise downstream proposal
/// quality and materialization readiness cannot distinguish "useful artifact
/// produced" from "chat-only completion".  This helper deliberately accepts
/// only stable ref/digest fields and caps the number and size of refs.
fn artifact_refs_from_agent_execution_result(result: &AgentExecutionResult) -> Vec<String> {
    let mut refs = Vec::new();
    push_bounded_artifact_ref(&mut refs, result.metadata.get("artifact_ref"));
    for (key, value) in &result.metadata {
        if key.starts_with("evidence_ref.artifact_") {
            push_bounded_artifact_ref(&mut refs, Some(value));
        }
    }
    if refs.is_empty() {
        push_bounded_artifact_ref(&mut refs, result.metadata.get("artifact_digest"));
    }
    refs.sort();
    refs.dedup();
    refs.truncate(MAX_ARTIFACT_REFS);
    refs
}

fn push_bounded_artifact_ref(refs: &mut Vec<String>, value: Option<&String>) {
    let Some(value) = value else {
        return;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    refs.push(bounded_artifact_ref(trimmed));
}

/// Bounded semantic signal used to create Skill Creator-compatible proposals.
///
/// The observer cannot author application-specific Skills and must not copy raw
/// task output into generated packages.  This value object applies a small
/// deterministic Specification over sanitized completion text: it keeps
/// reusable Skill OS concepts, folds well-known compound phrases, and emits a
/// trigger-oriented name/procedure that the downstream materialization Builder
/// can turn into valid `SKILL.md` frontmatter.
struct SemanticSkillCreatorSignal {
    target_skill_name: Option<String>,
    trigger_phrases: Vec<String>,
}

impl SemanticSkillCreatorSignal {
    fn from_task_result(result: &TaskResult) -> Self {
        let trigger_phrases = semantic_trigger_phrases(&result.output);
        let target_skill_name = if trigger_phrases.len() >= 2 {
            Some(trigger_phrases.join("-"))
        } else {
            None
        };
        Self {
            target_skill_name,
            trigger_phrases,
        }
    }

    /// Build a Skill Creator-style bounded summary.
    ///
    /// The summary is still count/ref based for audit safety, but when semantic
    /// triggers are available it exposes them as bounded trigger context instead
    /// of leaving materialization to infer identity from generic observer text.
    fn bounded_summary(&self, fallback: String) -> String {
        if self.trigger_phrases.is_empty() {
            return fallback;
        }
        format!(
            "Reusable Skill trigger context: {}; {}",
            self.trigger_phrases.join(", "),
            fallback
        )
    }

    /// Build concise procedural guidance for a generated Skill.
    ///
    /// This mirrors Skill Creator constraints: description/trigger identity
    /// must be meaningful, the body must stay lean, and provenance stays in refs
    /// rather than raw task artifacts.
    fn reusable_procedure(&self) -> String {
        if self.trigger_phrases.is_empty() {
            return "Review linked event-log and Agent Execution trace refs, extract provider-neutral steps that led to verified terminal success, and keep promoted skill drafts governed by curation, approval, rollback, and sanitized evidence gates.".into();
        }
        format!(
            "Use linked event-log and Agent Execution trace refs to repeat tasks involving {}. Keep the generated Skill concise, trigger-oriented, proposal-linked, registry-visible, telemetry-audited, and governed by approval, rollback, and sanitized evidence gates.",
            self.trigger_phrases.join(", ")
        )
    }
}

/// Extract deterministic trigger phrases from bounded completion text.
///
/// The phrase table is generic to the Skill OS domain rather than a particular
/// application.  It preserves compound concepts such as `registry-load-path`
/// because splitting them into individual words makes model-facing Skill names
/// much harder to trigger and audit.
fn semantic_trigger_phrases(output: &str) -> Vec<String> {
    let tokens = normalized_semantic_tokens(output);
    let mut discovered = BTreeSet::new();
    for index in 0..tokens.len() {
        if let Some(phrase) = semantic_compound_phrase(&tokens, index) {
            discovered.insert(phrase);
        }
        if is_semantic_trigger_token(&tokens[index]) {
            discovered.insert(tokens[index].clone());
        }
    }
    if tokens.iter().any(|token| token == "skill") && tokens.iter().any(|token| token == "package")
    {
        discovered.insert("skill-package".into());
    }
    if tokens.iter().any(|token| token == "registry")
        && tokens
            .iter()
            .any(|token| token == "path" || token == "loadpath")
    {
        discovered.insert("registry-load-path".into());
    }
    semantic_trigger_priority_order()
        .iter()
        .filter(|phrase| discovered.contains(**phrase))
        .take(MAX_SEMANTIC_TRIGGER_PHRASES)
        .map(|phrase| (*phrase).to_string())
        .collect()
}

fn normalized_semantic_tokens(output: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in output.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
        } else if !current.is_empty() {
            push_normalized_token(&mut tokens, &current);
            current.clear();
        }
    }
    if !current.is_empty() {
        push_normalized_token(&mut tokens, &current);
    }
    tokens
}

fn push_normalized_token(tokens: &mut Vec<String>, token: &str) {
    if token.len() < 4 || is_semantic_stop_word(token) {
        return;
    }
    tokens.push(token.to_string());
}

fn semantic_compound_phrase(tokens: &[String], index: usize) -> Option<String> {
    let current = tokens.get(index)?.as_str();
    let next = tokens.get(index + 1).map(String::as_str);
    match (current, next) {
        ("proposal", Some("linked")) => Some("proposal-linked".into()),
        ("skill", Some("package")) => Some("skill-package".into()),
        ("registry", Some("load")) => Some("registry-load-path".into()),
        ("usage", Some("telemetry")) => Some("usage-telemetry".into()),
        ("semantic", Some("naming")) => Some("semantic-naming".into()),
        ("semantic", Some("identity")) => Some("semantic-identity".into()),
        ("follow", Some("optimization")) => Some("follow-up-optimization".into()),
        ("measurable", Some("optimization")) => Some("measurable-optimization".into()),
        _ => None,
    }
}

fn semantic_trigger_priority_order() -> &'static [&'static str] {
    &[
        "materialization",
        "proposal-linked",
        "skill-package",
        "registry-load-path",
        "usage-telemetry",
        "semantic-naming",
        "semantic-identity",
        "measurable-optimization",
        "follow-up-optimization",
        "activation",
        "curation",
        "evaluation",
        "governance",
        "telemetry",
        "validation",
        "optimization",
        "registry",
        "proposal",
    ]
}

fn is_semantic_trigger_token(token: &str) -> bool {
    matches!(
        token,
        "activation"
            | "curation"
            | "evaluation"
            | "governance"
            | "loadpath"
            | "materialization"
            | "optimization"
            | "proposal"
            | "registry"
            | "telemetry"
            | "validation"
            | "verification"
    )
}

fn is_semantic_stop_word(token: &str) -> bool {
    matches!(
        token,
        "actual"
            | "after"
            | "before"
            | "could"
            | "covering"
            | "creation"
            | "future"
            | "measurable"
            | "naming"
            | "reusable"
            | "should"
            | "through"
            | "verification"
            | "wrote"
    )
}

/// Extract a bounded textual completion signal from Agent Execution output.
///
/// Service results commonly store chat output under the `output` key.  When a
/// provider returns a different bounded JSON shape, falling back to the compact
/// JSON string preserves replay evidence without copying prompts or context
/// bodies; the proposal command still records only character counts and refs.
fn agent_execution_output_text(output: &serde_json::Value) -> String {
    output
        .get("output")
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| output.to_string())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use macaca_kernel::executor::{TaskId, TokenUsage};

    use super::*;

    #[test]
    fn command_uses_only_bounded_refs_for_successful_task_completion() {
        let app_id = ApplicationId::new();
        let result = TaskResult {
            task_id: TaskId::new(),
            success: true,
            output: "raw task output that must not be copied into proposal bodies".into(),
            error: None,
            artifacts: vec!["/tmp/very-useful-artifact.txt".into()],
            completed_at: Utc::now(),
            tokens_used: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        let command = build_skill_experience_proposal_command(
            &app_id,
            "session-live-loop",
            "worker",
            &result,
            "trace-live-loop",
        )
        .expect("successful task should produce a proposal command");

        assert_eq!(command.scope.application_id, Some(app_id));
        assert_eq!(command.candidate.task_id, result.task_id.to_string());
        assert!(command.candidate.verified_terminal_success);
        assert_eq!(
            command.candidate.evidence_gate,
            SkillExperienceEvidenceGateStatus::Accepted
        );
        assert!(command.candidate.evidence_ids.iter().any(|id| id
            .starts_with("eventlog://sessions/session-live-loop/skill_self_evolution_observer/")));
        assert!(
            !command
                .candidate
                .bounded_summary
                .contains("raw task output"),
            "bounded summaries must not copy raw task output"
        );
        assert!(
            !command
                .candidate
                .reusable_procedure
                .contains("raw task output"),
            "procedure text must rely on refs instead of raw output"
        );
        assert!(command.candidate.validate().is_ok());
    }

    #[test]
    fn command_is_skipped_without_replayable_completion_evidence() {
        let app_id = ApplicationId::new();
        let result = TaskResult {
            task_id: TaskId::new(),
            success: true,
            output: "  ".into(),
            error: None,
            artifacts: Vec::new(),
            completed_at: Utc::now(),
            tokens_used: None,
        };

        assert!(build_skill_experience_proposal_command(
            &app_id,
            "session-empty",
            "worker",
            &result,
            "trace-empty"
        )
        .is_none());
    }

    #[test]
    fn agent_execution_metadata_artifact_ref_becomes_proposal_artifact_evidence() {
        let app_id = ApplicationId::new();
        let task_id = TaskId::new();
        let mut metadata = BTreeMap::new();
        metadata.insert("artifact_ref".into(), "tool:file_write:abcd1234".into());
        metadata.insert("artifact_digest".into(), "digest://artifact/1".into());
        let result = AgentExecutionResult {
            application_id: app_id,
            session_id: "session-artifact-evidence".into(),
            task_id: Some(task_id),
            target_agent: "coordinator".into(),
            status: AgentExecutionStatus::Completed,
            output: serde_json::json!({
                "output": "agent execution completed with a written artifact"
            }),
            context_snapshot: None,
            trace: TraceContext::new("test-artifact-evidence"),
            metadata,
        };

        let task_result = task_result_from_agent_execution_result(&result);
        let command = build_skill_experience_proposal_command(
            &app_id,
            &result.session_id,
            &result.target_agent,
            &task_result,
            &result.trace.trace_id,
        )
        .expect("artifact-backed execution should produce a proposal command");

        assert!(command
            .candidate
            .bounded_summary
            .contains("artifact_count=1"));
        assert_eq!(
            command
                .candidate
                .metadata
                .get("evidence_ref.artifact_0")
                .map(String::as_str),
            Some("tool:file_write:abcd1234")
        );
    }

    #[test]
    fn command_derives_semantic_skill_creator_identity_from_bounded_completion_signal() {
        let app_id = ApplicationId::new();
        let result = TaskResult {
            task_id: TaskId::new(),
            success: true,
            output: "Wrote a reusable verification note covering materialization, proposal-linked Skill package creation, registry load-path visibility, usage telemetry, semantic skill naming, and measurable follow-up optimization.".into(),
            error: None,
            artifacts: vec!["tool:file_write:semantic-live-note".into()],
            completed_at: Utc::now(),
            tokens_used: None,
        };

        let command = build_skill_experience_proposal_command(
            &app_id,
            "session-semantic-identity",
            "coordinator",
            &result,
            "trace-semantic-identity",
        )
        .expect("bounded semantic completion should produce a proposal command");

        assert_eq!(
            command.candidate.target_skill_name.as_deref(),
            Some("materialization-proposal-linked-skill-package-registry-load-path")
        );
        assert!(command
            .candidate
            .reusable_procedure
            .contains("materialization"));
        assert!(command.candidate.reusable_procedure.contains("telemetry"));
        assert!(
            !command
                .candidate
                .reusable_procedure
                .contains("semantic-live-note"),
            "Skill Creator-facing trigger text must not copy raw artifact refs"
        );
        assert!(command.candidate.validate().is_ok());
    }

    #[test]
    fn semantic_identity_uses_priority_phrases_across_realistic_artifact_summary() {
        let phrases = semantic_trigger_phrases(
            "Self-Evolution Materialization. Proposal-Linked Skill Creation. \
             Every Skill package should be traceable. Registry Load-Path Visibility. \
             Usage Telemetry and measurable follow-up optimization.",
        );

        assert_eq!(
            phrases,
            vec![
                "materialization",
                "proposal-linked",
                "skill-package",
                "registry-load-path"
            ]
        );
    }

    #[tokio::test]
    async fn agent_execution_result_converts_to_bounded_completion_event() {
        let app_id = ApplicationId::new();
        let task_id = TaskId::new();
        let result = AgentExecutionResult {
            application_id: app_id,
            session_id: "session-agent-execution".into(),
            task_id: Some(task_id),
            target_agent: "coordinator".into(),
            status: AgentExecutionStatus::Completed,
            output: serde_json::json!({
                "output": "agent execution completed with reusable evidence"
            }),
            context_snapshot: None,
            trace: TraceContext::new("test-agent-execution-result-observer"),
            metadata: BTreeMap::new(),
        };

        let output = agent_execution_output_text(&result.output);
        let task_result = TaskResult {
            task_id,
            success: true,
            output,
            error: None,
            artifacts: Vec::new(),
            completed_at: Utc::now(),
            tokens_used: None,
        };

        let command = build_skill_experience_proposal_command(
            &app_id,
            &result.session_id,
            &result.target_agent,
            &task_result,
            &result.trace.trace_id,
        )
        .expect("completed agent execution should produce a proposal command");

        assert_eq!(command.candidate.task_id, task_id.to_string());
        assert_eq!(command.candidate.agent_name.as_deref(), Some("coordinator"));
        assert!(
            command
                .candidate
                .evidence_ids
                .iter()
                .any(|id| id
                    == "trace://service.agent_execution/test-agent-execution-result-observer")
        );
        assert!(
            !command
                .candidate
                .bounded_summary
                .contains("agent execution completed"),
            "proposal summaries must carry counts and refs instead of raw output"
        );
    }
}
