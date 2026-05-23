//! CLI adapters for Skill governance and curation operations.
//!
//! The CLI is a presentation shell, so these commands build provider-neutral
//! SDK DTOs, call the focused Skill client, and print bounded JSON summaries.
//! They do not read skill packages, classify curation actions, or implement
//! lifecycle policy locally.

use std::collections::BTreeMap;

use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use macaca_sdk::{
    SkillAuthorKind, SkillCurationLifecycleAction, SkillCurationLifecycleCommand,
    SkillCurationRollbackCommand, SkillCurationRunCommand, SkillEvolutionPromoteDraftCommand,
    SkillEvolutionRejectDraftCommand, SkillExperienceProposalSnapshotCommand,
    SkillGovernanceSnapshotCommand, SkillServicePolicyHints, SkillServiceScope, SystemSkillClient,
    UnavailableSystemSkillClient,
};
use serde_json::Value;
use tracing::{info, warn};

/// Shared operator evidence refs accepted by mutating CLI commands.
#[derive(Debug, Clone, Default)]
pub struct SkillCliEvidenceRefs {
    pub reason: Option<String>,
    pub evidence_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub approval_ref: Option<String>,
}

/// Optional live runtime target for CLI Skill commands.
///
/// CLI is a presentation shell, not a provider composition root.  When an
/// application id is supplied, CLI reaches the already-running Web shell through
/// its public HTTP facade so every command observes the same live
/// `service.skill` runtime that the frontend and API use.  When no app id is
/// supplied, commands deliberately keep the SDK Null Object path as a diagnostic
/// unavailable state instead of pretending a live runtime exists.
#[derive(Debug, Clone, Default)]
pub struct SkillCliRuntimeTarget {
    pub app_id: Option<String>,
    pub api_base: Option<String>,
}

impl SkillCliRuntimeTarget {
    fn live_client(&self) -> MacacaResult<Option<LiveSkillOperationsClient>> {
        let Some(app_id) = self
            .app_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };
        uuid::Uuid::parse_str(app_id).map_err(|error| {
            MacacaError::Config(format!(
                "skill live operations require a valid app id: {error}"
            ))
        })?;
        let api_base = self
            .api_base
            .clone()
            .or_else(|| std::env::var("MACACA_API_BASE").ok())
            .unwrap_or_else(|| "http://127.0.0.1:3001".into());
        Ok(Some(LiveSkillOperationsClient::new(
            api_base,
            app_id.into(),
        )?))
    }
}

/// Lifecycle actions exposed by the CLI transport.
#[derive(Debug, Clone, Copy)]
pub enum SkillCliLifecycleAction {
    Pin,
    Unpin,
    Archive,
    Restore,
    Quarantine,
    ReleaseQuarantine,
    Reject,
}

impl SkillCliLifecycleAction {
    fn into_service_action(self) -> SkillCurationLifecycleAction {
        match self {
            Self::Pin => SkillCurationLifecycleAction::Pin,
            Self::Unpin => SkillCurationLifecycleAction::Unpin,
            Self::Archive => SkillCurationLifecycleAction::Archive,
            Self::Restore => SkillCurationLifecycleAction::Restore,
            Self::Quarantine => SkillCurationLifecycleAction::Quarantine,
            Self::ReleaseQuarantine => SkillCurationLifecycleAction::ReleaseQuarantine,
            Self::Reject => SkillCurationLifecycleAction::Reject,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pin => "pin",
            Self::Unpin => "unpin",
            Self::Archive => "archive",
            Self::Restore => "restore",
            Self::Quarantine => "quarantine",
            Self::ReleaseQuarantine => "release_quarantine",
            Self::Reject => "reject",
        }
    }

    fn route_segment(self) -> &'static str {
        match self {
            Self::ReleaseQuarantine => "release-quarantine",
            _ => self.as_str(),
        }
    }
}

