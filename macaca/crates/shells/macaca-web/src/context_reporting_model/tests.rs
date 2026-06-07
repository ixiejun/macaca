//! Unit tests for active-recall fallback policy on the reporting ChatModel wrapper.

use macaca_context::ContextPreflightRecallConfig;

use super::ContextReportingChatModel;

#[test]
fn active_recall_fallback_enables_bounded_retry_after_composer_timeout() {
    let active_vector_memory = macaca_proto::config::ActiveVectorMemoryContextConfig {
        enabled: true,
        timeout_ms: 5_000,
        max_chars: 2_048,
        max_tokens: 512,
        ..Default::default()
    };
    let preflight_cfg = ContextPreflightRecallConfig {
        enabled: false,
        allowed_tool_names: Vec::new(),
        timeout_ms: 1_500,
        max_chars: 4_000,
        max_tokens: 1_000,
        fatal_on_failure: false,
    };

    let fallback = ContextReportingChatModel::active_recall_fallback_config(
        &active_vector_memory,
        &preflight_cfg,
        false,
    );

    assert!(fallback.enabled);
    assert_eq!(fallback.timeout_ms, 5_000);
    assert_eq!(fallback.max_chars, 2_048);
    assert_eq!(fallback.max_tokens, 512);
    assert!(!fallback.fatal_on_failure);
}

#[test]
fn active_recall_fallback_stays_disabled_when_composer_proves_recall() {
    let active_vector_memory = macaca_proto::config::ActiveVectorMemoryContextConfig {
        enabled: true,
        timeout_ms: 5_000,
        ..Default::default()
    };
    let preflight_cfg = ContextPreflightRecallConfig {
        enabled: false,
        allowed_tool_names: Vec::new(),
        timeout_ms: 1_500,
        max_chars: 4_000,
        max_tokens: 1_000,
        fatal_on_failure: false,
    };

    let fallback = ContextReportingChatModel::active_recall_fallback_config(
        &active_vector_memory,
        &preflight_cfg,
        true,
    );

    assert!(!fallback.enabled);
    assert_eq!(fallback.timeout_ms, 1_500);
}
