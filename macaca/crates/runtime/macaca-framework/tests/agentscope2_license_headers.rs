// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use std::fs;
use std::path::PathBuf;

const REQUIRED_HEADER_LINES: [&str; 4] = [
    "// SPDX-License-Identifier: Apache-2.0",
    "// Derived from AgentScope Java 2.0 concepts and APIs.",
    "// Copyright 2024-2026 the original AgentScope author or authors.",
    "// Licensed under the Apache License, Version 2.0.",
];

const ADAPTED_SOURCE_FILES: &[&str] = &[
    "src/message.rs",
    "src/message_metadata.rs",
    "src/message_tests.rs",
    "src/provider_contract.rs",
    "src/provider_capability_matrix.rs",
    "src/provider_contract_tests.rs",
    "src/agent_contracts.rs",
    "src/agent_contracts_tests.rs",
    "src/event_contract.rs",
    "src/event_contract_projection.rs",
    "src/event_contract_tests.rs",
    "src/runtime_context.rs",
    "src/runtime_context_tests.rs",
    "src/middleware.rs",
    "src/permission_contract.rs",
    "src/model_transport_contract.rs",
    "src/model_transport_contract_tests.rs",
    "src/mcp_content_contract.rs",
    "src/mcp_content_contract_tests.rs",
    "src/harness_contracts.rs",
    "src/harness_contracts_tests.rs",
    "src/formatter_anthropic.rs",
    "src/mcp_http.rs",
    "src/mcp_stdio.rs",
    "src/mcp_toolkit.rs",
    "src/tool_schema.rs",
    "src/react_agent.rs",
    "src/agent_execution_config.rs",
    "src/react_agent_steps.rs",
    "src/tool_bridge.rs",
    "src/react_agent_stream_tests.rs",
    "src/react_agent_test_support.rs",
    "tests/agentscope2_license_headers.rs",
    "tests/agentscope2_framework_boundaries.rs",
];

#[test]
fn agentscope2_adapted_sources_have_apache_2_source_notice() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for &relative_path in ADAPTED_SOURCE_FILES {
        let path = manifest_dir.join(relative_path);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for required_line in REQUIRED_HEADER_LINES {
            assert!(
                source.contains(required_line),
                "{} is missing required AgentScope 2.0 license header line: {}",
                path.display(),
                required_line
            );
        }
    }
}

#[test]
fn agentscope2_third_party_notice_is_inspectable() {
    let notice_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/third-party-notices-agentscope-java.md");
    let notice = fs::read_to_string(&notice_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", notice_path.display()));

    assert!(notice.contains("AgentScope Java"));
    assert!(notice.contains("Apache License, Version 2.0"));
    assert!(notice.contains("https://java.agentscope.io/v2/zh/docs/index.html"));
}
