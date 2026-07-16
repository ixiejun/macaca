use super::summarization_strategy::{
    checkpoint_ref, LongDocumentExecutionPlan, SummarizationStrategyKind,
};

#[test]
fn summarization_commands_select_provider_neutral_strategies() {
    let expected = [
        (
            "summarization.summarize",
            SummarizationStrategyKind::Extractive,
        ),
        (
            "summarization.summarize_with_citations",
            SummarizationStrategyKind::Hybrid,
        ),
        (
            "summarization.summarize_many",
            SummarizationStrategyKind::LongDocumentSynthesis,
        ),
        (
            "summarization.summarize_conversation",
            SummarizationStrategyKind::RollingSummary,
        ),
        (
            "summarization.compress_context",
            SummarizationStrategyKind::ContextCompression,
        ),
        (
            "summarization.evaluate_summary",
            SummarizationStrategyKind::Evaluation,
        ),
    ];
    for (command, strategy) in expected {
        assert_eq!(
            SummarizationStrategyKind::for_command(command),
            Some(strategy)
        );
    }
    assert_eq!(
        SummarizationStrategyKind::for_command_and_mode(
            "summarization.summarize",
            Some("abstractive")
        ),
        Some(SummarizationStrategyKind::Abstractive)
    );
    assert!(checkpoint_ref(SummarizationStrategyKind::LongDocumentSynthesis, "trace").is_some());
    assert!(checkpoint_ref(SummarizationStrategyKind::Extractive, "trace").is_none());
}

#[test]
fn long_document_plan_is_bounded_and_resumable_without_source_content() {
    let plan = LongDocumentExecutionPlan::for_trace("trace", 64);
    assert_eq!(plan.chunk_refs.len(), 32);
    assert_eq!(plan.map_summary_refs.len(), 32);
    assert!(plan.checkpoint_ref.contains("long_document_synthesis"));
    assert_eq!(
        plan.partial_failure_policy,
        "retain_successful_maps_and_resume_failed_maps"
    );
}
