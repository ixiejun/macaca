//! Contract tests for skill runtime discovery, policy, and workspace projection.

use std::env;
use std::path::Path;

use crate::policy::os_matches_current;

use super::{path_belongs_to_snapshot_skill, SkillPolicy, SkillRuntime, SkillRuntimeOptions};

async fn write_skill(root: &Path, dir: &str, body: &str) {
    let skill_dir = root.join(dir);
    tokio::fs::create_dir_all(&skill_dir).await.unwrap();
    tokio::fs::write(skill_dir.join("SKILL.md"), body)
        .await
        .unwrap();
}

#[tokio::test]
async fn source_precedence_workspace_wins() {
    let app = tempfile::tempdir().unwrap();
    let ws = tempfile::tempdir().unwrap();
    write_skill(
        app.path(),
        "skills/demo",
        "---\nname: demo\ndescription: app\n---\napp",
    )
    .await;
    write_skill(
        ws.path(),
        "skills/demo",
        "---\nname: demo\ndescription: workspace\n---\nws",
    )
    .await;

    let snapshot = SkillRuntime
        .build_snapshot(
            "agent-alpha",
            SkillRuntimeOptions {
                workspace_dir: Some(ws.path().to_path_buf()),
                app_dir: Some(app.path().to_path_buf()),
                policy: SkillPolicy {
                    allow: Some(vec!["demo".into()]),
                    deny: Vec::new(),
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(snapshot.skills.len(), 1);
    assert_eq!(snapshot.skills[0].description, "workspace");
}

#[tokio::test]
async fn openclaw_metadata_filters_missing_env() {
    let app = tempfile::tempdir().unwrap();
    write_skill(
        app.path(),
        "skills/needs-env",
        "---\nname: needs-env\ndescription: env\nmetadata:\n  openclaw:\n    requires:\n      env: [MACACA_TEST_MISSING_ENV]\n---\nbody",
    )
    .await;

    let snapshot = SkillRuntime
        .build_snapshot(
            "agent-beta",
            SkillRuntimeOptions {
                app_dir: Some(app.path().to_path_buf()),
                policy: SkillPolicy {
                    allow: Some(vec!["needs-env".into()]),
                    deny: Vec::new(),
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(snapshot.skills.is_empty());
    assert_eq!(snapshot.filtered[0].reason, "missing_env");
}

#[test]
fn darwin_metadata_matches_macos_runtime() {
    assert!(os_matches_current("darwin") || env::consts::OS != "macos");
}

#[tokio::test]
async fn allowlist_limits_visible_skills() {
    let app = tempfile::tempdir().unwrap();
    write_skill(
        app.path(),
        "skills/a",
        "---\nname: a\ndescription: A\n---\nA",
    )
    .await;
    write_skill(
        app.path(),
        "skills/b",
        "---\nname: b\ndescription: B\n---\nB",
    )
    .await;

    let snapshot = SkillRuntime
        .build_snapshot(
            "agent-gamma",
            SkillRuntimeOptions {
                app_dir: Some(app.path().to_path_buf()),
                policy: SkillPolicy {
                    allow: Some(vec!["b".into()]),
                    deny: Vec::new(),
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(snapshot.skills.len(), 1);
    assert_eq!(snapshot.skills[0].name, "b");
}

#[tokio::test]
async fn snapshot_projects_visible_skill_into_workspace_available_skills() {
    let app = tempfile::tempdir().unwrap();
    let ws = tempfile::tempdir().unwrap();
    write_skill(
        app.path(),
        "skills/sample-skill",
        "---\nname: Sample Skill\ndescription: sample data helper\n---\nUse scripts/helper.py.",
    )
    .await;
    let script_dir = app.path().join("skills/sample-skill/scripts");
    tokio::fs::create_dir_all(&script_dir).await.unwrap();
    tokio::fs::write(script_dir.join("helper.py"), "print('ok')\n")
        .await
        .unwrap();

    let snapshot = SkillRuntime
        .build_snapshot(
            "agent-delta",
            SkillRuntimeOptions {
                workspace_dir: Some(ws.path().to_path_buf()),
                app_dir: Some(app.path().to_path_buf()),
                policy: SkillPolicy {
                    allow: Some(vec!["Sample Skill".into()]),
                    deny: Vec::new(),
                },
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let entry = &snapshot.skills[0];
    let projected_skill = ws.path().join("available_skills/sample_skill/SKILL.md");
    let projected_script = ws
        .path()
        .join("available_skills/sample_skill/scripts/helper.py");

    assert_eq!(entry.location, projected_skill);
    assert!(entry
        .source_location
        .ends_with("skills/sample-skill/SKILL.md"));
    assert!(projected_skill.exists());
    assert!(projected_script.exists());
    assert!(snapshot
        .prompt
        .contains("available_skills/sample_skill/SKILL.md"));
    assert!(path_belongs_to_snapshot_skill(&snapshot, &projected_script));
    assert!(path_belongs_to_snapshot_skill(
        &snapshot,
        &entry.source_base_dir.join("SKILL.md")
    ));
}
