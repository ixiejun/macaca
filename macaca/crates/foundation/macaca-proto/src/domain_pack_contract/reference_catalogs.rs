use std::collections::{BTreeMap, BTreeSet};

use crate::{
    workbench, DRIVER_SERVICE_ID, LLM_SERVICE_ID, MCP_SERVICE_ID, MEMORY_SERVICE_ID,
    TASK_SERVICE_ID,
};

use super::industrial_reference_catalogs::industrial_reference_domain_pack_definitions;
use super::model::{
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackSdkMetadata, DomainPackStability,
};

/// Return the data-only reference packs shipped by the provider-neutral platform.
///
/// These entries are descriptors, not implementations.  They document capability families that
/// already exist behind service/facade boundaries so SDKs can explain the platform without the
/// base OS importing optional package providers or branching on application behavior.
pub fn reference_domain_pack_definitions() -> Vec<DomainPackDefinition> {
    let mut definitions = vec![
        foundation_pack_definition(),
        developer_pack_definition(),
        knowledge_pack_definition(),
    ];
    definitions.extend(industrial_reference_domain_pack_definitions());
    definitions
}

/// Foundation pack metadata for generic host primitives already exposed as services.
pub fn foundation_pack_definition() -> DomainPackDefinition {
    pack_definition(
        "pack.foundation.v1",
        "foundation",
        None,
        [
            workbench::file::SERVICE_ID,
            workbench::config::SERVICE_ID,
            workbench::sandbox::SERVICE_ID,
            workbench::process::SERVICE_ID,
            workbench::approval::SERVICE_ID,
        ],
        [
            (
                "pack.foundation.files",
                ["read", "write", "snapshot"].as_slice(),
            ),
            ("pack.foundation.config", ["read", "write"].as_slice()),
            ("pack.foundation.process", ["spawn", "terminate"].as_slice()),
        ],
        [
            (
                workbench::file::SERVICE_ID,
                ["file.read", "file.write"].as_slice(),
            ),
            (
                workbench::process::SERVICE_ID,
                ["process.spawn", "process.kill"].as_slice(),
            ),
        ],
        "sdk.packs.foundation",
    )
}

/// Developer pack metadata for code/workbench services already routed through service calls.
pub fn developer_pack_definition() -> DomainPackDefinition {
    pack_definition(
        "pack.developer.v1",
        "developer",
        None,
        [
            workbench::git::SERVICE_ID,
            workbench::code_intelligence::SERVICE_ID,
            workbench::diagnostics::SERVICE_ID,
            workbench::review::SERVICE_ID,
            DRIVER_SERVICE_ID,
            MCP_SERVICE_ID,
        ],
        [
            ("pack.developer.repository", ["read", "write"].as_slice()),
            ("pack.developer.tooling", ["invoke", "inspect"].as_slice()),
            ("pack.developer.review", ["request", "record"].as_slice()),
        ],
        [
            (
                workbench::git::SERVICE_ID,
                ["git.status", "git.diff"].as_slice(),
            ),
            (
                DRIVER_SERVICE_ID,
                ["driver.status", "driver.catalog"].as_slice(),
            ),
            (MCP_SERVICE_ID, ["mcp.catalog", "mcp.invoke"].as_slice()),
        ],
        "sdk.packs.developer",
    )
}

/// Knowledge pack metadata for memory/context/intelligence services already serviceized.
pub fn knowledge_pack_definition() -> DomainPackDefinition {
    pack_definition(
        "pack.knowledge.v1",
        "knowledge",
        None,
        [
            MEMORY_SERVICE_ID,
            LLM_SERVICE_ID,
            TASK_SERVICE_ID,
            workbench::code_intelligence::SERVICE_ID,
        ],
        [
            ("pack.knowledge.memory", ["recall", "remember"].as_slice()),
            (
                "pack.knowledge.reasoning",
                ["summarize", "classify"].as_slice(),
            ),
            ("pack.knowledge.task", ["query", "snapshot"].as_slice()),
        ],
        [
            (
                MEMORY_SERVICE_ID,
                ["memory.recall", "memory.remember"].as_slice(),
            ),
            (LLM_SERVICE_ID, ["llm.chat", "llm.route"].as_slice()),
            (TASK_SERVICE_ID, ["task.query", "task.snapshot"].as_slice()),
        ],
        "sdk.packs.knowledge",
    )
}

fn pack_definition<const S: usize, const P: usize, const C: usize>(
    pack_id: &str,
    family_id: &str,
    parent_pack_id: Option<&str>,
    services: [&str; S],
    permission_scopes: [(&str, &[&str]); P],
    command_schemas: [(&str, &[&str]); C],
    client_namespace: &str,
) -> DomainPackDefinition {
    let services = services
        .into_iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let permission_scopes = permission_scopes
        .into_iter()
        .flat_map(|(scope, verbs)| verbs.iter().map(move |verb| format!("{scope}.{verb}")))
        .collect::<BTreeSet<_>>();
    let mut service_command_schemas = command_schemas
        .into_iter()
        .map(|(service, commands)| {
            (
                service.to_string(),
                commands
                    .iter()
                    .map(ToString::to_string)
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for service in &services {
        service_command_schemas
            .entry(service.clone())
            .or_insert_with(|| BTreeSet::from([format!("{service}.command.v1")]));
    }
    let service_result_schemas = service_command_schemas
        .iter()
        .map(|(service, commands)| {
            (
                service.clone(),
                commands
                    .iter()
                    .map(|command| format!("{command}.result"))
                    .collect(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let metadata = DomainPackMetadata {
        family_id: family_id.into(),
        parent_pack_id: parent_pack_id.map(ToString::to_string),
        version: "v1".into(),
        stability: DomainPackStability::Preview,
        availability: DomainPackAvailability::Available,
        service_command_schemas,
        service_result_schemas,
        permission_scopes,
        source_attribution: BTreeSet::from(["macaca-industrial-pack-catalog.v1".into()]),
        migration_notes: Vec::new(),
        policy_template: DomainPackPolicyTemplate {
            timeout_ms: Some(30_000),
            max_retries: Some(1),
            budget_units: Some(1),
            allow_network: None,
        },
        data_governance: DomainPackDataGovernance {
            classification: "bounded_metadata".into(),
            retention_policy: "service_defined".into(),
            redaction_policy: "audit_redacted".into(),
        },
        sdk: DomainPackSdkMetadata {
            client_namespace: client_namespace.into(),
            docs_url: "docs://macaca/developer-pack-platform".into(),
            examples: Vec::new(),
        },
        diagnostics: DomainPackDiagnostics {
            health_probe: "service.health".into(),
            unavailable_reason: "provider_not_registered".into(),
            replay_schema: "pack.discovery.v1".into(),
        },
        compatibility: DomainPackCompatibility {
            version_range: "^1".into(),
            parent_version_range: String::new(),
            service_version_ranges: BTreeMap::new(),
        },
        provider_descriptors: BTreeMap::new(),
    };
    DomainPackDefinition::with_metadata(pack_id, metadata, services)
}
