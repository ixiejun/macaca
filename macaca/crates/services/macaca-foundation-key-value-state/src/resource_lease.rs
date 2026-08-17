//! RAII resource reservations for key-value state provider calls.
//!
//! The ledger holds counters only. Dropping a lease on success, error, timeout,
//! cancellation, or terminated watch releases capacity without retaining a
//! namespace, key, value, provider payload, or application-specific state.

use std::sync::{Arc, Mutex};

use macaca_proto::{
    KeyValueResourceLimits, KeyValueResourceReservation, ServiceError, ServiceResult,
    FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
};

/// Thread-safe counter ledger owned by one key-value service composition.
#[derive(Clone)]
pub struct KeyValueResourceLedger {
    limits: KeyValueResourceLimits,
    current: Arc<Mutex<KeyValueResourceReservation>>,
}

impl KeyValueResourceLedger {
    /// Create a bounded ledger for one provider/runtime composition.
    pub fn new(limits: KeyValueResourceLimits) -> Self {
        Self {
            limits,
            current: Arc::new(Mutex::new(KeyValueResourceReservation::default())),
        }
    }

    /// Reserve counter capacity before a side-effecting provider call begins.
    pub fn reserve(
        &self,
        requested: KeyValueResourceReservation,
    ) -> ServiceResult<KeyValueResourceLease> {
        let mut current = self.current.lock().map_err(lock_error)?;
        let next = macaca_proto::reserve_key_value_resources(*current, requested, self.limits)
            .map_err(|_| {
                ServiceError::DisabledByPolicy(
                    "key-value resource reservation exceeds configured limits".into(),
                )
            })?;
        *current = next;
        tracing::info!(
            service_id = FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
            request_units = requested.request_units,
            "key-value resource reservation acquired"
        );
        Ok(KeyValueResourceLease {
            current: Arc::clone(&self.current),
            reservation: requested,
        })
    }

    /// Return a counter-only snapshot for health probes and deterministic tests.
    pub fn current(&self) -> ServiceResult<KeyValueResourceReservation> {
        self.current.lock().map(|value| *value).map_err(lock_error)
    }
}

/// RAII lease that guarantees resource release when dispatch scope terminates.
pub struct KeyValueResourceLease {
    current: Arc<Mutex<KeyValueResourceReservation>>,
    reservation: KeyValueResourceReservation,
}

impl Drop for KeyValueResourceLease {
    fn drop(&mut self) {
        if let Ok(mut current) = self.current.lock() {
            *current = subtract(*current, self.reservation);
            tracing::info!(
                service_id = FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
                request_units = self.reservation.request_units,
                "key-value resource reservation released"
            );
        }
    }
}

fn subtract(
    current: KeyValueResourceReservation,
    released: KeyValueResourceReservation,
) -> KeyValueResourceReservation {
    KeyValueResourceReservation {
        byte_units: current.byte_units.saturating_sub(released.byte_units),
        entry_units: current.entry_units.saturating_sub(released.entry_units),
        batch_operations: current
            .batch_operations
            .saturating_sub(released.batch_operations),
        watch_slots: current.watch_slots.saturating_sub(released.watch_slots),
        snapshot_units: current
            .snapshot_units
            .saturating_sub(released.snapshot_units),
        mutation_operations: current
            .mutation_operations
            .saturating_sub(released.mutation_operations),
        request_units: current.request_units.saturating_sub(released.request_units),
    }
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> ServiceError {
    ServiceError::AdapterFailure("key-value resource ledger lock poisoned".into())
}
