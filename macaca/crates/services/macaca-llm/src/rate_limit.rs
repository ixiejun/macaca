use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Sliding-window rate limiter.
///
/// Tracks request timestamps within the last `window` duration and blocks
/// (via `tokio::time::sleep`) when the limit is reached rather than rejecting
/// the request.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    /// Maximum number of requests allowed within the window.
    max_requests: usize,
    /// Width of the sliding window.
    window: Duration,
    /// Timestamps of recent requests (oldest first).
    timestamps: VecDeque<Instant>,
}

impl Inner {
    /// Drop entries that have fallen outside the window and return the current
    /// count of requests within it.
    fn evict_old(&mut self, now: Instant) -> usize {
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        while let Some(&front) = self.timestamps.front() {
            if front <= cutoff {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }
        self.timestamps.len()
    }

    /// If we are below the limit, record `now` and return `None`.
    /// Otherwise return the `Duration` to sleep before retrying.
    fn try_acquire(&mut self, now: Instant) -> Option<Duration> {
        let count = self.evict_old(now);
        if count < self.max_requests {
            self.timestamps.push_back(now);
            None
        } else {
            // Wait until the oldest request in the window expires.
            let oldest = *self.timestamps.front().expect("non-empty when at limit");
            let wait = (oldest + self.window).saturating_duration_since(now);
            Some(wait)
        }
    }
}

impl RateLimiter {
    /// Create a new limiter allowing `max_requests` per `window`.
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                max_requests,
                window,
                timestamps: VecDeque::new(),
            })),
        }
    }

    /// Convenience constructor for a requests-per-minute limit.
    pub fn per_minute(rpm: usize) -> Self {
        Self::new(rpm, Duration::from_secs(60))
    }

    /// Acquire a slot, sleeping as needed until one is available.
    pub async fn acquire(&self) {
        loop {
            let wait = {
                let mut g = self.inner.lock().expect("RateLimiter mutex poisoned");
                g.try_acquire(Instant::now())
            };
            match wait {
                None => return,
                Some(d) => tokio::time::sleep(d).await,
            }
        }
    }

    /// Returns the number of requests currently tracked within the window.
    pub fn current_count(&self) -> usize {
        let mut g = self.inner.lock().expect("RateLimiter mutex poisoned");
        g.evict_old(Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn allows_up_to_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        for _ in 0..3 {
            limiter.acquire().await;
        }
        assert_eq!(limiter.current_count(), 3);
    }

    #[tokio::test]
    async fn current_count_after_window_expires() {
        // Use a 1ms window so entries expire immediately.
        let limiter = RateLimiter::new(10, Duration::from_millis(1));
        limiter.acquire().await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert_eq!(limiter.current_count(), 0);
    }

    #[test]
    fn per_minute_constructor() {
        let limiter = RateLimiter::per_minute(60);
        assert_eq!(limiter.current_count(), 0);
    }
}
