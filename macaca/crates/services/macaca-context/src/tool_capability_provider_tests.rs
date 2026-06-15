use macaca_proto::{
    CapabilityToolDescriptor, CapabilityToolOriginKind, IndustrialToolDescriptor,
    ToolCatalogPlanResult, ToolFamilyRef, ToolHiddenReason, ToolPlanEntry, TraceContext,
};

use crate::{
    tool_capability_provider_arc, ContextAssembleInput, ContextComposeContext,
    ContextProviderStage, ContextReportBuilder, ToolCapabilityIndex, ToolCapabilityReport,
};

fn descriptor(id: &str, name: &str, family: &str) -> IndustrialToolDescriptor {
    let base = CapabilityToolDescriptor::new(
        "service.fixture",
        "provider.fixture",
        format!("capability.{family}.{name}"),
        name,
        "fixture",
        serde_json::json!({"type": "object"}),
        CapabilityToolOriginKind::Mcp,
    )
    .unwrap();
    IndustrialToolDescriptor::new(id, name, name, ToolFamilyRef::new(family).unwrap(), base)
        .unwrap()
}

#[tokio::test]
async fn tool_capability_context_provider_emits_compact_index_only() {
    let visible = ToolPlanEntry::visible(descriptor("tool.web.lookup", "lookup", "web"), vec![]);
    let hidden = ToolPlanEntry::hidden(
        descriptor("tool.memory.recall", "recall", "memory"),
        ToolHiddenReason::MissingSecret,
        vec!["secret-ref".into()],
    );
    let plan = ToolCatalogPlanResult {
        trace: TraceContext::new("trace-context-tools"),
        visible: vec![visible],
        hidden: vec![hidden],
        conflicts: Vec::new(),
        audit_refs: Vec::new(),
        captured_at: chrono::Utc::now(),
        metadata: Default::default(),
    };
    let provider = tool_capability_provider_arc(ToolCapabilityIndex::from_plan(&plan));

    assert_eq!(provider.stage(), ContextProviderStage::CapabilityIndex);

    let input = ContextAssembleInput::unscoped("agent", "model", vec![], Default::default());
    let ctx = ContextComposeContext {
        assemble_input: &input,
    };
    let outcome = provider.contribute(&ctx).await.unwrap();

    assert_eq!(outcome.candidates.len(), 1);
    let content = &outcome.candidates[0].content;
    assert!(content.contains("visible_count=\"1\""));
    assert!(content.contains("hidden_count=\"1\""));
    assert!(content.contains("family name=\"web\""));
    assert!(!content.contains("secret-ref"));
    assert!(!content.contains("parameters_schema"));
}

#[test]
fn context_report_records_tool_capability_counts() {
    let report = ContextReportBuilder::new("test-engine")
        .tool_capabilities(ToolCapabilityReport {
            provider_id: "tool_capability_index".into(),
            visible_count: 2,
            hidden_count: 1,
            skipped_count: 3,
            conflict_count: 1,
            reason_counts: [("missing_secret".into(), 1)].into_iter().collect(),
        })
        .build();

    assert_eq!(report.tool_capabilities.len(), 1);
    assert_eq!(report.tool_capabilities[0].hidden_count, 1);
    assert_eq!(
        report.tool_capabilities[0]
            .reason_counts
            .get("missing_secret"),
        Some(&1)
    );
}
