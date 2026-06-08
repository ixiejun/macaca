//! Contract tests for `SKILL.md` parsing and tier-1/tier-2 skill loading.

use super::parser::parse_frontmatter;
use super::{parse_skill_md, parse_skill_md_full, AgentSkill, extract_body};

#[test]
fn parse_frontmatter_basic() {
    let md = "---\nname: golang\ndescription: Go patterns\n---\n# Go\nContent here.";
    let fm = parse_frontmatter(md).unwrap();
    assert_eq!(fm.name, "golang");
    assert_eq!(fm.description, "Go patterns");
}

#[test]
fn extract_body_basic() {
    let md = "---\nname: golang\ndescription: Go patterns\n---\n# Go\n\nUse chi router.";
    let body = extract_body(md).unwrap();
    assert!(body.contains("chi router"));
    assert!(body.starts_with("# Go"));
}

#[test]
fn extract_body_empty() {
    let md = "---\nname: empty\n---\n";
    let body = extract_body(md).unwrap();
    assert!(body.is_empty());
}

#[test]
fn parse_skill_md_tuple() {
    let md = "---\nname: test\ndescription: Test skill\n---\nDo the thing.";
    let (name, desc, body) = parse_skill_md(md).unwrap();
    assert_eq!(name, "test");
    assert_eq!(desc, "Test skill");
    assert_eq!(body, "Do the thing.");
}

#[test]
fn parse_frontmatter_no_delimiter() {
    let md = "# Just markdown\nNo frontmatter.";
    let err = parse_frontmatter(md).unwrap_err();
    assert!(err.to_string().contains("frontmatter"));
}

#[test]
fn parse_frontmatter_missing_closing() {
    let md = "---\nname: broken\n# No closing";
    let err = parse_frontmatter(md).unwrap_err();
    assert!(err.to_string().contains("closing"));
}

#[test]
fn parse_frontmatter_empty_name() {
    let md = "---\nname: \"\"\ndescription: no name\n---\nbody";
    let err = parse_frontmatter(md).unwrap_err();
    assert!(err.to_string().contains("name"));
}

#[tokio::test]
async fn agent_skill_from_path() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("test-skill");
    tokio::fs::create_dir(&skill_dir).await.unwrap();
    tokio::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: A test\n---\n# Test\nDo stuff.",
    )
    .await
    .unwrap();

    let skill = AgentSkill::from_path(skill_dir.join("SKILL.md"))
        .await
        .unwrap();
    assert_eq!(skill.name, "test-skill");
    assert_eq!(skill.description, "A test");
    assert_eq!(skill.base_dir, skill_dir);
}

#[tokio::test]
async fn agent_skill_load_content() {
    let dir = tempfile::tempdir().unwrap();
    let skill_dir = dir.path().join("golang");
    tokio::fs::create_dir(&skill_dir).await.unwrap();
    tokio::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: golang\ndescription: Go patterns\n---\n# Go\nUse chi router.",
    )
    .await
    .unwrap();
    tokio::fs::write(skill_dir.join("helpers.sh"), "#!/bin/bash\necho hi")
        .await
        .unwrap();

    let skill = AgentSkill::from_path(skill_dir.join("SKILL.md"))
        .await
        .unwrap();
    let activated = skill.load_content().await.unwrap();
    assert_eq!(activated.name, "golang");
    assert!(activated.content.contains("chi router"));
    assert_eq!(activated.resources.len(), 1);
}

#[test]
fn parse_skill_md_full_macaca_mcp_servers() {
    let md = r#"---
name: browser
description: Browser MCP
metadata:
  macaca:
    mcpServers:
      sample-mcp:
        command: sample-mcp-bin
        args: [--headless]
        transport: stdio
        toolPrefix: browser_
---
body"#;
    let parsed = parse_skill_md_full(md).unwrap();
    assert_eq!(parsed.metadata.mcp_servers.len(), 1);
    let server = &parsed.metadata.mcp_servers[0];
    assert_eq!(server.id, "sample-mcp");
    assert_eq!(server.command, "sample-mcp-bin");
    assert_eq!(server.args, vec!["--headless"]);
    assert_eq!(server.transport, "stdio");
    assert_eq!(server.tool_prefix.as_deref(), Some("browser_"));
}

#[test]
fn parse_skill_md_full_openclaw_install_metadata() {
    let md = r#"---
name: sample-mcp-skill
description: Sample automation
metadata:
  openclaw:
    install:
      - id: npm-sample-mcp
        kind: npm
        package: "@sample/mcp"
        bins: [sample-mcp-bin]
        label: Install sample MCP
---
body"#;
    let parsed = parse_skill_md_full(md).unwrap();
    assert_eq!(parsed.metadata.install.len(), 1);
    let install = &parsed.metadata.install[0];
    assert_eq!(install.kind, "npm");
    assert_eq!(install.package.as_deref(), Some("@sample/mcp"));
    assert_eq!(install.bins, vec!["sample-mcp-bin"]);
}
