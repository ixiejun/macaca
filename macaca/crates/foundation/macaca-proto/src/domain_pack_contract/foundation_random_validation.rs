use super::foundation_random::{
    RandomBytesCommand, RandomIntegerCommand, RandomReplayPolicy, RandomSeedReference,
    RandomTestStreamCreateCommand, RandomTokenCommand,
};
use super::foundation_validation::{bounded_reference, opaque_artifact_reference};

impl RandomBytesCommand {
    /// Bound generated output before entropy and rate-limit policy evaluation.
    pub fn is_bounded_request(&self, max_bytes: u32, max_blocking_ms: u64) -> bool {
        self.length > 0
            && self.length <= max_bytes
            && self
                .max_blocking_ms
                .is_none_or(|value| value <= max_blocking_ms)
    }
}

impl RandomIntegerCommand {
    /// A bounded integer request must preserve a non-empty range.
    pub fn has_valid_range(&self) -> bool {
        self.min_inclusive < self.max_exclusive
    }
}

impl RandomTokenCommand {
    /// Bound token output and keep collision policy as a trace-safe identifier.
    pub fn is_bounded_request(&self, max_length: u32) -> bool {
        self.char_length > 0
            && self.char_length <= max_length
            && bounded_reference(&self.collision_warning_policy, 96)
    }
}

impl RandomSeedReference {
    /// Raw seed bytes must never appear in the protocol surface.
    pub fn is_safe_reference(&self) -> bool {
        opaque_artifact_reference(&self.seed_ref) && bounded_reference(&self.replay_binding, 160)
    }
}

impl RandomTestStreamCreateCommand {
    /// Admit deterministic generation only in declared test or replay contexts.
    pub fn is_allowed_in_context(&self, context: RandomReplayPolicy) -> bool {
        self.seed.is_safe_reference()
            && bounded_reference(&self.algorithm_id, 96)
            && self.replay_policy != RandomReplayPolicy::ProductionDenied
            && self.replay_policy == context
    }
}
