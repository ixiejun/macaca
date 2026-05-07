//! Package descriptor hooks for Agent Skills.
//!
//! This module is additive: it maps existing skill metadata into Route C
//! Package Manifest v0 without changing skill discovery, parsing, activation,
//! or provisioning behavior.  The hook lets future Store/package flows reason
//! about skills through the same descriptor contract used by applications.

use macaca_proto::{
    CapabilityId, DeveloperId, PackageCapability, PackageDescriptor, PackageId, PackageManifest,
    PackageRuntime, PackageRuntimeKind, PackageType,
};

use crate::agent_skill::{AgentSkill, SkillEntry};

/// Convert a parsed Agent Skill into a package descriptor.
pub fn agent_skill_package_descriptor(skill: &AgentSkill) -> PackageDescriptor {
    let mut manifest = PackageManifest::new(
        PackageId::new(format!("skill.{}", skill.name)),
        PackageType::Skill,
        "1.0.0",
        DeveloperId::new("local.skill"),
        PackageRuntime::new(
            PackageRuntimeKind::Custom("agent_skill_markdown".into()),
            "1",
        ),
    );
    manifest.provides.push(PackageCapability {
        id: CapabilityId::new(format!("skill.{}", skill.name)),
        description: skill.description.clone(),
    });
    manifest
        .metadata
        .insert("skill.source".into(), skill.source.clone());
    manifest
        .metadata
        .insert("skill.scope".into(), skill.source_scope.as_str().into());
    if let Some(homepage) = &skill.homepage {
        manifest
            .metadata
            .insert("skill.homepage".into(), homepage.clone());
    }
    PackageDescriptor::new(manifest)
}

/// Convert a full SkillEntry into a package descriptor with runtime metadata.
pub fn skill_entry_package_descriptor(entry: &SkillEntry) -> PackageDescriptor {
    let mut descriptor = agent_skill_package_descriptor(&entry.skill);
    if let Some(key) = &entry.metadata.skill_key {
        descriptor
            .manifest
            .metadata
            .insert("skill.key".into(), key.clone());
    }
    if !entry.metadata.mcp_servers.is_empty() {
        descriptor.manifest.metadata.insert(
            "skill.mcp_server_count".into(),
            entry.metadata.mcp_servers.len().to_string(),
        );
    }
    descriptor
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::agent_skill::SkillSourceScope;

    use super::*;

    #[test]
    fn agent_skill_maps_to_package_descriptor() {
        let skill = AgentSkill {
            name: "writer".into(),
            description: "Writing guidance".into(),
            location: PathBuf::from("/tmp/SKILL.md"),
            base_dir: PathBuf::from("/tmp"),
            canonical_location: PathBuf::from("/tmp/SKILL.md"),
            canonical_base_dir: PathBuf::from("/tmp"),
            source_scope: SkillSourceScope::Application,
            source: "application".into(),
            homepage: Some("https://example.invalid".into()),
            emoji: None,
        };

        let descriptor = agent_skill_package_descriptor(&skill);

        assert_eq!(descriptor.manifest.package_type, PackageType::Skill);
        assert_eq!(
            descriptor
                .manifest
                .metadata
                .get("skill.scope")
                .map(String::as_str),
            Some("application")
        );
    }
}