/// Print a sanitized Skill operations snapshot through the SDK Skill facade.
pub async fn execute_skill_operations_snapshot(target: SkillCliRuntimeTarget) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let response = client.get("").await?;
        info!(
            app_id = %client.app_id,
            "CLI emitted live Skill operations snapshot from public Web API facade"
        );
        return print_json(response);
    }

    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new("cli-skill-operations-snapshot");
    let scope = SkillServiceScope::default();
    info!(
        trace_id = %trace.trace_id,
        command = "skill.operations.snapshot",
        "CLI forwarding Skill operations snapshot through SDK Skill facade"
    );
    let governance = client
        .governance_snapshot(SkillGovernanceSnapshotCommand {
            trace: trace.clone(),
            scope: scope.clone(),
            include_archived: true,
            lifecycle_filters: Vec::new(),
        })
        .await?;
    let proposals = client
        .skill_experience_snapshot(SkillExperienceProposalSnapshotCommand {
            trace: trace.clone(),
            scope,
            include_discarded: false,
        })
        .await?;
    info!(
        trace_id = %trace.trace_id,
        governance_records = governance.records.len(),
        proposal_count = proposals.proposals.len(),
        "CLI emitted bounded Skill operations snapshot"
    );
    print_json(serde_json::json!({
        "trace_id": trace.trace_id,
        "governance_records": governance.records.len(),
        "proposal_count": proposals.proposals.len(),
        "governance": governance,
        "proposals": proposals,
    }))
}

/// Run deterministic curation analysis or approval-gated apply through SDK.
pub async fn execute_skill_curation_run(
    target: SkillCliRuntimeTarget,
    dry_run: bool,
    stale_after_days: i64,
    narrow_use_threshold: u64,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let trace_label = if dry_run {
            "cli-skill-curation-run"
        } else {
            "cli-skill-curation-apply"
        };
        let response = client
            .post(
                if dry_run {
                    "/curation/run"
                } else {
                    "/curation/apply"
                },
                live_operator_payload(refs)
                    .with_curation_thresholds(stale_after_days, narrow_use_threshold),
            )
            .await?;
        info!(
            app_id = %client.app_id,
            command = trace_label,
            dry_run,
            "CLI forwarded live Skill curation command through public Web API facade"
        );
        return print_json(response);
    }

    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new(if dry_run {
        "cli-skill-curation-run"
    } else {
        "cli-skill-curation-apply"
    });
    let command = SkillCurationRunCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        dry_run,
        stale_after_days,
        narrow_use_threshold,
        approval_refs: optional_vec(refs.approval_ref),
        policy_decision_refs: optional_vec(refs.policy_ref),
        audit_event_ids: Vec::new(),
        policy: policy_hints(),
    };
    info!(
        trace_id = %trace.trace_id,
        command = if dry_run { "skill.curation.run" } else { "skill.curation.apply" },
        dry_run,
        stale_after_days,
        narrow_use_threshold,
        approval_count = command.approval_refs.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "CLI forwarding Skill curation command through SDK Skill facade"
    );
    print_sdk_result(trace, client.curation_run(command).await)
}

/// Forward one lifecycle mutation request through the SDK Skill facade.
pub async fn execute_skill_lifecycle(
    target: SkillCliRuntimeTarget,
    action: SkillCliLifecycleAction,
    skill_id: String,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let path = format!(
            "/lifecycle/{}/{}",
            action.route_segment(),
            url_segment(&skill_id)
        );
        let response = client.post(&path, live_operator_payload(refs)).await?;
        info!(
            app_id = %client.app_id,
            action = action.as_str(),
            skill_id = %skill_id,
            "CLI forwarded live Skill lifecycle command through public Web API facade"
        );
        return print_json(response);
    }

    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new(format!("cli-skill-lifecycle-{}", action.as_str()));
    let command = SkillCurationLifecycleCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        skill_id: skill_id.clone(),
        name: skill_id,
        source: "cli-skill-operations".into(),
        source_scope: "operator".into(),
        author_kind: SkillAuthorKind::Unknown,
        reason: refs
            .reason
            .unwrap_or_else(|| "cli_skill_lifecycle_request".into()),
        evidence_ids: optional_vec(refs.evidence_ref),
        task_id: None,
        policy_decision_refs: optional_vec(refs.policy_ref),
        policy: policy_hints(),
    };
    info!(
        trace_id = %trace.trace_id,
        command = "skill.curation.lifecycle",
        action = action.as_str(),
        skill_id = %command.skill_id,
        evidence_count = command.evidence_ids.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "CLI forwarding Skill lifecycle command through SDK Skill facade"
    );
    print_sdk_result(
        trace,
        client
            .curation_lifecycle(action.into_service_action(), command)
            .await,
    )
}

