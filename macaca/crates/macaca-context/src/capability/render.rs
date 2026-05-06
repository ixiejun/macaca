//! Pure rendering + collision helpers for capability catalogs → [`ContextCandidate`] rows.

use crate::capability::model::{
    CapabilityCollisionRecord, McpCapabilityCatalog, McpServerCapabilitySummary,
    RuntimeToolCapabilityCatalog, SkillCapabilityCatalog,
};
use crate::{
    ContextCacheClass, ContextCandidate, ContextCandidateKind, ContextScope, ContextTarget,
};
use crate::estimate::estimate_text_tokens;
use crate::prompt::TrustLevel;

/// Computes duplicate local tool keys across MCP servers (same bare name advertised twice).
///
/// Modeling note: MCP clients often register tools prefixed by server; collisions here mean the
/// **declared exposed name** clashes before any client-side rewrite — still worth auditing.
pub fn mcp_tool_collisions(servers: &[McpServerCapabilitySummary]) -> Vec<CapabilityCollisionRecord> {
    let mut buckets: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for srv in servers {
        for tool in &srv.exposed_tools {
            buckets
                .entry(tool.clone())
                .or_default()
                .push(srv.server_id.clone());
        }
    }
    let mut out = Vec::new();
    for (local_key, mut claimants) in buckets {
        claimants.sort();
        claimants.dedup();
        if claimants.len() > 1 {
            out.push(CapabilityCollisionRecord {
                local_key,
                claimants,
            });
        }
    }
    out
}

