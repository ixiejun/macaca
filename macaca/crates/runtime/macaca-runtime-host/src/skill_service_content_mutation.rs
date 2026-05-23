//! Built-in local Strategy for safe Skill package content mutation.
//!
//! The Skill service owns mutation semantics, while runtime-host owns concrete
//! provider wiring.  This module is the local filesystem Strategy used in
//! development and tests.  It performs deterministic validation, creates a
//! rollback memento before each side effect, and emits sanitized logs that
//! never include raw skill content or package bytes.

use std::path::{Component, Path, PathBuf};

use macaca_proto::{ServiceError, ServiceResult};
use macaca_skill::{
    SkillContentMutationCommand, SkillContentMutationDenied, SkillContentMutationKind,
    SkillContentMutationResult, SkillContentMutationStatus,
};
use serde::Serialize;
use tokio::fs;

/// Local filesystem applier for governed Skill content mutations.
#[derive(Debug, Default, Clone)]
pub(crate) struct LocalSkillContentMutationStrategy;

#[derive(Debug, Serialize)]
struct SkillMutationMemento {
    skill_id: String,
    relative_path: PathBuf,
    mutation_kind: SkillContentMutationKind,
    existed_before: bool,
    previous_bytes: Option<Vec<u8>>,
    captured_at: chrono::DateTime<chrono::Utc>,
}

impl LocalSkillContentMutationStrategy {
    /// Validate and apply one mutation command.
    ///
    /// Validation denials are returned as structured command results so shells
    /// can render policy state without parsing service errors.  Filesystem
    /// failures remain service errors because the provider could not complete
    /// the already-approved side-effect protocol.
    pub(crate) async fn apply(
        &self,
        command: SkillContentMutationCommand,
    ) -> ServiceResult<SkillContentMutationResult> {
        if let Err(denial) = command.validate() {
            tracing::warn!(
                trace_id = %command.trace.trace_id,
                skill_id = %command.skill_id,
                mutation_kind = ?command.kind,
                reason = %denial.reason,
                "skill content mutation denied by contract validation"
            );
            return Ok(SkillContentMutationResult::denied(&command, denial));
        }

        let package_root = normalized_existing_root(&command.package_root).await?;
        let target = checked_target_path(&package_root, &command.relative_path).await?;
        reject_symlink_path(&package_root, &target).await?;
        let rollback_memento_ref = write_memento(&package_root, &target, &command).await?;
        let bytes_written = apply_file_side_effect(&target, &command).await?;
        let result = SkillContentMutationResult {
            status: SkillContentMutationStatus::Applied,
            skill_id: command.skill_id.clone(),
            relative_path: command.relative_path.clone(),
            mutation_kind: command.kind.clone(),
            mutated: true,
            rollback_memento_ref: Some(rollback_memento_ref),
            bytes_written,
            denied_reason: None,
            evidence_ids: command.sanitized_evidence_ids(),
            policy_decision_refs: command.sanitized_policy_decision_refs(),
            trace_id: command.trace.trace_id.clone(),
            captured_at: chrono::Utc::now(),
        };
        tracing::info!(
            trace_id = %command.trace.trace_id,
            skill_id = %command.skill_id,
            relative_path = %command.relative_path.display(),
            mutation_kind = ?command.kind,
            bytes_written,
            rollback_memento = result.rollback_memento_ref.as_deref().unwrap_or(""),
            "skill content mutation applied through local strategy"
        );
        Ok(result)
    }
}

async fn normalized_existing_root(root: &Path) -> ServiceResult<PathBuf> {
    let canonical = fs::canonicalize(root).await.map_err(|err| {
        ServiceError::InvalidArgument(format!("skill package root is unavailable: {err}"))
    })?;
    let metadata = fs::metadata(&canonical).await.map_err(|err| {
        ServiceError::InvalidArgument(format!("skill package root metadata is unavailable: {err}"))
    })?;
    if !metadata.is_dir() {
        return Err(ServiceError::InvalidArgument(
            "skill package root must be a directory".into(),
        ));
    }
    Ok(canonical)
}

