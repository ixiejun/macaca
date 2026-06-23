use crate::source_artifact::resolve_source_artifact_ref;
use chrono::Utc;
use macaca_proto::{AgentId, AgentManifest, AgentState, Capability, Permission, PermissionLevel};

use super::todos::{required_session_id, SessionQuery};
use crate::application_shell_adapter::select_app_scoped_agent_manifests;

/// Provider-neutral fixture agent names for manifest dedup tests (Object Mother).
const FIXTURE_ENTRY_AGENT: &str = "entry-agent";
const FIXTURE_PLAN_AGENT: &str = "plan-agent";

fn test_agent_manifest(name: &str, capability: &str) -> AgentManifest {
    AgentManifest {
        id: AgentId::new(),
        name: name.to_string(),
        capabilities: vec![Capability {
            name: capability.to_string(),
            description: String::new(),
        }],
        permission: Permission {
            level: PermissionLevel::User,
            allowed_tools: Vec::new(),
            allowed_paths: Vec::new(),
            network_access: false,
        },
        state: AgentState::Created,
        created_at: Utc::now(),
        model: String::new(),
    }
}

#[test]
fn app_scoped_agent_selection_deduplicates_previous_same_name_agents() {
    let previous_entry = test_agent_manifest(FIXTURE_ENTRY_AGENT, "todo_goal_management");
    let app_entry = test_agent_manifest(FIXTURE_ENTRY_AGENT, "coding_session_coordination");
    let planner = test_agent_manifest(FIXTURE_PLAN_AGENT, "code_change_planning");
    let coder = test_agent_manifest("coder", "patch_authoring");
    let reviewer = test_agent_manifest("reviewer", "structured_review");
    let previous_planner = test_agent_manifest(FIXTURE_PLAN_AGENT, "todo_planning");
    let runtime_ids = vec![app_entry.id, planner.id, coder.id, reviewer.id];
    let declared_names = vec![
        FIXTURE_ENTRY_AGENT.to_string(),
        FIXTURE_PLAN_AGENT.to_string(),
        "coder".to_string(),
        "reviewer".to_string(),
    ];

    let selected = select_app_scoped_agent_manifests(
        vec![
            previous_entry,
            planner.clone(),
            coder.clone(),
            reviewer.clone(),
            previous_planner,
            app_entry.clone(),
        ],
        &runtime_ids,
        Some(&declared_names),
    );

    assert_eq!(selected.len(), 4);
    assert_eq!(
        selected
            .iter()
            .map(|agent| agent.name.as_str())
            .collect::<Vec<_>>(),
        vec![FIXTURE_ENTRY_AGENT, FIXTURE_PLAN_AGENT, "coder", "reviewer"]
    );
    assert_eq!(selected[0].id, app_entry.id);
    assert_eq!(
        selected[0].capabilities[0].name,
        "coding_session_coordination"
    );
}

#[test]
fn resolves_short_event_ref_against_requested_session() {
    let resolved = resolve_source_artifact_ref("session-a", "event/42").unwrap();
    assert_eq!(resolved.session_id, "session-a");
    assert_eq!(resolved.seq, 42);
    assert_eq!(resolved.canonical_ref, "events/session-a/00000042");
}

#[test]
fn resolves_canonical_event_ref_when_session_matches() {
    let resolved = resolve_source_artifact_ref("session-a", "events/session-a/00000007").unwrap();
    assert_eq!(resolved.session_id, "session-a");
    assert_eq!(resolved.seq, 7);
    assert_eq!(resolved.canonical_ref, "events/session-a/00000007");
}

#[test]
fn rejects_cross_session_refs() {
    let error = resolve_source_artifact_ref("session-a", "events/session-b/00000007").unwrap_err();
    assert!(error.contains("cross-session"));
}

#[test]
fn unsupported_provider_refs_return_error_message() {
    let error = resolve_source_artifact_ref("session-a", "workspace-memory").unwrap_err();
    assert!(error.contains("not backed by EventLog retrieval"));
}

#[test]
fn required_session_id_rejects_missing_or_blank_values() {
    let missing = required_session_id(&SessionQuery::default(), "list_todos").unwrap_err();
    assert_eq!(missing.0, axum::http::StatusCode::BAD_REQUEST);

    let blank = required_session_id(
        &SessionQuery {
            session_id: Some("  ".into()),
        },
        "list_todos",
    )
    .unwrap_err();
    assert_eq!(blank.0, axum::http::StatusCode::BAD_REQUEST);
}

#[test]
fn required_session_id_trims_and_accepts_current_session() {
    let query = SessionQuery {
        session_id: Some(" session-a ".into()),
    };
    let session_id = match required_session_id(&query, "list_todos") {
        Ok(session_id) => session_id,
        Err(_) => panic!("expected required_session_id to accept a non-empty session id"),
    };

    assert_eq!(session_id, "session-a");
}
