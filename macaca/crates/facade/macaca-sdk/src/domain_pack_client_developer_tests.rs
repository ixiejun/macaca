use macaca_proto::domain_pack_contract::{
    developer_browser_automation::{
        DEVELOPER_BROWSER_AUTOMATION_PACK_ID, DEVELOPER_BROWSER_AUTOMATION_SERVICE_ID,
    },
    developer_ci::{DEVELOPER_CI_PACK_ID, DEVELOPER_CI_SERVICE_ID},
    developer_code::{DEVELOPER_CODE_PACK_ID, DEVELOPER_CODE_SERVICE_ID},
    developer_design_tools::{DEVELOPER_DESIGN_TOOLS_PACK_ID, DEVELOPER_DESIGN_TOOLS_SERVICE_ID},
    developer_issue_tracker::{
        DEVELOPER_ISSUE_TRACKER_PACK_ID, DEVELOPER_ISSUE_TRACKER_SERVICE_ID,
    },
    developer_repository::{DEVELOPER_REPOSITORY_PACK_ID, DEVELOPER_REPOSITORY_SERVICE_ID},
    developer_terminal::{DEVELOPER_TERMINAL_PACK_ID, DEVELOPER_TERMINAL_SERVICE_ID},
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig,
};

use super::*;

// Developer SDK tests validate catalog discovery only. The SDK must not create
// parser, language-server, repository, CI, issue tracker, terminal, process,
// browser, design-tool, remote-service, mock, or unavailable providers.

#[tokio::test]
async fn catalog_client_discovers_developer_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            DEVELOPER_CODE_PACK_ID,
            DEVELOPER_CODE_SERVICE_ID,
            "code.inspect_workspace",
            "developer_code_provider_not_installed",
            "language-intelligence",
        ),
        (
            DEVELOPER_REPOSITORY_PACK_ID,
            DEVELOPER_REPOSITORY_SERVICE_ID,
            "repository.status",
            "developer_repository_provider_not_installed",
            "local-vcs",
        ),
        (
            DEVELOPER_CI_PACK_ID,
            DEVELOPER_CI_SERVICE_ID,
            "ci.inspect_status",
            "developer_ci_provider_not_installed",
            "pipeline-service",
        ),
        (
            DEVELOPER_ISSUE_TRACKER_PACK_ID,
            DEVELOPER_ISSUE_TRACKER_SERVICE_ID,
            "issue_tracker.search_issues",
            "developer_issue_tracker_provider_not_installed",
            "issue-model",
        ),
        (
            DEVELOPER_TERMINAL_PACK_ID,
            DEVELOPER_TERMINAL_SERVICE_ID,
            "terminal.plan_spawn",
            "developer_terminal_provider_not_installed",
            "process-runtime",
        ),
        (
            DEVELOPER_BROWSER_AUTOMATION_PACK_ID,
            DEVELOPER_BROWSER_AUTOMATION_SERVICE_ID,
            "browser.plan_context",
            "developer_browser_automation_provider_not_installed",
            "browser-runtime",
        ),
        (
            DEVELOPER_DESIGN_TOOLS_PACK_ID,
            DEVELOPER_DESIGN_TOOLS_SERVICE_ID,
            "design_tools.inspect_node",
            "developer_design_tools_provider_not_installed",
            "design-read",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid developer id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("developer descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/developer"));
    }
}

#[tokio::test]
async fn catalog_client_reports_developer_unavailable_reasons() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let command = DomainPackResolveCommand {
        declaration: AppServiceContractConfig {
            optional_packs: vec![
                DEVELOPER_CODE_PACK_ID.into(),
                DEVELOPER_REPOSITORY_PACK_ID.into(),
                DEVELOPER_CI_PACK_ID.into(),
                DEVELOPER_ISSUE_TRACKER_PACK_ID.into(),
                DEVELOPER_TERMINAL_PACK_ID.into(),
                DEVELOPER_BROWSER_AUTOMATION_PACK_ID.into(),
                DEVELOPER_DESIGN_TOOLS_PACK_ID.into(),
            ],
            ..Default::default()
        },
    };

    let result = client.resolve_declaration(&command).await.unwrap();

    for (pack_id, reason) in [
        (
            DEVELOPER_CODE_PACK_ID,
            "developer_code_provider_not_installed",
        ),
        (
            DEVELOPER_REPOSITORY_PACK_ID,
            "developer_repository_provider_not_installed",
        ),
        (DEVELOPER_CI_PACK_ID, "developer_ci_provider_not_installed"),
        (
            DEVELOPER_ISSUE_TRACKER_PACK_ID,
            "developer_issue_tracker_provider_not_installed",
        ),
        (
            DEVELOPER_TERMINAL_PACK_ID,
            "developer_terminal_provider_not_installed",
        ),
        (
            DEVELOPER_BROWSER_AUTOMATION_PACK_ID,
            "developer_browser_automation_provider_not_installed",
        ),
        (
            DEVELOPER_DESIGN_TOOLS_PACK_ID,
            "developer_design_tools_provider_not_installed",
        ),
    ] {
        assert!(result
            .effective
            .unresolved_optional_packs
            .contains(&pack_id.to_string()));
        assert_eq!(
            result.effective.unavailable_pack_reasons.get(pack_id),
            Some(&reason.to_string())
        );
    }
}