async fn checked_target_path(root: &Path, relative: &Path) -> ServiceResult<PathBuf> {
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ServiceError::InvalidArgument(
            "skill mutation target path escaped package root".into(),
        ));
    }
    Ok(root.join(relative))
}

async fn reject_symlink_path(root: &Path, target: &Path) -> ServiceResult<()> {
    let mut current = root.to_path_buf();
    let relative = target.strip_prefix(root).map_err(|_| {
        ServiceError::InvalidArgument("skill mutation target path escaped package root".into())
    })?;
    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current).await {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(ServiceError::InvalidArgument(
                    "skill mutation target must not traverse symlinks".into(),
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => break,
            Err(err) => {
                return Err(ServiceError::InvalidArgument(format!(
                    "skill mutation target metadata is unavailable: {err}"
                )));
            }
        }
    }
    Ok(())
}

async fn write_memento(
    package_root: &Path,
    target: &Path,
    command: &SkillContentMutationCommand,
) -> ServiceResult<String> {
    let previous_bytes = match fs::read(target).await {
        Ok(bytes) => Some(bytes),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(ServiceError::InvalidArgument(format!(
                "skill mutation could not read rollback source: {err}"
            )));
        }
    };
    let memento_id = format!("skill-mutation-{}", uuid::Uuid::new_v4());
    let memento_dir = package_root.join(".macaca").join("skill-mutation-mementos");
    fs::create_dir_all(&memento_dir).await.map_err(|err| {
        ServiceError::InvalidArgument(format!("skill mutation memento directory failed: {err}"))
    })?;
    let memento_path = memento_dir.join(format!("{memento_id}.json"));
    let memento = SkillMutationMemento {
        skill_id: command.skill_id.clone(),
        relative_path: command.relative_path.clone(),
        mutation_kind: command.kind.clone(),
        existed_before: previous_bytes.is_some(),
        previous_bytes,
        captured_at: chrono::Utc::now(),
    };
    let bytes = serde_json::to_vec(&memento).map_err(|err| {
        ServiceError::InvalidArgument(format!(
            "skill mutation memento serialization failed: {err}"
        ))
    })?;
    fs::write(&memento_path, bytes).await.map_err(|err| {
        ServiceError::InvalidArgument(format!("skill mutation memento write failed: {err}"))
    })?;
    Ok(format!(
        "skill-mutation-memento://{}",
        memento_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(&memento_id)
    ))
}

async fn apply_file_side_effect(
    target: &Path,
    command: &SkillContentMutationCommand,
) -> ServiceResult<u64> {
    match command.kind {
        SkillContentMutationKind::RemoveSupportFile
        | SkillContentMutationKind::ArchiveMaterialization => match fs::remove_file(target).await {
            Ok(()) => Ok(0),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(err) => Err(ServiceError::InvalidArgument(format!(
                "skill mutation remove failed: {err}"
            ))),
        },
        SkillContentMutationKind::CreateSkill
        | SkillContentMutationKind::PatchSkill
        | SkillContentMutationKind::WriteSupportFile
        | SkillContentMutationKind::RestoreMaterialization => {
            let content = command.content.as_deref().ok_or_else(|| {
                ServiceError::InvalidArgument(
                    SkillContentMutationDenied::new(
                        "skill content mutation requires bounded content",
                        false,
                    )
                    .reason,
                )
            })?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).await.map_err(|err| {
                    ServiceError::InvalidArgument(format!(
                        "skill mutation parent directory creation failed: {err}"
                    ))
                })?;
            }
            let tmp_path =
                target.with_extension(format!("macaca-tmp-{}", uuid::Uuid::new_v4().simple()));
            fs::write(&tmp_path, content).await.map_err(|err| {
                ServiceError::InvalidArgument(format!("skill mutation temp write failed: {err}"))
            })?;
            fs::rename(&tmp_path, target).await.map_err(|err| {
                ServiceError::InvalidArgument(format!("skill mutation atomic rename failed: {err}"))
            })?;
            Ok(content.len() as u64)
        }
    }
}
