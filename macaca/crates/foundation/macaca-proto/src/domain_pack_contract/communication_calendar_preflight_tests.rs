use std::collections::BTreeMap;

use super::super::communication_calendar::{
    CalendarAttendee, CalendarCreateEventCommand, CalendarEvent, CalendarRecurrence,
};
use super::super::communication_calendar_preflight::{
    CalendarAdmissionEvidence, CalendarDispatchPreflight,
};
use super::super::pack_preflight::{
    DomainPackApprovalEvidence, DomainPackCommandPreflight, DomainPackEntitlementEvidence,
    DomainPackPolicyEvidence, DomainPackResourceReservation,
};

#[test]
fn calendar_admission_rejects_every_host_policy_gate_before_provider_dispatch() {
    let gate = CalendarDispatchPreflight::new(8, 32, ["calendar.create_event"]);
    let command = valid_command();
    let preflight = valid_preflight();
    let evidence = valid_evidence();

    let mut dispatched = false;
    assert_eq!(
        gate.dispatch_create(&command, &preflight, &evidence, || {
            dispatched = true;
            "event-handle"
        }),
        Ok("event-handle")
    );
    assert!(dispatched);

    for rejected in rejection_cases(&evidence) {
        let mut rejected_dispatched = false;
        assert!(gate
            .dispatch_create(&command, &preflight, &rejected, || rejected_dispatched =
                true)
            .is_err());
        assert!(!rejected_dispatched);
    }
}

#[test]
fn calendar_validation_failure_never_reaches_dispatch() {
    let gate = CalendarDispatchPreflight::new(8, 32, ["calendar.create_event"]);
    let invalid = CalendarCreateEventCommand {
        idempotency_key: String::new(),
        ..valid_command()
    };
    let mut dispatched = false;
    let result = gate.dispatch_create(&invalid, &valid_preflight(), &valid_evidence(), || {
        dispatched = true
    });
    assert_eq!(
        result
            .expect_err("invalid event must be denied")
            .reason_code,
        "calendar_validation_failed"
    );
    assert!(!dispatched);
}

fn valid_command() -> CalendarCreateEventCommand {
    CalendarCreateEventCommand {
        event: CalendarEvent {
            event_id: "event:one".into(),
            source_id: "source:one".into(),
            title_ref: "artifact:title".into(),
            description_ref: Some("artifact:description".into()),
            start_epoch_ms: 1_000,
            end_epoch_ms: 2_000,
            timezone_id: "UTC".into(),
            recurrence: Some(CalendarRecurrence {
                frequency: "daily".into(),
                interval: 1,
                count: Some(2),
                until_epoch_ms: None,
                timezone_id: "UTC".into(),
                expansion_limit: 2,
            }),
            attendees: vec![CalendarAttendee {
                attendee_id: "principal:one".into(),
                role: "required".into(),
                response_state: "needs_action".into(),
                identity_scope: "tenant:one".into(),
            }],
        },
        idempotency_key: "idempotency:event-one".into(),
    }
}

fn valid_preflight() -> DomainPackCommandPreflight {
    DomainPackCommandPreflight {
        command_name: "calendar.create_event".into(),
        requested_scope: "calendar.write".into(),
        policy: DomainPackPolicyEvidence {
            decision_ref: "policy:allowed".into(),
            allowed: true,
            reason_code: "allowed".into(),
        },
        approval: Some(DomainPackApprovalEvidence {
            approval_ref: "approval:allowed".into(),
            approved: true,
            reason_code: "approved".into(),
        }),
        entitlement: DomainPackEntitlementEvidence {
            entitlement_ref: "entitlement:allowed".into(),
            provider_available: true,
            scope_granted: true,
            command_supported: true,
            host_capability_enabled: true,
            reason_code: "allowed".into(),
        },
        required_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
        reserved_resources: DomainPackResourceReservation {
            units: BTreeMap::from([("provider_calls".into(), 1)]),
        },
    }
}

fn valid_evidence() -> CalendarAdmissionEvidence {
    CalendarAdmissionEvidence {
        source_owned_by_caller: true,
        credential_secret_reference_valid: true,
        timezone_valid: true,
        recurrence_within_limit: true,
        idempotency_available: true,
        conflict_policy_allows_write: true,
        external_invite_approved: true,
        availability_privacy_allowed: true,
        import_export_within_limit: true,
        provider_capability_available: true,
        rate_limit_available: true,
        timeout_within_limit: true,
        resource_budget_reserved: true,
    }
}

fn rejection_cases(value: &CalendarAdmissionEvidence) -> Vec<CalendarAdmissionEvidence> {
    vec![
        CalendarAdmissionEvidence {
            source_owned_by_caller: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            credential_secret_reference_valid: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            timezone_valid: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            recurrence_within_limit: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            idempotency_available: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            conflict_policy_allows_write: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            external_invite_approved: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            availability_privacy_allowed: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            import_export_within_limit: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            provider_capability_available: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            rate_limit_available: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            timeout_within_limit: false,
            ..value.clone()
        },
        CalendarAdmissionEvidence {
            resource_budget_reserved: false,
            ..value.clone()
        },
    ]
}
