//! Integration-style tests for capability [`ContextProvider`]s (skills / MCP / runtime tools).
//!
//! These exercise the glue between neutral catalogs and composer outcomes without involving
//! `macaca-web` or a live MCP subprocess.

use std::sync::Arc;

use crate::capability::model::{
    DeclaredCapabilityDependency, McpCapabilityCatalog, McpServerCapabilitySummary,
    RuntimeToolCapabilityCatalog, SkillCapabilityCatalog, SkillCapabilityRecord,
};
use crate::{
    mcp_capability_provider_arc, runtime_tool_capability_provider_arc, skill_capability_provider_arc,
    CapabilityCollisionRecord, ContextAssembleInput, ContextComposeContext, ContextProviderStage,
    TrustLevel,
};

fn compose_ctx(input: &ContextAssembleInput) -> ContextComposeContext<'_> {
    ContextComposeContext {
        assemble_input: input,
    }
}

#[tokio::test]
async fn mcp_capability_provider_fence_untrusted_and_collision_diagnostics() {
    let catalog = Arc::new(McpCapabilityCatalog {
        servers: vec![McpServerCapabilitySummary {
            server_id: "server_a".into(),
            transport: "stdio".into(),
            lifecycle: "Global".into(),
            state: "Ready".into(),
            exposed_tools: vec!["dup".into(), "only_a".into()],
        }],
        collisions: vec![CapabilityCollisionRecord {
            local_key: "dup".into(),
            claimants: vec!["server_a".into(), "server_b".into()],
        }],
    });

    let provider = mcp_capability_provider_arc(Arc::clone(&catalog));
    assert_eq!(provider.stage(), ContextProviderStage::CapabilityIndex);

    let assemble = ContextAssembleInput::legacy("agent", "model", vec![], Default::default());
    let ctx = compose_ctx(&assemble);
    let out = provider.contribute(&ctx).await.unwrap();
    assert_eq!(out.candidates.len(), 1);
    let c = &out.candidates[0];
    assert!(c.content.starts_with("```mcp_capability_index"));
    assert!(matches!(c.trust, TrustLevel::Untrusted));

    assert_eq!(out.diagnostics.len(), 1);
    assert!(out.diagnostics[0].message.contains("dup"));
    assert!(out.diagnostics[0].message.contains("server_a"));
}

#[tokio::test]
async fn skill_capability_provider_dependency_gap_becomes_diagnostics() {
    let catalog = Arc::new(SkillCapabilityCatalog {
        entries: vec![SkillCapabilityRecord {
            stable_id: "skill:application:demo".into(),
            name: "demo".into(),
            description: "uses figma mcp".into(),
            location_ref: "/skills/demo/SKILL.md".into(),
            source_label: "app".into(),
            source_scope: "application".into(),
            declared_dependencies: vec![DeclaredCapabilityDependency {
                capability_id: "mcp_server:figma".into(),
                notes: None,
            }],
        }],
        filtered: Vec::new(),
        truncated: false,
    });

    // Ready set intentionally omits `figma` so the provider should surface a diagnostic.
    let ready = Arc::new(vec!["other".into()]);
    let provider = skill_capability_provider_arc(catalog, ready);

    let assemble = ContextAssembleInput::legacy("agent", "model", vec![], Default::default());
    let out = provider
        .contribute(&compose_ctx(&assemble))
        .await
        .unwrap();
    assert_eq!(out.candidates.len(), 1);
    assert!(out.diagnostics.iter().any(|d| d.message.contains("figma")));
}

#[tokio::test]
async fn runtime_tool_capability_provider_emits_sorted_names() {
    let catalog = Arc::new(RuntimeToolCapabilityCatalog {
        tool_names: vec!["z_last".into(), "a_first".into(), "a_first".into()],
    });

    let provider = runtime_tool_capability_provider_arc(catalog);

    let assemble = ContextAssembleInput::legacy("agent", "model", vec![], Default::default());
    let out = provider
        .contribute(&compose_ctx(&assemble))
        .await
        .unwrap();
    assert_eq!(out.candidates.len(), 1);
    let body = &out.candidates[0].content;
    let z = body.find("z_last").expect("z_last present");
    let a = body.find("a_first").expect("a_first present");
    assert!(a < z, "names should be sorted for stable context");
}