/// Restore governance state from a curation rollback memento ref through SDK.
pub async fn execute_skill_rollback(
    target: SkillCliRuntimeTarget,
    rollback_ref: String,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let response = client
            .post(
                "/curation/rollback",
                live_operator_payload(refs).with_rollback_ref(rollback_ref.clone()),
            )
            .await?;
        info!(
            app_id = %client.app_id,
            rollback_ref = %rollback_ref,
            "CLI forwarded live Skill rollback command through public Web API facade"
        );
        return print_json(response);
    }

    let client = UnavailableSystemSkillClient;
    let trace = TraceContext::new("cli-skill-curation-rollback");
    let command = SkillCurationRollbackCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        rollback_ref,
        approval_refs: optional_vec(refs.approval_ref),
        policy_decision_refs: optional_vec(refs.policy_ref),
        audit_event_ids: Vec::new(),
        policy: policy_hints(),
    };
    info!(
        trace_id = %trace.trace_id,
        command = "skill.curation.rollback",
        approval_count = command.approval_refs.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "CLI forwarding Skill rollback command through SDK Skill facade"
    );
    print_sdk_result(trace, client.curation_rollback(command).await)
}

/// Promote or reject a draft proposal through the SDK Skill facade.
pub async fn execute_skill_proposal_decision(
    target: SkillCliRuntimeTarget,
    proposal_id: String,
    promote: bool,
    refs: SkillCliEvidenceRefs,
) -> MacacaResult<()> {
    if let Some(client) = target.live_client()? {
        let decision = if promote { "promote" } else { "reject" };
        let path = format!("/proposals/{}/{}", url_segment(&proposal_id), decision);
        let response = client.post(&path, live_operator_payload(refs)).await?;
        info!(
            app_id = %client.app_id,
            proposal_id = %proposal_id,
            decision,
            "CLI forwarded live Skill proposal decision through public Web API facade"
        );
        return print_json(response);
    }

    let client = UnavailableSystemSkillClient;
    if promote {
        let trace = TraceContext::new("cli-skill-proposal-promote");
        let command = SkillEvolutionPromoteDraftCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            proposal_id,
            reason: refs
                .reason
                .unwrap_or_else(|| "cli_skill_proposal_promote".into()),
            evidence_ids: optional_vec(refs.evidence_ref),
            policy_decision_refs: optional_vec(refs.policy_ref),
            policy: policy_hints(),
        };
        info!(
            trace_id = %trace.trace_id,
            command = "skill.evolution.promote_draft",
            proposal_id = %command.proposal_id,
            evidence_count = command.evidence_ids.len(),
            policy_decision_count = command.policy_decision_refs.len(),
            "CLI forwarding Skill proposal promotion through SDK Skill facade"
        );
        return print_sdk_result(trace, client.promote_skill_draft(command).await);
    }

    let trace = TraceContext::new("cli-skill-proposal-reject");
    let command = SkillEvolutionRejectDraftCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        proposal_id,
        rationale: refs
            .reason
            .unwrap_or_else(|| "cli_skill_proposal_reject".into()),
        evidence_ids: optional_vec(refs.evidence_ref),
        policy_decision_refs: optional_vec(refs.policy_ref),
        policy: policy_hints(),
    };
    info!(
        trace_id = %trace.trace_id,
        command = "skill.evolution.reject_draft",
        proposal_id = %command.proposal_id,
        evidence_count = command.evidence_ids.len(),
        policy_decision_count = command.policy_decision_refs.len(),
        "CLI forwarding Skill proposal rejection through SDK Skill facade"
    );
    print_sdk_result(trace, client.reject_skill_draft(command).await)
}

fn optional_vec(value: Option<String>) -> Vec<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .into_iter()
        .collect()
}

