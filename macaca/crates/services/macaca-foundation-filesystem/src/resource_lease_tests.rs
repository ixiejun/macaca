//! Deterministic release tests for filesystem resource leases.

use macaca_proto::{FilesystemResourceLimits, FilesystemResourceReservation};

use crate::FilesystemResourceLedger;

fn limits() -> FilesystemResourceLimits {
    FilesystemResourceLimits {
        max_byte_units: 10,
        max_entry_units: 10,
        max_recursive_operations: 1,
        max_watch_slots: 1,
        max_snapshot_units: 10,
        max_mutation_operations: 1,
        max_request_units: 1,
    }
}

#[test]
fn leases_release_after_success_error_or_cancelled_scope() {
    let ledger = FilesystemResourceLedger::new(limits());
    let request = FilesystemResourceReservation {
        byte_units: 4,
        request_units: 1,
        ..Default::default()
    };
    {
        let _success = ledger.reserve(request).unwrap();
        assert_eq!(ledger.current().unwrap().request_units, 1);
    }
    assert_eq!(
        ledger.current().unwrap(),
        FilesystemResourceReservation::default()
    );
    let failed = (|| -> Result<(), ()> {
        let _error = ledger.reserve(request).unwrap();
        Err(())
    })();
    assert!(failed.is_err());
    assert_eq!(
        ledger.current().unwrap(),
        FilesystemResourceReservation::default()
    );
}

#[test]
fn exceeded_reservations_do_not_mutate_current_capacity() {
    let ledger = FilesystemResourceLedger::new(limits());
    assert!(ledger
        .reserve(FilesystemResourceReservation {
            request_units: 2,
            ..Default::default()
        })
        .is_err());
    assert_eq!(
        ledger.current().unwrap(),
        FilesystemResourceReservation::default()
    );
}
