use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// Sliding-window rate limiter that caps requests to `rpm` per 60-second window.
///
/// Multiple async tasks can share a single `RpmLimiter` behind an `Arc`.  Each
/// call to [`acquire`] blocks until a slot is available in the current window,
/// then records the timestamp of the allowed request.
pub struct RpmLimiter {
    rpm: u32,
    window: Mutex<VecDeque<Instant>>,
}

impl RpmLimiter {
    pub fn new(rpm: u32) -> Self {
        Self {
            rpm,
            window: Mutex::new(VecDeque::with_capacity(rpm as usize)),
        }
    }

    /// Wait until the RPM budget allows another request, then reserve a slot.
    pub async fn acquire(&self) {
        loop {
            let mut window = self.window.lock().await;
            let now = Instant::now();

            // Evict timestamps that have aged out of the 60-second window.
            // Use saturating_duration_since so a future timestamp (shouldn't happen)
            // is treated as age zero rather than causing a panic.
            window.retain(|t| now.saturating_duration_since(*t) < Duration::from_secs(60));

            if (window.len() as u32) < self.rpm {
                window.push_back(now);
                return;
            }

            // Sleep until the oldest in-window timestamp ages out, freeing a slot.
            let oldest = *window.front().unwrap();
            let elapsed = now.saturating_duration_since(oldest);
            let wait = Duration::from_secs(60).saturating_sub(elapsed) + Duration::from_millis(1);
            drop(window); // Release lock before sleeping so other tasks can check.
            tokio::time::sleep(wait).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn allows_up_to_rpm_immediately() {
        let limiter = Arc::new(RpmLimiter::new(5));
        // First 5 acquires should complete without sleeping.
        for _ in 0..5 {
            limiter.acquire().await;
        }
        let window = limiter.window.lock().await;
        assert_eq!(window.len(), 5);
    }

    /// Verifies that timestamps older than 60 seconds are evicted from the window,
    /// freeing slots for new requests.  Uses tokio's time-pause feature to advance
    /// the clock without sleeping in real time.
    #[tokio::test(start_paused = true)]
    async fn sliding_window_evicts_aged_out_timestamps() {
        let limiter = RpmLimiter::new(2);

        // Fill the window.
        limiter.acquire().await;
        limiter.acquire().await;
        {
            let window = limiter.window.lock().await;
            assert_eq!(window.len(), 2, "window should be full after 2 acquires");
        }

        // Advance the tokio clock past the 60-second eviction boundary.
        tokio::time::advance(Duration::from_secs(61)).await;

        // Both old timestamps should now be evicted; these two calls must complete
        // without blocking (no sleep needed).
        limiter.acquire().await;
        limiter.acquire().await;

        let window = limiter.window.lock().await;
        assert_eq!(
            window.len(),
            2,
            "window should contain only the two new timestamps"
        );
        // All entries should be recent (after the time advance).
        for t in window.iter() {
            // After a 61-second advance, new entries recorded via Instant::now()
            // should be less than 1 second old relative to the current paused clock.
            assert!(
                Instant::now().saturating_duration_since(*t) < Duration::from_secs(1),
                "timestamp should be recent"
            );
        }
    }

    #[tokio::test]
    async fn window_stays_within_rpm_across_calls() {
        let limiter = RpmLimiter::new(3);
        for _ in 0..3 {
            limiter.acquire().await;
        }
        let window = limiter.window.lock().await;
        assert!(
            window.len() as u32 <= 3,
            "window length should never exceed rpm"
        );
    }
}
