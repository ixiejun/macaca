use std::collections::{BTreeMap, BTreeSet};

use super::developer_browser_automation::*;
use super::developer_ci::*;
use super::developer_code::*;
use super::developer_common::{DeveloperCommandEnvelope, DeveloperError};
use super::developer_design_tools::*;
use super::developer_issue_tracker::*;
use super::developer_repository::*;
use super::developer_terminal::*;
use super::*;

// Developer tests validate provider-neutral contract shape only. They do not
// start language servers, parsers, scanners, Git clients, CI clients, issue
// tracker clients, processes, browsers, design tools, remotes, or provider
// APIs. Fixtures use refs and hashes instead of raw source, diffs, patches,
// logs, artifacts, output streams, screenshots, DOM, comments, credentials, or
// provider payloads.

#[test]
fn developer_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            developer_code_pack_definition(),
            DEVELOPER_CODE_PACK_ID,
            DEVELOPER_CODE_SERVICE_ID,
            DEVELOPER_CODE_COMMANDS,
            "developer_code_provider_not_installed",
            "language-intelligence",
            "code.inspect_workspace",
        ),
        (
            developer_repository_pack_definition(),
            DEVELOPER_REPOSITORY_PACK_ID,
            DEVELOPER_REPOSITORY_SERVICE_ID,
            DEVELOPER_REPOSITORY_COMMANDS,
            "developer_repository_provider_not_installed",
            "local-vcs",
            "repository.status",
        ),
        (
            developer_ci_pack_definition(),
            DEVELOPER_CI_PACK_ID,
            DEVELOPER_CI_SERVICE_ID,
            DEVELOPER_CI_COMMANDS,
            "developer_ci_provider_not_installed",
            "pipeline-service",
            "ci.inspect_status",
        ),
        (
            developer_issue_tracker_pack_definition(),
            DEVELOPER_ISSUE_TRACKER_PACK_ID,
            DEVELOPER_ISSUE_TRACKER_SERVICE_ID,
            DEVELOPER_ISSUE_TRACKER_COMMANDS,
            "developer_issue_tracker_provider_not_installed",
            "issue-model",
            "issue_tracker.search_issues",
        ),
        (
            developer_terminal_pack_definition(),
            DEVELOPER_TERMINAL_PACK_ID,
            DEVELOPER_TERMINAL_SERVICE_ID,
            DEVELOPER_TERMINAL_COMMANDS,
            "developer_terminal_provider_not_installed",
            "process-runtime",
            "terminal.plan_spawn",
        ),
        (
            developer_browser_automation_pack_definition(),
            DEVELOPER_BROWSER_AUTOMATION_PACK_ID,
            DEVELOPER_BROWSER_AUTOMATION_SERVICE_ID,
            DEVELOPER_BROWSER_AUTOMATION_COMMANDS,
            "developer_browser_automation_provider_not_installed",
            "browser-runtime",
            "browser.plan_context",
        ),
        (
            developer_design_tools_pack_definition(),
            DEVELOPER_DESIGN_TOOLS_PACK_ID,
            DEVELOPER_DESIGN_TOOLS_SERVICE_ID,
            DEVELOPER_DESIGN_TOOLS_COMMANDS,
            "developer_design_tools_provider_not_installed",
            "design-read",
            "design_tools.inspect_node",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.developer.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/developer"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("developer descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_developer_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let code = find_pack(&definitions, DEVELOPER_CODE_PACK_ID);
    let repository = find_pack(&definitions, DEVELOPER_REPOSITORY_PACK_ID);
    let ci = find_pack(&definitions, DEVELOPER_CI_PACK_ID);
    let issue = find_pack(&definitions, DEVELOPER_ISSUE_TRACKER_PACK_ID);
    let terminal = find_pack(&definitions, DEVELOPER_TERMINAL_PACK_ID);
    let browser = find_pack(&definitions, DEVELOPER_BROWSER_AUTOMATION_PACK_ID);
    let design = find_pack(&definitions, DEVELOPER_DESIGN_TOOLS_PACK_ID);

    assert_eq!(
        code.metadata
            .provider_descriptors
            .get("language-intelligence")
            .and_then(|descriptor| descriptor.metadata.get("raw_source_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        repository
            .metadata
            .provider_descriptors
            .get("local-vcs")
            .and_then(|descriptor| descriptor.metadata.get("raw_diff_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        ci.metadata
            .provider_descriptors
            .get("log-artifact-service")
            .and_then(|descriptor| descriptor.metadata.get("raw_logs_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        issue
            .metadata
            .provider_descriptors
            .get("comment-attachment")
            .and_then(|descriptor| descriptor.metadata.get("raw_comments_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        terminal
            .metadata
            .provider_descriptors
            .get("process-runtime")
            .and_then(|descriptor| descriptor.metadata.get("raw_output_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        browser
            .metadata
            .provider_descriptors
            .get("browser-runtime")
            .and_then(|descriptor| descriptor.metadata.get("raw_dom_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        design
            .metadata
            .provider_descriptors
            .get("design-read")
            .and_then(|descriptor| descriptor.metadata.get("raw_design_in_trace"))
            .map(String::as_str),
        Some("false")
    );
}

#[test]
fn developer_command_and_result_dtos_are_serde_compatible() {
    let envelope = DeveloperCommandEnvelope {
        subject_ref: "developer:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "synthetic".into())]),
        cursor: None,
        page_size: Some(10),
        idempotency_key: Some("idem-developer".into()),
    };

    let values = [
        serde_json::to_value(CodeInspectWorkspaceCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(RepositoryStatusCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(CiInspectStatusCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(IssueTrackerSearchIssuesCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(TerminalPlanSpawnCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(BrowserPlanContextCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(DesignToolsInspectNodeCommand { request: envelope }).unwrap(),
        serde_json::to_value(CodeResultEnvelope::<CodeWorkspace> {
            status: CodeResultStatus::ApprovalRequired,
            data: None,
            page: None,
            error: Some(DeveloperError {
                code: "approval_required".into(),
                message: "synthetic approval required".into(),
                retryable: false,
                trace_safe_detail: Some("patch_apply".into()),
            }),
        })
        .unwrap(),
        serde_json::to_value(RepositoryResultEnvelope::<RepositoryHandle> {
            status: RepositoryResultStatus::DirtyWorktree,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(CiResultEnvelope::<CiRun> {
            status: CiResultStatus::StaleStatus,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(IssueTrackerResultEnvelope::<IssueItem> {
            status: IssueTrackerResultStatus::TransitionDenied,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(TerminalResultEnvelope::<TerminalSession> {
            status: TerminalResultStatus::InvalidCommand,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(BrowserResultEnvelope::<BrowserPage> {
            status: BrowserResultStatus::ActionabilityFailed,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(DesignToolsResultEnvelope::<DesignFile> {
            status: DesignToolsResultStatus::WriteDenied,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn developer_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&developer_code_descriptor_hashes()),
        hash_values(&developer_repository_descriptor_hashes()),
        hash_values(&developer_ci_descriptor_hashes()),
        hash_values(&developer_issue_tracker_descriptor_hashes()),
        hash_values(&developer_terminal_descriptor_hashes()),
        hash_values(&developer_browser_automation_descriptor_hashes()),
        hash_values(&developer_design_tools_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 7);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn developer_validation_helpers_are_provider_neutral() {
    let workspace = CodeWorkspace {
        workspace_ref: "workspace".into(),
        file_count: 10,
        ..Default::default()
    };
    let repository = RepositoryHandle {
        repository_ref: "repo".into(),
        workspace_ref: "workspace".into(),
        ..Default::default()
    };
    let ci_run = CiRun {
        run_ref: "run".into(),
        status: CiStatus {
            status_ref: "status".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let issue = IssueItem {
        issue_ref: "issue".into(),
        version_hash: "version".into(),
        ..Default::default()
    };
    let process = TerminalProcessSpec {
        spec_ref: "spec".into(),
        command_hash: "command".into(),
        env_policy: TerminalEnvironmentPolicy {
            policy_ref: "env".into(),
            ..Default::default()
        },
        workdir_scope: TerminalWorkdirScope {
            scope_ref: "workdir".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let context = BrowserContextProfile {
        profile_ref: "context".into(),
        isolation_mode: "isolated".into(),
        ..Default::default()
    };
    let file = DesignFile {
        file_ref: "file".into(),
        version_hash: "version".into(),
        ..Default::default()
    };

    assert!(workspace.is_bounded(100));
    assert!(repository.is_scoped());
    assert!(ci_run.has_terminal_or_active_status());
    assert!(issue.has_version_precondition());
    assert!(process.is_policy_bound());
    assert!(context.is_isolated());
    assert!(file.has_version_precondition());
}

#[test]
fn invalid_developer_descriptor_is_rejected() {
    let mut invalid = developer_code_pack_definition();
    invalid.pack_id = "developer.code.v1".into();

    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

fn hash_values<T: serde::Serialize>(value: &T) -> Vec<String> {
    let json = serde_json::to_value(value).expect("descriptor hash fixture serializes");
    json.as_object()
        .expect("descriptor hashes serialize as object")
        .values()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn find_pack<'a>(
    definitions: &'a [DomainPackDefinition],
    pack_id: &str,
) -> &'a DomainPackDefinition {
    definitions
        .iter()
        .find(|definition| definition.pack_id == pack_id)
        .expect("industrial catalog includes specialized developer descriptor")
}