/// Renders Tier-1 skill metadata as a fenced capability index chunk (explicitly **no SKILL.md bodies**).
fn render_skill_capability_text(cat: &SkillCapabilityCatalog) -> String {
    let mut w = String::from(
        "Skill capability index (compact; read SKILL.md on demand).\n\
         Usage: match task → open `location_ref` → follow progressive disclosure.\n\n",
    );
    w.push_str("<skill_capabilities>\n");
    for e in &cat.entries {
        w.push_str("  <entry");
        if !e.declared_dependencies.is_empty() {
            let deps: Vec<_> = e
                .declared_dependencies
                .iter()
                .map(|d| d.capability_id.as_str())
                .collect();
            w.push_str(&format!(
                r#" declared_capabilities="{}""#,
                deps.join(",")
            ));
        }
        w.push_str(">\n");
        w.push_str(&format!("    <id>{}</id>\n", xml_escape(&e.stable_id)));
        w.push_str(&format!("    <name>{}</name>\n", xml_escape(&e.name)));
        w.push_str(&format!(
            "    <description>{}</description>\n",
            xml_escape(&e.description)
        ));
        w.push_str(&format!(
            "    <location_ref>{}</location_ref>\n",
            xml_escape(&e.location_ref)
        ));
        w.push_str(&format!(
            "    <namespace>{}:{}</namespace>\n",
            xml_escape(&e.source_scope),
            xml_escape(&e.source_label)
        ));
        w.push_str("  </entry>\n");
    }
    w.push_str("</skill_capabilities>\n");
    if cat.truncated {
        w.push_str("\n(note: catalog was truncated by skill runtime limits.)\n");
    }
    if !cat.filtered.is_empty() {
        w.push_str("\n<filtered_skills>\n");
        for f in &cat.filtered {
            w.push_str(&format!(
                "  <skill name=\"{}\" reason=\"{}\"/>\n",
                xml_escape(&f.name),
                xml_escape(&f.reason)
            ));
        }
        w.push_str("</filtered_skills>\n");
    }
    w
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn skill_candidates(cat: &SkillCapabilityCatalog) -> Option<ContextCandidate> {
    // Avoid injecting noise when discovery found nothing actionable.
    if cat.entries.is_empty() && cat.filtered.is_empty() {
        return None;
    }
    let text = render_skill_capability_text(cat);
    if text.trim().is_empty() {
        return None;
    }
    let tokens = estimate_text_tokens(&text).max(1);
    Some(ContextCandidate {
        source_id: "capability.skill.index".into(),
        kind: ContextCandidateKind::CapabilityIndex,
        scope: ContextScope::Agent,
        priority: 820,
        trust: TrustLevel::Trusted,
        cache_class: ContextCacheClass::Dynamic,
        target: ContextTarget::SystemStable,
        content: text,
        token_estimate: tokens,
        diagnostics: Vec::new(),
    })
}

/// Builds the primary skill capability candidate plus optional per-entry diagnostics for missing MCP deps (policy hook).
pub fn skill_catalog_to_candidate(cat: &SkillCapabilityCatalog) -> Option<ContextCandidate> {
    skill_candidates(cat)
}

/// Returns lightweight dependency gap notes comparing declared `mcp_server:*` deps against ready server IDs.
pub fn skill_missing_dependency_notes(
    cat: &SkillCapabilityCatalog,
    ready_mcp_servers: &[String],
) -> Vec<String> {
    let ready: std::collections::BTreeSet<String> =
        ready_mcp_servers.iter().cloned().collect();
    let mut notes = Vec::new();
    for e in &cat.entries {
        for dep in &e.declared_dependencies {
            let cap = dep.capability_id.strip_prefix("mcp_server:");
            let Some(mid) = cap else {
                continue;
            };
            if mid.is_empty() {
                continue;
            }
            if !ready.contains(mid) {
                notes.push(format!(
                    "skill `{}` declares MCP capability `{}` but server `{}` not in ready probe set",
                    e.name,
                    dep.capability_id,
                    mid
                ));
            }
        }
    }
    notes
}

fn render_mcp_capability_text(cat: &McpCapabilityCatalog) -> String {
    let mut w = String::from(
        "MCP capability index (compact). Tools listed are registrations only — not executions.\n\
         Remote MCP **resources** and **prompt templates** are never auto-injected; load explicitly when policy allows.\n\n",
    );
    if !cat.collisions.is_empty() {
        w.push_str("<mcp_capability_collisions>\n");
        for c in &cat.collisions {
            w.push_str(&format!(
                "  <tool name=\"{}\" servers=\"{}\"/>\n",
                xml_escape(&c.local_key),
                xml_escape(&c.claimants.join(","))
            ));
        }
        w.push_str("</mcp_capability_collisions>\n\n");
    }
    w.push_str("<mcp_servers>\n");
    for srv in &cat.servers {
        w.push_str("  <server>\n");
        w.push_str(&format!(
            "    <id>{}</id>\n",
            xml_escape(&srv.server_id)
        ));
        w.push_str(&format!(
            "    <transport>{}</transport>\n",
            xml_escape(&srv.transport)
        ));
        w.push_str(&format!(
            "    <lifecycle>{}</lifecycle>\n",
            xml_escape(&srv.lifecycle)
        ));
        w.push_str(&format!("    <state>{}</state>\n", xml_escape(&srv.state)));
        let tools_qual: Vec<String> = srv
            .exposed_tools
            .iter()
            .map(|t| format!("{}::{}", srv.server_id, t))
            .collect();
        w.push_str(&format!(
            "    <tools>{}</tools>\n",
            xml_escape(&tools_qual.join(", "))
        ));
        w.push_str("  </server>\n");
    }
    w.push_str("</mcp_servers>\n");
    w
}

/// MCP surface defaults to **untrusted + dynamic** conservative policy per capability-context spec.
pub fn mcp_catalog_to_candidate(cat: &McpCapabilityCatalog) -> Option<ContextCandidate> {
    if cat.servers.is_empty() && cat.collisions.is_empty() {
        return None;
    }
    let text = render_mcp_capability_text(cat);
    if text.trim().is_empty() {
        return None;
    }
    let tokens = estimate_text_tokens(&text).max(1);
    Some(ContextCandidate {
        source_id: "capability.mcp.index".into(),
        kind: ContextCandidateKind::CapabilityIndex,
        scope: ContextScope::Application,
        priority: 780,
        trust: TrustLevel::Untrusted,
        cache_class: ContextCacheClass::Dynamic,
        target: ContextTarget::UserSide,
        content: format!("```mcp_capability_index\n{text}\n```"),
        token_estimate: tokens,
        diagnostics: Vec::new(),
    })
}

fn render_runtime_tools_text(cat: &RuntimeToolCapabilityCatalog) -> String {
    let mut names = cat.tool_names.clone();
    names.sort();
    names.dedup();
    let mut w = String::from("Runtime tool names available to this agent (compact index).\n");
    w.push_str("<runtime_tools>\n");
    for n in &names {
        w.push_str(&format!("  <tool name=\"{}\"/>\n", xml_escape(n)));
    }
    w.push_str("</runtime_tools>\n");
    w
}

pub fn runtime_tool_catalog_to_candidate(cat: &RuntimeToolCapabilityCatalog) -> Option<ContextCandidate> {
    if cat.tool_names.is_empty() {
        return None;
    }
    let text = render_runtime_tools_text(cat);
    if text.trim().is_empty() {
        return None;
    }
    let tokens = estimate_text_tokens(&text).max(1);
    Some(ContextCandidate {
        source_id: "capability.runtime_tools.index".into(),
        kind: ContextCandidateKind::CapabilityIndex,
        scope: ContextScope::Agent,
        priority: 760,
        trust: TrustLevel::Trusted,
        cache_class: ContextCacheClass::Dynamic,
        target: ContextTarget::SystemStable,
        content: text,
        token_estimate: tokens,
        diagnostics: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SkillCapabilityRecord;
    use crate::capability::model::DeclaredCapabilityDependency;

    #[test]
    fn collision_detects_duplicate_tool_across_servers() {
        let servers = vec![
            McpServerCapabilitySummary {
                server_id: "a".into(),
                transport: "stdio".into(),
                lifecycle: "session".into(),
                state: "ready".into(),
                exposed_tools: vec!["click".into()],
            },
            McpServerCapabilitySummary {
                server_id: "b".into(),
                transport: "stdio".into(),
                lifecycle: "session".into(),
                state: "ready".into(),
                exposed_tools: vec!["click".into()],
            },
        ];
        let c = mcp_tool_collisions(&servers);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].local_key, "click");
    }

    #[test]
    fn skill_render_excludes_activation_body_but_keeps_frontmatter_description() {
        let cat = SkillCapabilityCatalog {
            entries: vec![SkillCapabilityRecord {
                stable_id: "skill:workspace:demo".into(),
                name: "demo".into(),
                description: "short desc only".into(),
                location_ref: "/tmp/skills/demo/SKILL.md".into(),
                source_label: "app".into(),
                source_scope: "workspace".into(),
                declared_dependencies: Vec::new(),
            }],
            filtered: Vec::new(),
            truncated: false,
        };
        let c = skill_catalog_to_candidate(&cat).unwrap();
        assert!(c.content.contains("short desc only"));
        assert!(!c.content.contains("HUGE BODY"));
        assert!(c.content.contains("<skill_capabilities>"));
    }

    #[test]
    fn mcp_candidate_is_fenced_and_untrusted() {
        let cat = McpCapabilityCatalog {
            servers: vec![McpServerCapabilitySummary {
                server_id: "srv".into(),
                transport: "stdio".into(),
                lifecycle: "global".into(),
                state: "ready".into(),
                exposed_tools: vec!["t1".into()],
            }],
            collisions: Vec::new(),
        };
        let c = mcp_catalog_to_candidate(&cat).unwrap();
        assert!(c.content.starts_with("```mcp_capability_index"));
        assert!(matches!(c.trust, TrustLevel::Untrusted));
    }

    #[test]
    fn missing_dependency_emits_notes() {
        let cat = SkillCapabilityCatalog {
            entries: vec![SkillCapabilityRecord {
                stable_id: "x".into(),
                name: "s".into(),
                description: "d".into(),
                location_ref: "p".into(),
                source_label: "l".into(),
                source_scope: "workspace".into(),
                declared_dependencies: vec![DeclaredCapabilityDependency {
                    capability_id: "mcp_server:playwright".into(),
                    notes: None,
                }],
            }],
            filtered: Vec::new(),
            truncated: false,
        };
        let notes = skill_missing_dependency_notes(&cat, &["other".into()]);
        assert!(notes.iter().any(|n| n.contains("playwright")));
    }
}
