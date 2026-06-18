//! Autonomy schedule serviceization policy checks.
//!
//! These assertions keep presentation and Web route code on serviceized
//! Scheduler/Autonomy paths. The module is separate from token scanning because
//! it validates route-shape contracts instead of generic forbidden literals.

use super::support::{read_optional_presentation_source, repository_root, workspace_root};

pub fn assert_autonomy_schedule_management_uses_serviceized_paths_only() {
    let root = workspace_root();
    let repo = repository_root();
    let frontend_facade = repo.join("frontend/lib/autonomy.ts");
    if let Some(facade) = read_optional_presentation_source(&frontend_facade) {
        assert!(
            facade.contains("/autonomy"),
            "frontend autonomy facade must call the serviceized /autonomy namespace"
        );
        assert!(
            !facade.contains("/api/apps/${encodeURIComponent(appId)}/schedules"),
            "frontend autonomy facade must not call the retired direct schedule namespace"
        );
        assert!(
            !facade.contains("heartbeat_wake"),
            "frontend schedule mutations must not expose heartbeat native cadence as a Scheduler target"
        );
    }

    let schedule_editor_path = repo.join("frontend/components/autonomy/ScheduleEditorDrawer.tsx");
    if let Some(schedule_editor) = read_optional_presentation_source(&schedule_editor_path) {
        assert!(
            !schedule_editor.contains("Heartbeat wake")
                && !schedule_editor.contains("wake_scope_key")
                && !schedule_editor.contains("wake_reason_code"),
            "application schedule editor must not expose heartbeat native cadence fields"
        );
    }

    let routes = std::fs::read_to_string(
        root.join("crates/shells/macaca-web/src/routes/autonomy_schedules.rs"),
    )
    .expect("routes/autonomy_schedules.rs should be readable");
    let serviceized_section = routes
        .split("// Serviceized application autonomy schedule routes")
        .nth(1)
        .and_then(|tail| tail.split("// Event Log API").next())
        .expect("serviceized autonomy schedule section should exist");
    assert!(
        !serviceized_section.contains("macaca_task::TaskScheduler"),
        "serviceized autonomy routes must use Scheduler service clients, not retired TaskScheduler construction"
    );
}
