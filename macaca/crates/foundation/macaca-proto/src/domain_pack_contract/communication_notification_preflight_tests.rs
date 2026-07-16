use std::collections::BTreeMap;

use super::super::communication_notification::{
    NotificationDeliveryChannel, NotificationMessage, NotificationPublishCommand,
    NotificationSchedule, NotificationScheduleCommand, NotificationTarget,
};
use super::super::communication_notification_preflight::{
    NotificationAdmissionEvidence, NotificationDispatchPreflight,
};
use super::super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackPreflightStatus, DomainPackResourceReservation,
};

#[test]
fn notification_admission_rejects_every_common_side_effect_gate_before_dispatch() {
    let gate = NotificationDispatchPreflight::new(["notification.publish"]);
    let command = valid_publish();
    let preflight = valid_preflight("notification.publish", "notification.publish");
    let evidence = valid_evidence();

    let mut dispatched = false;
    assert_eq!(
        gate.dispatch_publish(&command, &preflight, &evidence, || {
            dispatched = true;
            "delivered"
        }),
        Ok("delivered")
    );
    assert!(dispatched);

    for rejected in [
        with_consent_denied(&evidence),
        with_host_unsupported(&evidence),
        with_provider_unhealthy(&evidence),
        with_target_denied(&evidence),
        with_content_sensitivity_denied(&evidence),
        with_payload_limit_exceeded(&evidence),
        with_channel_denied(&evidence),
        with_rate_limited(&evidence),
        with_resource_unreserved(&evidence),
        with_entitlement_denied(&evidence),
        with_action_limit_exceeded(&evidence),
    ] {
        let mut rejected_dispatched = false;
        assert!(gate
            .dispatch_publish(&command, &preflight, &rejected, || rejected_dispatched =
                true)
            .is_err());
        assert!(!rejected_dispatched);
    }
}

#[test]
fn notification_preflight_reports_bounded_diagnostics_for_invalid_and_unavailable_paths() {
    let gate = NotificationDispatchPreflight::new(["notification.publish"]);
    let command = valid_publish();
    let evidence = valid_evidence();
    let invalid_scope = valid_preflight("notification.publish", "notification.unknown");
    let missing_provider = DomainPackCommandPreflight {
        entitlement: DomainPackEntitlementEvidence {
            provider_available: false,
            ..valid_preflight("notification.publish", "notification.publish").entitlement
        },
        ..valid_preflight("notification.publish", "notification.publish")
    };
    let unsupported = DomainPackCommandPreflight {
        entitlement: DomainPackEntitlementEvidence {
            command_supported: false,
            ..valid_preflight("notification.publish", "notification.publish").entitlement
        },
        ..valid_preflight("notification.publish", "notification.publish")
    };
    let invalid_command = NotificationPublishCommand {
        client_request_id: String::new(),
        ..valid_publish()
    };

    let denied = gate
        .evaluate_publish(&command, &invalid_scope, &evidence)
        .expect_err("undeclared scope must be denied");
    assert_eq!(denied.status, DomainPackPreflightStatus::Denied);
    assert_eq!(denied.reason_code, "permission_not_declared");
    let unavailable = gate
        .evaluate_publish(&command, &missing_provider, &evidence)
        .expect_err("unhealthy provider must not dispatch");
    assert_eq!(unavailable.status, DomainPackPreflightStatus::Unavailable);
    assert_eq!(unavailable.reason_code, "provider_unavailable");
    let mut unsupported_dispatched = false;
    let unsupported = gate
        .dispatch_publish(&command, &unsupported, &evidence, || {
            unsupported_dispatched = true
        })
        .expect_err("unsupported command must not dispatch");
    assert_eq!(unsupported.status, DomainPackPreflightStatus::Unsupported);
    assert_eq!(unsupported.reason_code, "command_not_supported");
    assert!(!unsupported_dispatched);
    let invalid = gate
        .evaluate_publish(
            &invalid_command,
            &valid_preflight("notification.publish", "notification.publish"),
            &evidence,
        )
        .expect_err("invalid command must not dispatch");
    assert_eq!(invalid.reason_code, "notification_validation_failed");
}