fn policy_hints() -> SkillServicePolicyHints {
    SkillServicePolicyHints {
        required_permissions: Vec::new(),
        entitlement_ready: None,
        package_ready: None,
        metadata: BTreeMap::new(),
    }
}

/// Small HTTP Adapter over the public Web Skill operations API.
///
/// The adapter intentionally knows only route transport details: app id, base
/// URL, request method, and bounded JSON payloads.  It does not decode Skill
/// service DTOs or make lifecycle decisions, preserving Web/service ownership
/// of curation semantics while still allowing CLI to prove a live runtime path.
#[derive(Debug, Clone)]
struct LiveSkillOperationsClient {
    api_base: String,
    app_id: String,
    http: reqwest::Client,
}

impl LiveSkillOperationsClient {
    fn new(api_base: String, app_id: String) -> MacacaResult<Self> {
        let api_base = normalize_api_base(&api_base)?;
        Ok(Self {
            api_base,
            app_id,
            http: reqwest::Client::new(),
        })
    }

    async fn get(&self, path: &str) -> MacacaResult<Value> {
        let url = self.url(path);
        info!(
            app_id = %self.app_id,
            url = %url,
            method = "GET",
            "CLI requesting live Skill operations state from Web API facade"
        );
        let response = self.http.get(url).send().await.map_err(http_error)?;
        decode_live_response(response).await
    }

    async fn post(&self, path: &str, payload: LiveSkillOperatorPayload) -> MacacaResult<Value> {
        let url = self.url(path);
        info!(
            app_id = %self.app_id,
            url = %url,
            method = "POST",
            evidence_count = payload.evidence_ids.len(),
            policy_decision_count = payload.policy_decision_refs.len(),
            approval_count = payload.approval_refs.len(),
            "CLI forwarding live Skill operation to Web API facade"
        );
        let response = self
            .http
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(http_error)?;
        decode_live_response(response).await
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/api/apps/{}/skills/operations{}",
            self.api_base,
            url_segment(&self.app_id),
            path
        )
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct LiveSkillOperatorPayload {
    reason: String,
    rationale: String,
    evidence_ids: Vec<String>,
    policy_decision_refs: Vec<String>,
    approval_refs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rollback_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stale_after_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    narrow_use_threshold: Option<u64>,
    source: String,
    source_scope: String,
}

impl LiveSkillOperatorPayload {
    fn with_curation_thresholds(
        mut self,
        stale_after_days: i64,
        narrow_use_threshold: u64,
    ) -> Self {
        self.stale_after_days = Some(stale_after_days);
        self.narrow_use_threshold = Some(narrow_use_threshold);
        self
    }

    fn with_rollback_ref(mut self, rollback_ref: String) -> Self {
        self.rollback_ref = Some(rollback_ref);
        self
    }
}

fn live_operator_payload(refs: SkillCliEvidenceRefs) -> LiveSkillOperatorPayload {
    let reason = refs
        .reason
        .unwrap_or_else(|| "cli_skill_operation".into())
        .trim()
        .to_string();
    let reason = if reason.is_empty() {
        "cli_skill_operation".into()
    } else {
        reason
    };
    LiveSkillOperatorPayload {
        rationale: reason.clone(),
        reason,
        evidence_ids: optional_vec(refs.evidence_ref),
        policy_decision_refs: optional_vec(refs.policy_ref),
        approval_refs: optional_vec(refs.approval_ref),
        rollback_ref: None,
        stale_after_days: None,
        narrow_use_threshold: None,
        source: "cli-skill-operations".into(),
        source_scope: "operator".into(),
    }
}

fn normalize_api_base(value: &str) -> MacacaResult<String> {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(MacacaError::Config(
            "skill live operations require a non-empty API base".into(),
        ));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(MacacaError::Config(
            "skill live operations API base must start with http:// or https://".into(),
        ));
    }
    Ok(trimmed.to_string())
}

fn url_segment(value: &str) -> String {
    value.replace('%', "%25").replace('/', "%2F")
}

fn http_error(error: reqwest::Error) -> MacacaError {
    MacacaError::Config(format!("skill live operations API request failed: {error}"))
}

