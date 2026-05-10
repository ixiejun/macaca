//! Context budget primitives.

use serde::{Deserialize, Serialize};

/// Provider-neutral request budget used by context engines.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextBudget {
    pub max_tokens: u32,
    pub reserve_output_tokens: u32,
}

impl ContextBudget {
    pub fn new(max_tokens: u32, reserve_output_tokens: u32) -> Self {
        Self {
            max_tokens,
            reserve_output_tokens,
        }
    }

    pub fn input_budget(&self) -> u32 {
        self.max_tokens.saturating_sub(self.reserve_output_tokens)
    }
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_tokens: 120_000,
            reserve_output_tokens: 4_096,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_budget_saturates() {
        assert_eq!(ContextBudget::new(100, 40).input_budget(), 60);
        assert_eq!(ContextBudget::new(10, 40).input_budget(), 0);
    }
}
