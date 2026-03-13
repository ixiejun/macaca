use std::sync::{Arc, Mutex};
use macaca_proto::types::TokenUsage;

/// Per-model pricing in USD per 1 000 tokens.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub prompt_per_1k: f64,
    pub completion_per_1k: f64,
}

impl ModelPricing {
    pub fn cost_for(&self, usage: TokenUsage) -> f64 {
        let prompt_cost = (usage.prompt_tokens as f64 / 1_000.0) * self.prompt_per_1k;
        let completion_cost = (usage.completion_tokens as f64 / 1_000.0) * self.completion_per_1k;
        prompt_cost + completion_cost
    }
}

/// Known model pricing table (approximate, USD).
pub fn default_pricing(model: &str) -> ModelPricing {
    // Strip version suffixes like "-20241022" for lookup simplicity.
    if model.starts_with("gpt-4o") {
        ModelPricing { prompt_per_1k: 0.005, completion_per_1k: 0.015 }
    } else if model.starts_with("gpt-4-turbo") || model.starts_with("gpt-4") {
        ModelPricing { prompt_per_1k: 0.01, completion_per_1k: 0.03 }
    } else if model.starts_with("gpt-3.5") {
        ModelPricing { prompt_per_1k: 0.0005, completion_per_1k: 0.0015 }
    } else if model.contains("claude-3-5-sonnet") {
        ModelPricing { prompt_per_1k: 0.003, completion_per_1k: 0.015 }
    } else if model.contains("claude-3-opus") {
        ModelPricing { prompt_per_1k: 0.015, completion_per_1k: 0.075 }
    } else if model.contains("claude-3") {
        ModelPricing { prompt_per_1k: 0.00025, completion_per_1k: 0.00125 }
    } else {
        // Unknown model — assume zero cost rather than panicking.
        ModelPricing { prompt_per_1k: 0.0, completion_per_1k: 0.0 }
    }
}

/// Accumulated usage and cost statistics, safe to share across threads.
#[derive(Debug, Default)]
struct Inner {
    total_prompt_tokens: u64,
    total_completion_tokens: u64,
    total_tokens: u64,
    total_cost_usd: f64,
    request_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    inner: Arc<Mutex<Inner>>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a completed request.
    pub fn record(&self, model: &str, usage: TokenUsage) {
        let pricing = default_pricing(model);
        let cost = pricing.cost_for(usage);

        let mut g = self.inner.lock().expect("CostTracker mutex poisoned");
        g.total_prompt_tokens += u64::from(usage.prompt_tokens);
        g.total_completion_tokens += u64::from(usage.completion_tokens);
        g.total_tokens += u64::from(usage.total_tokens);
        g.total_cost_usd += cost;
        g.request_count += 1;
    }

    pub fn total_prompt_tokens(&self) -> u64 {
        self.inner.lock().expect("CostTracker mutex poisoned").total_prompt_tokens
    }

    pub fn total_completion_tokens(&self) -> u64 {
        self.inner.lock().expect("CostTracker mutex poisoned").total_completion_tokens
    }

    pub fn total_tokens(&self) -> u64 {
        self.inner.lock().expect("CostTracker mutex poisoned").total_tokens
    }

    pub fn total_cost_usd(&self) -> f64 {
        self.inner.lock().expect("CostTracker mutex poisoned").total_cost_usd
    }

    pub fn request_count(&self) -> u64 {
        self.inner.lock().expect("CostTracker mutex poisoned").request_count
    }

    /// Reset all counters.
    pub fn reset(&self) {
        let mut g = self.inner.lock().expect("CostTracker mutex poisoned");
        *g = Inner::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(p: u32, c: u32) -> TokenUsage {
        TokenUsage { prompt_tokens: p, completion_tokens: c, total_tokens: p + c }
    }

    #[test]
    fn cost_accumulates() {
        let tracker = CostTracker::new();
        tracker.record("gpt-4o", usage(1000, 500));
        tracker.record("gpt-4o", usage(500, 200));

        assert_eq!(tracker.total_prompt_tokens(), 1500);
        assert_eq!(tracker.total_completion_tokens(), 700);
        assert_eq!(tracker.total_tokens(), 2200);
        assert_eq!(tracker.request_count(), 2);

        // gpt-4o: prompt 0.005/1k, completion 0.015/1k
        // request 1: 1.0*0.005 + 0.5*0.015 = 0.005 + 0.0075 = 0.0125
        // request 2: 0.5*0.005 + 0.2*0.015 = 0.0025 + 0.003  = 0.0055
        let expected = 0.0125 + 0.0055;
        let diff = (tracker.total_cost_usd() - expected).abs();
        assert!(diff < 1e-9, "cost mismatch: {}", tracker.total_cost_usd());
    }

    #[test]
    fn reset_clears_all() {
        let tracker = CostTracker::new();
        tracker.record("gpt-4o", usage(1000, 500));
        tracker.reset();
        assert_eq!(tracker.total_tokens(), 0);
        assert_eq!(tracker.total_cost_usd(), 0.0);
        assert_eq!(tracker.request_count(), 0);
    }

    #[test]
    fn unknown_model_zero_cost() {
        let tracker = CostTracker::new();
        tracker.record("llama-3-custom", usage(1000, 500));
        assert_eq!(tracker.total_cost_usd(), 0.0);
        assert_eq!(tracker.total_tokens(), 1500);
    }

    #[test]
    fn tracker_is_clone_shared() {
        let t1 = CostTracker::new();
        let t2 = t1.clone();
        t1.record("gpt-4o", usage(100, 50));
        assert_eq!(t2.total_tokens(), 150);
    }
}
