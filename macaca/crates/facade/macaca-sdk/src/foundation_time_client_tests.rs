//! Time SDK Facade tests prove that helpers use declared traced service calls.

use macaca_proto::{
    compose_installed_domain_pack_catalog, AppServiceContractConfig, DomainPackAvailability,
    TraceContext, FOUNDATION_TIME_PACK_ID, FOUNDATION_TIME_SERVICE_ID,
};
use macaca_proto::{TimeAdmissionFailure, TimeResourceReservation};

use super::{
    mock_clock_setup_command, now_command, timer_create_command, timezone_conversion_command,
    TimeDomainPackCommandBuildOutcome,
};
use crate::domain_pack_client::{DomainPackResolveResult, SystemDomainPackClient};
use crate::{CatalogBackedDomainPackClient, DomainPackResolveCommand};

async fn resolved() -> DomainPackResolveResult {
    let mut definition = macaca_proto::foundation_time_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    CatalogBackedDomainPackClient::new(compose_installed_domain_pack_catalog(vec![definition]))
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_TIME_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn helpers_build_canonical_time_service_calls() {
    let outcome = timezone_conversion_command(
        serde_json::json!({"target_timezone":{"zone_id":"UTC"}}),
        TraceContext::new("trace-sdk-time"),
    )
    .build(&resolved().await)
    .unwrap();
    let TimeDomainPackCommandBuildOutcome::Ready(command) = outcome else {
        panic!("expected ready")
    };
    assert_eq!(command.service_id, FOUNDATION_TIME_SERVICE_ID);
    assert_eq!(command.command_name, "time.convert_timezone");
}

#[tokio::test]
async fn clock_read_helpers_build_traced_canonical_calls() {
    for (trace_id, builder) in [
        (
            "trace-sdk-time-now",
            now_command(
                serde_json::json!({}),
                TraceContext::new("trace-sdk-time-now"),
            ),
        ),
        (
            "trace-sdk-time-mock",
            mock_clock_setup_command(
                serde_json::json!({"source":"frozen-test-clock"}),
                TraceContext::new("trace-sdk-time-mock"),
            ),
        ),
    ] {
        let outcome = builder.build(&resolved().await).unwrap();
        let TimeDomainPackCommandBuildOutcome::Ready(command) = outcome else {
            panic!("expected ready")
        };
        assert_eq!(command.service_id, FOUNDATION_TIME_SERVICE_ID);
        assert_eq!(command.command_name, "time.now");
        assert_eq!(command.trace.unwrap().trace_id.as_str(), trace_id);
    }
}

#[tokio::test]
async fn timer_rejection_does_not_create_a_service_call() {
    let outcome = timer_create_command(
        serde_json::json!({}),
        Err(TimeAdmissionFailure::QuotaExceeded),
        TraceContext::new("trace-sdk-time-denied"),
    )
    .build(&resolved().await)
    .unwrap();
    assert_eq!(
        outcome,
        TimeDomainPackCommandBuildOutcome::Rejected(TimeAdmissionFailure::QuotaExceeded)
    );
}

#[tokio::test]
async fn timer_helper_preserves_trace_and_admission_evidence() {
    let outcome = timer_create_command(
        serde_json::json!({"duration":{"millis":10}}),
        Ok(TimeResourceReservation {
            reservation_id: "reserved".into(),
            timer_count: 1,
            duration_ms: 10,
        }),
        TraceContext::new("trace-sdk-time-timer"),
    )
    .build(&resolved().await)
    .unwrap();
    let TimeDomainPackCommandBuildOutcome::Ready(command) = outcome else {
        panic!("expected ready")
    };
    assert_eq!(
        command.trace.unwrap().trace_id.as_str(),
        "trace-sdk-time-timer"
    );
}