async fn decode_live_response(response: reqwest::Response) -> MacacaResult<Value> {
    let status = response.status();
    let data = response.json::<Value>().await.map_err(http_error)?;
    if status.is_success() {
        return Ok(data);
    }
    let reason = data
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("skill live operations request failed");
    Err(MacacaError::Config(format!(
        "skill live operations API returned HTTP {status}: {reason}"
    )))
}

fn print_sdk_result<T: serde::Serialize>(
    trace: TraceContext,
    result: MacacaResult<T>,
) -> MacacaResult<()> {
    match result {
        Ok(result) => print_json(serde_json::json!({
            "trace_id": trace.trace_id,
            "status": "ok",
            "result": result,
        })),
        Err(error) => {
            warn!(
                trace_id = %trace.trace_id,
                error_class = "unavailable_or_denied",
                "CLI Skill command returned structured service error"
            );
            print_json(serde_json::json!({
                "trace_id": trace.trace_id,
                "status": "unavailable_or_denied",
                "error": error.to_string(),
            }))
        }
    }
}

fn print_json(value: serde_json::Value) -> MacacaResult<()> {
    let rendered = serde_json::to_string_pretty(&value).map_err(MacacaError::from)?;
    println!("{rendered}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        live_operator_payload, normalize_api_base, SkillCliEvidenceRefs, SkillCliLifecycleAction,
        SkillCliRuntimeTarget,
    };

    #[tokio::test]
    async fn cli_skill_snapshot_uses_sdk_null_object() {
        super::execute_skill_operations_snapshot(SkillCliRuntimeTarget::default())
            .await
            .unwrap();
    }

    #[test]
    fn cli_skill_live_target_builds_public_web_api_url() {
        let target = SkillCliRuntimeTarget {
            app_id: Some("2c96f3f2-b78c-5edd-beb4-740c8c004910".into()),
            api_base: Some("http://127.0.0.1:3001/".into()),
        };
        let client = target.live_client().unwrap().unwrap();
        assert_eq!(
            client.url("/curation/run"),
            "http://127.0.0.1:3001/api/apps/2c96f3f2-b78c-5edd-beb4-740c8c004910/skills/operations/curation/run"
        );
    }

    #[test]
    fn cli_skill_live_payload_keeps_operator_refs_bounded() {
        let payload = live_operator_payload(SkillCliEvidenceRefs {
            reason: Some("approved curation".into()),
            evidence_ref: Some("evidence://run/1".into()),
            policy_ref: Some("policy://decision/1".into()),
            approval_ref: Some("approval://operator/1".into()),
        })
        .with_curation_thresholds(14, 2);
        let json = serde_json::to_value(payload).unwrap();
        assert_eq!(json["reason"], "approved curation");
        assert_eq!(json["evidence_ids"][0], "evidence://run/1");
        assert_eq!(json["policy_decision_refs"][0], "policy://decision/1");
        assert_eq!(json["approval_refs"][0], "approval://operator/1");
        assert_eq!(json["stale_after_days"], 14);
        assert_eq!(json["narrow_use_threshold"], 2);
    }

    #[test]
    fn cli_skill_live_target_rejects_non_http_api_base() {
        assert!(normalize_api_base("file:///tmp/socket").is_err());
        assert_eq!(
            normalize_api_base("http://127.0.0.1:3001/").unwrap(),
            "http://127.0.0.1:3001"
        );
    }

    #[test]
    fn cli_skill_lifecycle_uses_public_route_segments() {
        assert_eq!(SkillCliLifecycleAction::Pin.route_segment(), "pin");
        assert_eq!(
            SkillCliLifecycleAction::ReleaseQuarantine.route_segment(),
            "release-quarantine"
        );
    }

    #[test]
    fn cli_skill_operations_do_not_import_runtime_or_web() {
        let source = include_str!("skill_operations.rs");
        let runtime_host_import = ["macaca", "_runtime_host::"].concat();
        let web_import = ["macaca", "_web::"].concat();
        let provider_state_symbol = ["SkillProvider", "GovernanceState"].concat();
        assert!(source.contains("UnavailableSystemSkillClient"));
        assert!(!source.contains(&runtime_host_import));
        assert!(!source.contains(&web_import));
        assert!(!source.contains(&provider_state_symbol));
    }
}
