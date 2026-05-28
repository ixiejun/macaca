//! Application-neutral proof for the Codex-class workbench proposal.
//!
//! The fixture runs a real repository mutation through generic workbench
//! services. Product behavior stays in this test-level application fixture;
//! runtime, kernel, SDK, and service layers only see provider-neutral commands.

mod support;

use macaca_proto::workbench::{
    code_intelligence as code, diagnostics as diag, file, git, hook, interaction, process, review,
    sandbox,
};
use macaca_proto::{ServiceHealth, WorkbenchCommandStatus, TOOL_SERVICE_ID};
use support::*;

#[tokio::test]
async fn application_neutral_fixture_runs_full_workbench_workflow() {
    let manifest = proof_manifest();
    assert_manifest_declares_full_workbench_surface(&manifest);

    let repo = proof_repo();
    let scope = proof_scope(&manifest);
    let mut audit_refs = Vec::new();
    let providers = ProofProviders::new();
    let mut streamed = providers.app_protocol.subscribe_notifications();

    eprintln!("codex_class_application_neutral_proof event=manifest_declared");
    initialize_app_protocol(&providers.app_protocol, &scope).await;
    subscribe_global_app_events(&providers.app_protocol, &scope).await;

    let thread = start_thread(&providers.interaction, &scope, &mut audit_refs).await;
    let turn = start_turn(
        &providers.interaction,
        &scope,
        &thread.thread_id,
        &mut audit_refs,
    )
    .await;
    stream_event(
        &providers.app_protocol,
        &scope,
        "interaction.turn.started",
        interaction::SERVICE_ID,
        Some(&thread.thread_id),
        Some(&turn.turn_id),
    )
    .await;

    let read = call_service::<file::FileServiceResponse>(
        &providers.file,
        file::READ_COMMAND,
        file::FileReadRequest {
            target: proof_path(repo.path(), "README.md"),
            inline_budget_bytes: 1024,
        },
        "trace-proof-file-read",
    )
    .await;
    assert!(matches!(
        read.output,
        Some(file::FileServiceResponse::Read { .. })
    ));
    push_audit(&read, &mut audit_refs);
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "file.read.completed",
        file::SERVICE_ID,
    )
    .await;

    let search = call_service::<code::CodeIntelligenceResponse>(
        &providers.code,
        code::SEARCH_COMMAND,
        code::CodeSearchRequest {
            workspace_root: repo.path().to_string_lossy().to_string(),
            query: "proof_marker".into(),
            include_globs: Vec::new(),
            max_results: 5,
            snippet_budget_bytes: 32,
        },
        "trace-proof-code-search",
    )
    .await;
    assert!(matches!(
        search.output,
        Some(code::CodeIntelligenceResponse::Search { ref results, .. }) if results.len() == 1
    ));
    push_audit(&search, &mut audit_refs);
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "code.search.completed",
        code::SERVICE_ID,
    )
    .await;

    let approval_ref = approve_patch(&providers.approval, &mut audit_refs).await;
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "approval.request.resolved",
        macaca_proto::workbench::approval::SERVICE_ID,
    )
    .await;

    run_hook(&providers.hook, &scope, true, &mut audit_refs).await;
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "hook.pre_tool.completed",
        hook::SERVICE_ID,
    )
    .await;

    let patched = call_service::<git::GitServiceResponse>(
        &providers.git,
        git::APPLY_PATCH_COMMAND,
        git::GitApplyPatchRequest {
            workspace_root: repo.path().to_string_lossy().to_string(),
            patch_text: proof_patch(),
            require_approval: true,
            approval_ref: Some(approval_ref),
            actor_ref: "actor://application-neutral-proof".into(),
        },
        "trace-proof-git-apply-patch",
    )
    .await;
    let marker = match patched.output {
        Some(git::GitServiceResponse::PatchApplied(provenance)) => {
            audit_refs.push(provenance.audit_ref);
            provenance.pre_change_marker
        }
        _ => panic!("expected git patch provenance"),
    };
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "git.patch.applied",
        git::SERVICE_ID,
    )
    .await;

    let sandbox_record =
        prepare_sandbox(&providers.sandbox, repo.path(), &scope, &mut audit_refs).await;
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "sandbox.environment.ready",
        sandbox::SERVICE_ID,
    )
    .await;

    let test_run = run_process_test(
        &providers.process,
        repo.path(),
        &scope,
        &sandbox_record,
        &mut audit_refs,
    )
    .await;
    assert_eq!(test_run.record.exit_code, Some(0));
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "process.output.completed",
        process::SERVICE_ID,
    )
    .await;

    let tool_result = invoke_skill_tool().await;
    assert_eq!(tool_result.status, "ok");
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "tool.skill.completed",
        TOOL_SERVICE_ID,
    )
    .await;

    run_hook(&providers.hook, &scope, false, &mut audit_refs).await;
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "hook.post_tool.completed",
        hook::SERVICE_ID,
    )
    .await;

    append_interaction_item(
        &providers.interaction,
        &scope,
        &thread.thread_id,
        &turn.turn_id,
        interaction::InteractionItemKind::ToolResult,
        "application-neutral proof completed service-owned tool chain",
        &mut audit_refs,
    )
    .await;

    let review_ref = start_review(&providers.review, &audit_refs).await;
    let review_result = review_result(&providers.review, &review_ref, &mut audit_refs).await;
    assert_eq!(review_result.findings.len(), 1);
    assert!(review_result.artifact_ref.is_some());
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "review.result.completed",
        review::SERVICE_ID,
    )
    .await;

    providers
        .diagnostics
        .upsert_source(diag::DiagnosticsSourceSnapshot {
            source: diag::DiagnosticsSourceKind::Process,
            service_id: process::SERVICE_ID.into(),
            health: ServiceHealth::Healthy,
            summary: Some(summary("process proof output captured as bounded evidence")),
            artifact_refs: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
        .await;
    let trace_bundle = call_service::<diag::DiagnosticsServiceResponse>(
        &providers.diagnostics,
        diag::TRACE_BUNDLE_COMMAND,
        diag::DiagnosticsTraceBundleRequest {
            trace_refs: audit_refs.clone(),
            include_audit_refs: true,
            redaction_profile: diag::DiagnosticsRedactionProfile::default(),
        },
        "trace-proof-diagnostics-bundle",
    )
    .await;
    assert!(matches!(
        trace_bundle.output,
        Some(diag::DiagnosticsServiceResponse::TraceBundle(ref bundle)) if bundle.bounded
    ));
    push_audit(&trace_bundle, &mut audit_refs);
    stream_service_event(
        &providers.app_protocol,
        &scope,
        "diagnostics.trace_bundle.completed",
        diag::SERVICE_ID,
    )
    .await;

    let replay = call_service::<git::GitServiceResponse>(
        &providers.git,
        git::ROLLBACK_MARKER_REPLAY_COMMAND,
        git::GitRollbackMarkerReplayRequest {
            workspace_root: repo.path().to_string_lossy().to_string(),
            marker_ref: marker.marker_ref,
        },
        "trace-proof-git-rollback-replay",
    )
    .await;
    assert!(matches!(
        replay.output,
        Some(git::GitServiceResponse::RollbackReplay { audit_ref, .. })
            if audit_ref.starts_with("audit://git/rollback/")
    ));

    let snapshot = interaction_snapshot(&providers.interaction, &scope).await;
    assert!(matches!(
        snapshot.output,
        Some(interaction::InteractionResponse::Snapshot(state))
            if state.threads == 1 && state.turns == 1 && state.items >= 1
    ));

    let mut notification_count = 0;
    while notification_count < 11 {
        let notification = streamed.recv().await.unwrap();
        assert!(notification.event.summary.is_some());
        notification_count += 1;
    }
    eprintln!(
        "codex_class_application_neutral_proof event=workflow_complete notifications={} audits={}",
        notification_count,
        audit_refs.len()
    );
    assert!(audit_refs.len() >= 10);
    assert!(matches!(
        trace_bundle.status,
        WorkbenchCommandStatus::Completed
    ));
}
