//! Declarative concurrency-isolation policy application.
//!
//! Pure function module — no I/O, fully unit-testable.

use super::types::ConcurrencyIsolationPolicy;

pub fn apply_concurrency_isolation(
    policy: &ConcurrencyIsolationPolicy,
    mut args: Vec<String>,
) -> Vec<String> {
    let already_covered = policy
        .skip_if_any_arg_prefix
        .iter()
        .any(|prefix| args.iter().any(|arg| arg.starts_with(prefix)));
    if already_covered {
        return args;
    }
    for required in &policy.required_args {
        if !args.iter().any(|arg| arg == required) {
            args.push(required.clone());
        }
    }
    args
}
