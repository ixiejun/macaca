use super::ai_embedding::AI_EMBEDDING_COMMANDS;
use super::ai_llm::AI_LLM_COMMANDS;
use super::ai_model_evaluation::AI_MODEL_EVALUATION_COMMANDS;
use std::cell::Cell;

use super::ai_preflight::{
    AiPackCommandPreflight, AiPackCommandPreflightSpec, AiPackPreflightStatus,
};
use super::ai_rerank::AI_RERANK_COMMANDS;
use super::ai_speech::AI_SPEECH_COMMANDS;
use super::ai_vision::AI_VISION_COMMANDS;

// AI preflight tests stay at the generic contract layer. They prove every AI
// child pack can share one declaration/policy/entitlement/resource/approval
// Specification without constructing hosted models, local runtimes, OCR engines,
// speech engines, evaluation runners, plugins, or unavailable providers.

#[test]
fn ai_preflight_accepts_declared_scopes_for_all_ai_child_packs() {
    for case in ai_cases() {
        for scope in case.scopes {
            assert!(
                case.spec.declaration.validate_scope(scope).is_ok(),
                "{} scope {scope}",
                case.name
            );
        }

        let preflight = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
        assert!(case.spec.evaluate(&preflight).is_ok(), "{}", case.name);
    }
}

#[test]
fn ai_preflight_rejects_unknown_scopes_and_commands_before_provider_dispatch() {
    for case in ai_cases() {
        assert!(case
            .spec
            .declaration
            .validate_scope("ai.unknown.scope")
            .is_err());

        let preflight = AiPackCommandPreflight::allowed("unknown.command", case.primary_scope);
        let rejection = case.spec.evaluate(&preflight).unwrap_err();
        assert_eq!(rejection.status, AiPackPreflightStatus::Unsupported);
        assert_eq!(rejection.reason_code, "unsupported_command");
    }
}

#[test]
fn ai_preflight_rejects_policy_entitlement_host_and_quota_failures() {
    for case in ai_cases() {
        let mut policy_denied = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
        policy_denied.policy.allowed = false;
        assert_rejection(
            &case.spec,
            &policy_denied,
            AiPackPreflightStatus::Denied,
            "policy_denied",
        );

        let mut provider_unavailable =
            AiPackCommandPreflight::allowed(case.command, case.primary_scope);
        provider_unavailable.entitlement.provider_access = false;
        assert_rejection(
            &case.spec,
            &provider_unavailable,
            AiPackPreflightStatus::Unavailable,
            "provider_unavailable",
        );

        let mut entitlement_denied =
            AiPackCommandPreflight::allowed(case.command, case.primary_scope);
        entitlement_denied.entitlement.scope_granted = false;
        assert_rejection(
            &case.spec,
            &entitlement_denied,
            AiPackPreflightStatus::Denied,
            "entitlement_denied",
        );

        let mut host_disabled = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
        host_disabled.entitlement.host_capability_enabled = false;
        assert_rejection(
            &case.spec,
            &host_disabled,
            AiPackPreflightStatus::Unavailable,
            "host_capability_disabled",
        );

        let mut quota_exceeded = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
        quota_exceeded.reserved_resources.provider_calls = 0;
        assert_rejection(
            &case.spec,
            &quota_exceeded,
            AiPackPreflightStatus::QuotaExceeded,
            "resource_reservation_insufficient",
        );
    }
}

#[test]
fn ai_preflight_requires_approval_for_sensitive_or_long_running_commands() {
    for case in ai_cases() {
        let mut preflight =
            AiPackCommandPreflight::allowed(case.approval_command, case.primary_scope);
        preflight.approval = None;

        assert_rejection(
            &case.spec,
            &preflight,
            AiPackPreflightStatus::Denied,
            "approval_required",
        );
    }
}

#[test]
fn ai_preflight_guard_skips_provider_dispatch_for_rejected_paths() {
    for case in ai_cases() {
        let provider_calls = Cell::new(0_u32);

        for rejected in rejected_preflights(&case) {
            let result = case.spec.dispatch_after_preflight(&rejected, || {
                provider_calls.set(provider_calls.get() + 1);
                "provider-called"
            });

            assert!(result.is_err(), "{}", case.name);
            assert_eq!(
                provider_calls.get(),
                0,
                "{} rejected preflight must not dispatch provider",
                case.name
            );
        }

        let accepted = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
        let result = case.spec.dispatch_after_preflight(&accepted, || {
            provider_calls.set(provider_calls.get() + 1);
            "provider-called"
        });

        assert_eq!(result.unwrap(), "provider-called", "{}", case.name);
        assert_eq!(
            provider_calls.get(),
            1,
            "{} accepted preflight should dispatch exactly once",
            case.name
        );
    }
}

fn assert_rejection(
    spec: &AiPackCommandPreflightSpec,
    preflight: &AiPackCommandPreflight,
    status: AiPackPreflightStatus,
    reason_code: &str,
) {
    let rejection = spec.evaluate(preflight).unwrap_err();
    assert_eq!(rejection.status, status);
    assert_eq!(rejection.reason_code, reason_code);
}

