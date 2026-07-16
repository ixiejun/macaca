## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API comparison notes for IANA tzdb, Google Time Zone API, GeoNames Timezone API, Mapbox Tilequery-style boundary lookup, timezone-boundary-builder, Unicode CLDR/ICU, Java `java.time`, JavaScript Temporal, and Windows time-zone identifiers.
- [x] 1.3 Confirm boundaries with foundation time, location geocode, location maps, location place-search, location route, workflow schedule, and communication calendar so timezone does not absorb unrelated scheduling, geolocation capture, or map behavior.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral commands for `timezone.lookup_by_coordinates`, `timezone.resolve_zone`, `timezone.get_offset`, `timezone.list_transitions`, `timezone.convert_instant`, `timezone.resolve_local_time`, `timezone.get_display_names`, `timezone.inspect_database`, and `timezone.inspect_mapping`.
- [x] 2.2 Define `TimezoneCommandContext`, `TimezoneCoordinateQuery`, `TimezoneZone`, `TimezoneLookupResult`, `TimezoneOffset`, `TimezoneTransition`, `TimezoneLocalResolution`, `TimezoneDisplayNames`, `TimezoneDatabaseInfo`, `TimezoneBoundaryProvenance`, `TimezoneIdentifierMapping`, and `TimezoneError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, invalid-zone, invalid-coordinate, ambiguous-boundary, stale-database, nonexistent-local-time, ambiguous-local-time, quota-exceeded, provider-failure, and conflict results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, provider classes, identifier systems, dataset versions, freshness policy, boundary provenance, display-name datasets, permissions, resource budgets, SDK metadata, compatibility, diagnostics, and documentation URL.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, tzdb version fixtures, DST gap/fold fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `location.timezone.lookup.read`, `location.timezone.offset.read`, `location.timezone.names.read`, and `location.timezone.database.inspect`.
- [ ] 3.2 Enforce coordinate precision, spatial redaction, region, data freshness, retention, provider-class, identifier-system, and bounded range policies before dispatch.
- [ ] 3.3 Require explicit local-time resolver strategy for gap/fold handling: reject, earlier, later, compatible, or explicit offset.
- [ ] 3.4 Add resource reservation and quota checks for remote calls, transition-list range, page size, boundary dataset access, display-name lookup, retained snapshots, and replay metadata.
- [ ] 3.5 Add approval behavior for precise coordinate lookup, host-native provider access, external network disclosure, stale data override, and high-volume transition queries.
- [ ] 3.6 Add tests proving denied, unavailable, unsupported, stale-database, and quota paths do not call concrete providers.

## 4. Service Provider And Replacement Strategy

- [ ] 4.1 Implement the timezone service provider contract behind the service runtime; do not construct providers from kernel, SDK, shells, or generic application-framework code.
- [x] 4.2 Add provider descriptor support for embedded tzdb, boundary lookup, remote API, host-native, display-name, mock, and unavailable provider classes.
- [x] 4.3 Add mock and unavailable providers for deterministic tests; external supplier adapters must remain optional providers or plugin/remote modules.
- [ ] 4.4 Add provider conformance tests for coordinate lookup, alias resolution, Windows/IANA mapping, offset calculation, transition listing, instant conversion, local gap/fold resolution, display names, database inspection, redaction, and unsupported-command reporting.
- [ ] 4.5 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, dataset refresh, stale-database reporting, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.location.timezone.v1` with command schemas, DTO schemas, permission scopes, examples, availability, provider classes, database versions, freshness, diagnostics, compatibility, and documentation URL.
- [ ] 5.2 Extend application admission so required declarations block when unavailable or stale beyond policy and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers or branch on provider names.
- [ ] 5.4 Add WASM/application ABI exposure for timezone commands using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for coordinate lookup, zone normalization, offset calculation, transition listing, instant conversion, local-time gap/fold resolution, display names, database inspection, and unavailable diagnostics.

## 6. Trace, Audit, Replay, And Boundary Gates

- [ ] 6.1 Emit sanitized `timezone.pack_declared`, `timezone.admission_validated`, `timezone.policy_decision`, `timezone.entitlement_checked`, `timezone.resource_reserved`, `timezone.command_requested`, `timezone.provider_selected`, `timezone.command_succeeded`, `timezone.command_failed`, `timezone.unavailable`, `timezone.database_stale`, and `timezone.snapshot_recorded` events.
- [ ] 6.2 Add replay tests proving every command is trace-addressable through the canonical service path after refresh/restart and includes database-version evidence.
- [ ] 6.3 Add dependency-boundary gates proving microkernel, SDK, shells, and generic application framework do not import concrete timezone providers or embedded tzdb loaders.
- [ ] 6.4 Add no-direct-provider-call gates proving all timezone commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [ ] 6.5 Add redaction tests for exact coordinates, raw provider payloads, raw boundary geometry, credentials, database paths, host identifiers, snapshots, and stale-database diagnostics.
- [ ] 6.6 Run `openspec validate add-pack-location-timezone --strict`, DTO compatibility tests, DST gap/fold conformance tests, tzdb version tests, boundary gates, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/location/timezone.md` with purpose, manifest declarations, required/optional behavior, scopes, command DTOs, result DTOs, IANA/Windows mappings, instant/local semantics, DST gap/fold strategies, database freshness, boundary ambiguity, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, adapter responsibilities, tzdb/boundary/display-name versioning, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add minimal app-facing examples for coordinate lookup, offset calculation, transition listing, local-time gap/fold resolution, display-name lookup, database inspection, and unavailable-provider diagnostics using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-location-timezone` complete.
