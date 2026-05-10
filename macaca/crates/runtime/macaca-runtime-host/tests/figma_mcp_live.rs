//! Live Figma MCP (`figma-developer-mcp` over stdio) checks against a real design file.
//!
//! These tests are **`#[ignored]` by default** so CI and offline runs stay green. Enable locally:
//!
//! ```text
//! # Prefer env so secrets stay out of shell history when possible:
//! export FIGMA_API_KEY='...'
//! cargo test -p macaca-runtime-host figma_ -- --ignored --nocapture
//! ```
//!
//! If `FIGMA_API_KEY` is unset, tests try `macaca/config/default.toml` → `[mcp.env].FIGMA_API_KEY`
//! (resolved from `CARGO_MANIFEST_DIR`). Placeholder tokens (`YOUR_*`, etc.) count as absent.

use std::collections::BTreeMap;
use std::path::PathBuf;

use macaca_framework::mcp::{
    client_from_transport, McpSessionMode, McpTimeouts, McpTransportConfig,
};
use macaca_framework::message::ContentBlock;
use macaca_runtime_host::mcp_runtime::{
    McpDefinitionSource, McpLifecycleScope, McpRuntimeFacade, McpRuntimeStatusState,
    McpServerDefinition, McpToolPolicy,
};

/// Public sample referenced by docs; validates read access for the supplied token only.
/// Design URL used to derive IDs (node-id URL style uses hyphen, API uses colon).
const FIGMA_SAMPLE_DESIGN_URL: &str =
    "https://www.figma.com/design/RbmcUzFtiVceQOziRvYoJU/Untitled?node-id=52-61152&t=nt8sSdwRoqemXnst-4";
const SAMPLE_FILE_KEY: &str = "RbmcUzFtiVceQOziRvYoJU";
const SAMPLE_NODE_ID: &str = "52:61152";

fn macaca_default_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/default.toml")
}

fn resolve_figma_api_key() -> Option<String> {
    if let Ok(v) = std::env::var("FIGMA_API_KEY") {
        let t = v.trim().to_owned();
        if !t.is_empty() {
            return Some(t);
        }
    }

    let path = macaca_default_config_path();
    let raw = std::fs::read_to_string(path).ok()?;
    let value: toml::Value = raw.parse().ok()?;
    value
        .get("mcp")?
        .get("env")?
        .get("FIGMA_API_KEY")
        .and_then(|x| x.as_str())
        .map(std::string::ToString::to_string)
}

fn actionable_figma_token(key: &str) -> bool {
    let key = key.trim();
    if key.is_empty() {
        return false;
    }
    if key.starts_with("YOUR_") || key.starts_with("REPLACE_") {
        return false;
    }
    if key.starts_with('<') && key.ends_with('>') {
        return false;
    }
    true
}

fn figma_mcp_stdio_env(api_key: &str) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("FIGMA_API_KEY".into(), api_key.trim().to_string());
    env
}

fn figma_stdio_config(api_key: &str) -> McpTransportConfig {
    // Matches bundled `compat_mappings.toml` for `figma-developer-mcp`.
    McpTransportConfig::Stdio {
        command: "npx".into(),
        args: vec!["-y".into(), "figma-developer-mcp".into(), "--stdio".into()],
        env: figma_mcp_stdio_env(api_key),
        cwd: None,
    }
}

fn fig_os_server_definition(api_key: &str) -> McpServerDefinition {
    McpServerDefinition {
        id: "figma".into(),
        transport: figma_stdio_config(api_key),
        lifecycle: McpLifecycleScope::AgentSession,
        session_mode: McpSessionMode::Stateful,
        tool_prefix: None,
        required_bins: vec!["npx".into()],
        enabled: true,
        source: McpDefinitionSource::Compatibility,
        concurrency_isolation: None,
    }
}

fn live_mcp_timeouts() -> McpTimeouts {
    McpTimeouts {
        connect: std::time::Duration::from_secs(90),
        list_tools: std::time::Duration::from_secs(90),
        call_tool: std::time::Duration::from_secs(120),
    }
}

fn require_figma_api_key() -> String {
    let key = resolve_figma_api_key().unwrap_or_default();
    if actionable_figma_token(&key) {
        return key;
    }
    panic!(
        "FIGMA_API_KEY missing or placeholder. Set env FIGMA_API_KEY or a real token under [mcp.env] in {} (see FIGMA_SAMPLE_DESIGN_URL in this file).",
        macaca_default_config_path().display()
    );
}

fn json_text_from_mcp_result(result: &macaca_framework::mcp::McpCallResult) -> String {
    result
        .content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text(tb) => Some(tb.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
#[ignore = "live Figma MCP: needs npx + network + valid FIGMA_API_KEY; cargo test -p macaca-runtime-host figma_stdio -- --ignored --nocapture"]
async fn figma_stdio_client_fetch_node_tree() {
    let api_key = require_figma_api_key();

    let config = figma_stdio_config(&api_key);
    let mut client =
        client_from_transport(config, live_mcp_timeouts()).expect("valid stdio MCP config");

    client
        .connect()
        .await
        .expect("initialize figma-developer-mcp via npx/stderr check if spawn fails");

    let tools = client.list_tools().await.expect("tools/list");

    let tool_name = tools
        .iter()
        .find(|t| t.name == "get_figma_data")
        .map(|t| t.name.as_str())
        .unwrap_or_else(|| {
            panic!(
                "missing get_figma_data; advertised tools: {:?}",
                tools.iter().map(|t| &t.name).collect::<Vec<_>>()
            )
        });

    // Schema follows project SKILL examples; tweak if upstream renames inputs.
    let args = serde_json::json!({
        "fileKey": SAMPLE_FILE_KEY,
        "nodeId": SAMPLE_NODE_ID,
        "depth": 3
    });

    let result = client
        .call_tool(tool_name, args)
        .await
        .expect("tools/call get_figma_data");

    let _ = client.close().await;

    assert!(
        !result.is_error,
        "tool error bit set; MCP metadata {:?}",
        result.metadata
    );

    let text = json_text_from_mcp_result(&result).trim().to_owned();
    assert!(
        text.len() > 64,
        "expected substantive design JSON payload for node {}",
        SAMPLE_NODE_ID
    );

    eprintln!(
        "figma_stdio_client_fetch_node_tree OK — {} chars (url={})",
        text.len(),
        FIGMA_SAMPLE_DESIGN_URL
    );
}

#[tokio::test]
#[ignore = "live Figma MCP via McpRuntimeFacade::probe; cargo test -p macaca-runtime-host figma_facade_probe -- --ignored --nocapture"]
async fn figma_facade_probe_lists_get_figma_data() {
    let api_key = require_figma_api_key();

    let def = fig_os_server_definition(&api_key);
    let facade = McpRuntimeFacade::new();
    facade.upsert_definition(def.clone()).await;

    let statuses = facade.probe(&McpToolPolicy::default()).await;
    let status = statuses
        .into_iter()
        .find(|s| s.server_id == "figma")
        .expect("probe should include merged figma definition");

    assert_eq!(
        status.state,
        McpRuntimeStatusState::Ready,
        "unexpected probe state failure_reason={:?} tools={:?}",
        status.failure_reason,
        status.exposed_tools,
    );

    assert!(
        status.exposed_tools.iter().any(|n| n == "get_figma_data"),
        "probe should surface get_figma_data; saw {:?}",
        status.exposed_tools
    );

    eprintln!(
        "figma_facade_probe_lists_get_figma_data OK — {}",
        FIGMA_SAMPLE_DESIGN_URL
    );
}