fn rejected_preflights(case: &AiCase) -> Vec<AiPackCommandPreflight> {
    let mut unsupported_command =
        AiPackCommandPreflight::allowed("unsupported.command", case.primary_scope);
    unsupported_command.command_name = "unsupported.command".into();

    let unknown_scope = AiPackCommandPreflight::allowed(case.command, "ai.unknown.scope");

    let mut policy_denied = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
    policy_denied.policy.allowed = false;

    let mut provider_unavailable =
        AiPackCommandPreflight::allowed(case.command, case.primary_scope);
    provider_unavailable.entitlement.provider_access = false;

    let mut entitlement_denied = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
    entitlement_denied.entitlement.scope_granted = false;

    let mut host_disabled = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
    host_disabled.entitlement.host_capability_enabled = false;

    let mut quota_exceeded = AiPackCommandPreflight::allowed(case.command, case.primary_scope);
    quota_exceeded.reserved_resources.provider_calls = 0;

    let mut approval_required =
        AiPackCommandPreflight::allowed(case.approval_command, case.primary_scope);
    approval_required.approval = None;

    vec![
        unsupported_command,
        unknown_scope,
        policy_denied,
        provider_unavailable,
        entitlement_denied,
        host_disabled,
        quota_exceeded,
        approval_required,
    ]
}

struct AiCase {
    name: &'static str,
    spec: AiPackCommandPreflightSpec,
    scopes: &'static [&'static str],
    primary_scope: &'static str,
    command: &'static str,
    approval_command: &'static str,
}

fn ai_cases() -> Vec<AiCase> {
    vec![
        AiCase {
            name: "llm",
            spec: AiPackCommandPreflightSpec::new(
                AI_LLM_COMMANDS.iter().copied(),
                ["ai.llm.invoke", "ai.llm.route", "ai.llm.budget"],
                ["llm.chat", "llm.complete", "llm.route_model"],
            ),
            scopes: &["ai.llm.invoke", "ai.llm.route", "ai.llm.budget"],
            primary_scope: "ai.llm.invoke",
            command: "llm.estimate_tokens",
            approval_command: "llm.chat",
        },
        AiCase {
            name: "embedding",
            spec: AiPackCommandPreflightSpec::new(
                AI_EMBEDDING_COMMANDS.iter().copied(),
                ["ai.embedding.invoke", "ai.embedding.batch"],
                ["embedding.batch_embed"],
            ),
            scopes: &["ai.embedding.invoke", "ai.embedding.batch"],
            primary_scope: "ai.embedding.invoke",
            command: "embedding.estimate_cost",
            approval_command: "embedding.batch_embed",
        },
        AiCase {
            name: "rerank",
            spec: AiPackCommandPreflightSpec::new(
                AI_RERANK_COMMANDS.iter().copied(),
                ["ai.rerank.invoke", "ai.rerank.explain"],
                ["rerank.rerank", "rerank.batch_rerank"],
            ),
            scopes: &["ai.rerank.invoke", "ai.rerank.explain"],
            primary_scope: "ai.rerank.invoke",
            command: "rerank.inspect_model",
            approval_command: "rerank.batch_rerank",
        },
        AiCase {
            name: "vision",
            spec: AiPackCommandPreflightSpec::new(
                AI_VISION_COMMANDS.iter().copied(),
                ["ai.vision.invoke", "ai.vision.ocr", "ai.vision.moderate"],
                ["vision.analyze_video", "vision.moderate_visual"],
            ),
            scopes: &["ai.vision.invoke", "ai.vision.ocr", "ai.vision.moderate"],
            primary_scope: "ai.vision.invoke",
            command: "vision.extract_visual_evidence",
            approval_command: "vision.analyze_video",
        },
        AiCase {
            name: "speech",
            spec: AiPackCommandPreflightSpec::new(
                AI_SPEECH_COMMANDS.iter().copied(),
                [
                    "ai.speech.recognize",
                    "ai.speech.synthesize",
                    "ai.speech.translate",
                ],
                ["speech.speech_to_text", "speech.text_to_speech"],
            ),
            scopes: &[
                "ai.speech.recognize",
                "ai.speech.synthesize",
                "ai.speech.translate",
            ],
            primary_scope: "ai.speech.recognize",
            command: "speech.list_voices",
            approval_command: "speech.speech_to_text",
        },
        AiCase {
            name: "model-evaluation",
            spec: AiPackCommandPreflightSpec::new(
                AI_MODEL_EVALUATION_COMMANDS.iter().copied(),
                ["ai.eval.run", "ai.eval.dataset", "ai.eval.report"],
                [
                    "model_evaluation.run_eval",
                    "model_evaluation.export_report",
                ],
            ),
            scopes: &["ai.eval.run", "ai.eval.dataset", "ai.eval.report"],
            primary_scope: "ai.eval.run",
            command: "model_evaluation.calculate_metrics",
            approval_command: "model_evaluation.run_eval",
        },
    ]
}
