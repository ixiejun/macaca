//! Resource reservation Decorator primitives for filesystem provider calls.
//!
//! The ledger stores only counters, never roots, paths, handles, or content. A
//! lease is released by `Drop`, which makes timeout and task cancellation safe:
//! abandoning the dispatch future releases its reservation before a later call
//! can consume the same bounded capacity.

use std::sync::{Arc, Mutex};

use macaca_proto::{
    FilesystemResourceLimits, FilesystemResourceReservation, ServiceError, ServiceResult,
    FOUNDATION_FILESYSTEM_SERVICE_ID,
};

/// Thread-safe counter ledger owned by a filesystem service composition.
#[derive(Clone)]
pub struct FilesystemResourceLedger {
    limits: FilesystemResourceLimits,
    current: Arc<Mutex<FilesystemResourceReservation>>,
}

impl FilesystemResourceLedger {
    /// Create a bounded ledger for one provider/runtime composition.
    pub fn new(limits: FilesystemResourceLimits) -> Self {
        Self {
            limits,
            current: Arc::new(Mutex::new(FilesystemResourceReservation::default())),
        }
    }

    /// Reserve counters before a side-effecting provider call.
    pub fn reserve(
        &self,
        requested: FilesystemResourceReservation,
    ) -> ServiceResult<FilesystemResourceLease> {
        let mut current = self.current.lock().map_err(lock_error)?;
        let next = add(*current, requested);
        if exceeds(next, self.limits) {
            tracing::warn!(
                service_id = FOUNDATION_FILESYSTEM_SERVICE_ID,
                "filesystem resource reservation rejected"
            );
            return Err(ServiceError::DisabledByPolicy(
                "filesystem resource reservation exceeds configured limits".into(),
            ));
        }
        *current = next;
        tracing::info!(
            service_id = FOUNDATION_FILESYSTEM_SERVICE_ID,
            request_units = requested.request_units,
            "filesystem resource reservation acquired"
        );
        Ok(FilesystemResourceLease {
            current: Arc::clone(&self.current),
            reservation: requested,
        })
    }

    /// Return a counter-only snapshot for health and deterministic tests.
    pub fn current(&self) -> ServiceResult<FilesystemResourceReservation> {
        self.current.lock().map(|value| *value).map_err(lock_error)
    }
}

/// RAII lease that releases resource counters on result, error, timeout, or cancellation.
pub struct FilesystemResourceLease {
    current: Arc<Mutex<FilesystemResourceReservation>>,
    reservation: FilesystemResourceReservation,
}

impl Drop for FilesystemResourceLease {
    fn drop(&mut self) {
        if let Ok(mut current) = self.current.lock() {
            *current = subtract(*current, self.reservation);
            tracing::info!(
                service_id = FOUNDATION_FILESYSTEM_SERVICE_ID,
                request_units = self.reservation.request_units,
                "filesystem resource reservation released"
            );
        }
    }
}

fn add(
    current: FilesystemResourceReservation,
    requested: FilesystemResourceReservation,
) -> FilesystemResourceReservation {
    FilesystemResourceReservation {
        byte_units: current.byte_units.saturating_add(requested.byte_units),
        entry_units: current.entry_units.saturating_add(requested.entry_units),
        recursive_operations: current
            .recursive_operations
            .saturating_add(requested.recursive_operations),
        watch_slots: current.watch_slots.saturating_add(requested.watch_slots),
        snapshot_units: current
            .snapshot_units
            .saturating_add(requested.snapshot_units),
        mutation_operations: current
            .mutation_operations
            .saturating_add(requested.mutation_operations),
        request_units: current
            .request_units
            .saturating_add(requested.request_units),
    }
}

fn subtract(
    current: FilesystemResourceReservation,
    released: FilesystemResourceReservation,
) -> FilesystemResourceReservation {
    FilesystemResourceReservation {
        byte_units: current.byte_units.saturating_sub(released.byte_units),
        entry_units: current.entry_units.saturating_sub(released.entry_units),
        recursive_operations: current
            .recursive_operations
            .saturating_sub(released.recursive_operations),
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

fn exceeds(value: FilesystemResourceReservation, limits: FilesystemResourceLimits) -> bool {
    value.byte_units > limits.max_byte_units
        || value.entry_units > limits.max_entry_units
        || value.recursive_operations > limits.max_recursive_operations
        || value.watch_slots > limits.max_watch_slots
        || value.snapshot_units > limits.max_snapshot_units
        || value.mutation_operations > limits.max_mutation_operations
        || value.request_units > limits.max_request_units
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> ServiceError {
    ServiceError::AdapterFailure("filesystem resource ledger lock poisoned".into())
}
