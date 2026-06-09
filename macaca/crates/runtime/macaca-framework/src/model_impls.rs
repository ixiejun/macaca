//! Retired concrete chat model adapter surface.
//!
//! `macaca-framework` is now limited to provider-neutral model contracts,
//! transport DTOs, message formatters, and test harness abstractions. Concrete
//! HTTP model adapters belong in the LLM service or an approved runtime-host
//! provider package so framework consumers always cross the service boundary.
