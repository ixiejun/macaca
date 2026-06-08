//! Workspace projection for model-facing skill paths.
//!
//! Copies visible skill trees into `workspace/available_skills/<slug>/` so LLM
//! clients see stable relative paths without following symlinks outside the root.

use std::collections::HashMap;
use std::path::Path;

use macaca_proto::MacacaResult;
use tracing::{debug, warn};

use crate::agent_skill::SkillEntry;

/// Project visible skills into `workspace_dir/available_skills/` and rewrite locations.
pub(crate) async fn project_prompt_entries(
    entries: &mut [SkillEntry],
    workspace_dir: &Path,
) -> MacacaResult<()> {
    if entries.is_empty() {
        return Ok(());
    }

    let projection_root = workspace_dir.join("available_skills");
    tokio::fs::create_dir_all(&projection_root).await?;
    debug!(
        root = %projection_root.display(),
        count = entries.len(),
        "skill runtime: projecting skills into workspace"
    );

    let mut used_slugs: HashMap<String, usize> = HashMap::new();

    for entry in entries {
        let base_slug = stable_skill_slug(&entry.skill.name);
        let counter = used_slugs.entry(base_slug.clone()).or_insert(0);
        let slug = if *counter == 0 {
            base_slug
        } else {
            format!("{base_slug}_{counter}")
        };
        *counter += 1;

        let target_dir = projection_root.join(&slug);
        if target_dir.exists() {
            tokio::fs::remove_dir_all(&target_dir).await?;
        }
        copy_skill_dir_without_symlinks(&entry.skill.canonical_base_dir, &target_dir).await?;

        // Projected paths are the model-facing contract; canonical fields stay for audit.
        entry.skill.location = target_dir.join("SKILL.md");
        entry.skill.base_dir = target_dir;
    }

    Ok(())
}

/// Produce a filesystem-safe slug from a skill display name.
pub(crate) fn stable_skill_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            previous_was_separator = false;
        } else if !previous_was_separator && !slug.is_empty() {
            slug.push('_');
            previous_was_separator = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        "skill".into()
    } else {
        slug
    }
}

/// Deep-copy a skill directory tree, skipping symlinks for safety.
async fn copy_skill_dir_without_symlinks(source: &Path, target: &Path) -> MacacaResult<()> {
    tokio::fs::create_dir_all(target).await?;
    let mut stack = vec![(source.to_path_buf(), target.to_path_buf())];

    while let Some((from_dir, to_dir)) = stack.pop() {
        tokio::fs::create_dir_all(&to_dir).await?;
        let mut children = tokio::fs::read_dir(&from_dir).await?;
        while let Some(child) = children.next_entry().await? {
            let from_path = child.path();
            let file_type = tokio::fs::symlink_metadata(&from_path).await?.file_type();
            if file_type.is_symlink() {
                warn!(
                    path = %from_path.display(),
                    "skipping symlink while projecting skill directory"
                );
                continue;
            }

            let to_path = to_dir.join(child.file_name());
            if file_type.is_dir() {
                stack.push((from_path, to_path));
            } else if file_type.is_file() {
                tokio::fs::copy(&from_path, &to_path).await?;
            }
        }
    }

    Ok(())
}