#[test]
fn notification_schedule_validates_horizon_without_wall_clock_dependence() {
    let gate = NotificationDispatchPreflight::new(["notification.schedule"]);
    let command = NotificationScheduleCommand {
        publish: valid_publish(),
        schedule: NotificationSchedule {
            deliver_at_epoch_ms: None,
            relative_delay_ms: Some(1_000),
            timezone_id: Some("UTC".into()),
            expiry_epoch_ms: Some(2_000),
        },
    };
    let rejected = NotificationAdmissionEvidence {
        schedule_horizon_within_limit: false,
        ..valid_evidence()
    };
    let result = gate.evaluate_schedule(
        &command,
        &valid_preflight("notification.schedule", "notification.schedule"),
        &rejected,
    );
    assert_eq!(
        result
            .expect_err("overlong schedule must be denied")
            .reason_code,
        "notification_schedule_invalid"
    );
}

fn valid_publish() -> NotificationPublishCommand {
    NotificationPublishCommand {
        message: NotificationMessage {
            title_ref: "artifact:title".into(),
            body_ref: "artifact:body".into(),
            locale: Some("en-US".into()),
            sensitivity: "private".into(),
            category_id: None,
            collapse_key: None,
        },
        target: NotificationTarget {
            target_id: "target:recipient".into(),
            target_kind: "user".into(),
            subscription: None,
            redaction_label: "recipient".into(),
        },
        channel: NotificationDeliveryChannel::InApp,
        client_request_id: "request:notification".into(),
    }
}

fn valid_preflight(command_name: &str, scope: &str) -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: command_name.into(),
        requested_scope: scope.into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy:allowed".into(),
            allowed: true,
            reason_code: "allowed".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval:granted".into(),
            approved: true,
            reason_code: "approved".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement:granted".into(),
            provider_available: true,
            scope_granted: true,
            command_supported: true,
            host_capability_enabled: true,
            reason_code: "granted".into(),
        },
        required_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
        reserved_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
    }
}

fn valid_evidence() -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        consent_granted: true,
        host_supported: true,
        provider_healthy: true,
        target_scope_granted: true,
        content_sensitivity_allowed: true,
        payload_within_limit: true,
        channel_allowed: true,
        rate_limit_available: true,
        resource_budget_reserved: true,
        entitlement_granted: true,
        action_count: 1,
        max_action_count: 3,
        schedule_horizon_within_limit: true,
    }
}

fn with_consent_denied(value: &NotificationAdmissionEvidence) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        consent_granted: false,
        ..value.clone()
    }
}

fn with_host_unsupported(value: &NotificationAdmissionEvidence) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        host_supported: false,
        ..value.clone()
    }
}

fn with_provider_unhealthy(value: &NotificationAdmissionEvidence) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        provider_healthy: false,
        ..value.clone()
    }
}

fn with_target_denied(value: &NotificationAdmissionEvidence) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        target_scope_granted: false,
        ..value.clone()
    }
}

fn with_content_sensitivity_denied(
    value: &NotificationAdmissionEvidence,
) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        content_sensitivity_allowed: false,
        ..value.clone()
    }
}

fn with_payload_limit_exceeded(
    value: &NotificationAdmissionEvidence,
) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        payload_within_limit: false,
        ..value.clone()
    }
}

fn with_channel_denied(value: &NotificationAdmissionEvidence) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        channel_allowed: false,
        ..value.clone()
    }
}

fn with_rate_limited(value: &NotificationAdmissionEvidence) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        rate_limit_available: false,
        ..value.clone()
    }
}

fn with_resource_unreserved(
    value: &NotificationAdmissionEvidence,
) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        resource_budget_reserved: false,
        ..value.clone()
    }
}

fn with_entitlement_denied(value: &NotificationAdmissionEvidence) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        entitlement_granted: false,
        ..value.clone()
    }
}

fn with_action_limit_exceeded(
    value: &NotificationAdmissionEvidence,
) -> NotificationAdmissionEvidence {
    NotificationAdmissionEvidence {
        action_count: value.max_action_count + 1,
        ..value.clone()
    }
}
