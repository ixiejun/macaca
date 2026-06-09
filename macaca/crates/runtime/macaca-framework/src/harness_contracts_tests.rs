// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;

#[test]
fn filesystem_specs_cover_required_shapes() {
    let specs = vec![
        HarnessFilesystemSpec::Local {
            root_ref: "workspace-root".to_string(),
        },
        HarnessFilesystemSpec::Remote {
            endpoint_ref: "remote-fs".to_string(),
        },
        HarnessFilesystemSpec::Sandbox {
            sandbox_ref: "sandbox-ref".to_string(),
        },
        HarnessFilesystemSpec::Baked {
            image_ref: "image-ref".to_string(),
        },
    ];

    assert_eq!(specs.len(), 4);
}

#[test]
fn sandbox_backend_can_be_unavailable_without_fallback() {
    let backend = HarnessSandboxBackendSpec::Unavailable {
        reason: "sandbox service absent".to_string(),
    };

    assert!(matches!(
        backend,
        HarnessSandboxBackendSpec::Unavailable { .. }
    ));
}

#[test]
fn plan_mode_transition_is_agent_local_contract() {
    let transition = HarnessPlanModeTransition::UseTool {
        kind: HarnessPlanToolKind::UpdatePlan,
    };

    assert!(matches!(
        transition,
        HarnessPlanModeTransition::UseTool {
            kind: HarnessPlanToolKind::UpdatePlan
        }
    ));
}
