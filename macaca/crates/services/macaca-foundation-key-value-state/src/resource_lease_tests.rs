//! Deterministic release tests for key-value state resource leases.

use macaca_proto::{KeyValueResourceLimits, KeyValueResourceReservation};

use crate::KeyValueResourceLedger;

fn limits() -> KeyValueResourceLimits {
    KeyValueResourceLimits {
        max_byte_units: 10,
        max_entry_units: 10,
        max_batch_operations: 1,
        max_watch_slots: 1,
        max_snapshot_units: 10,
        max_mutation_operations: 1,
        max_request_units: 1,
    }
}

#[test]
fn leases_release_after_success_error_or_cancelled_scope() {
    let ledger = KeyValueResourceLedger::new(limits());
    let request = KeyValueResourceReservation {
        byte_units: 4,
        request_units: 1,
        ..Default::default()
    };
    {
        let _success = ledger.reserve(request).unwrap();
    }
    assert_eq!(
        ledger.current().unwrap(),
        KeyValueResourceReservation::default()
    );
    let failed = (|| -> Result<(), ()> {
        let _error = ledger.reserve(request).unwrap();
        Err(())
    })();
    assert!(failed.is_err());
    assert_eq!(
        ledger.current().unwrap(),
        KeyValueResourceReservation::default()
    );
}

#[tokio::test]
async fn timeout_drops_lease_and_releases_capacity() {
    let ledger = KeyValueResourceLedger::new(limits());
    let lease = ledger
        .reserve(KeyValueResourceReservation {
            request_units: 1,
            ..Default::default()
        })
        .unwrap();
    let timed = tokio::time::timeout(std::time::Duration::from_millis(1), async move {
        let _lease = lease;
        std::future::pending::<()>().await;
    })
    .await;
    assert!(timed.is_err());
    assert_eq!(
        ledger.current().unwrap(),
        KeyValueResourceReservation::default()
    );
}
