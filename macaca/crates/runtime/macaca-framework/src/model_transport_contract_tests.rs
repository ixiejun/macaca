// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use super::*;

#[test]
fn formatter_family_ref_is_provider_neutral() {
    let family = FormatterFamilyRef::new("formatter.family.generic", "parser.generic");

    assert_eq!(family.family_id, "formatter.family.generic");
    assert_eq!(family.parser_id, "parser.generic");
}

#[test]
fn model_exception_taxonomy_has_sanitized_reason() {
    let error = ModelException::RateLimit {
        reason: "quota exhausted".to_string(),
    };

    assert_eq!(error.sanitized_reason(), "quota exhausted");
}

#[test]
fn transport_result_can_report_unavailable_without_client() {
    let result = ModelTransportResult::Failed {
        error: ModelException::Unavailable {
            reason: "model service absent".to_string(),
        },
    };

    assert!(matches!(
        result,
        ModelTransportResult::Failed {
            error: ModelException::Unavailable { .. }
        }
    ));
}
