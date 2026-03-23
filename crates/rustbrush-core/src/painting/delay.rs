//! Adaptive delay controller for auto-tuning click timing.
//!
//! Starts with a base delay and adjusts up/down based on whether
//! commands succeed or fail, finding the optimal speed for the
//! current server connection.

use std::time::Duration;

/// Adaptive delay controller.
#[derive(Debug, Clone)]
pub struct AdaptiveDelay {
    /// Current delay in milliseconds.
    current_ms: u64,
    /// Minimum allowed delay.
    min_ms: u64,
    /// Maximum allowed delay.
    max_ms: u64,
    /// How many consecutive successes before reducing delay.
    speedup_threshold: u32,
    /// How much to reduce delay on speedup (percentage).
    speedup_factor: f64,
    /// How much to increase delay on failure (percentage).
    slowdown_factor: f64,
    /// Consecutive successes counter.
    consecutive_successes: u32,
    /// Whether adaptive mode is enabled.
    enabled: bool,
}

impl AdaptiveDelay {
    pub fn new(base_ms: u64) -> Self {
        Self {
            current_ms: base_ms,
            min_ms: 5,
            max_ms: 200,
            speedup_threshold: 50,
            speedup_factor: 0.9,
            slowdown_factor: 1.5,
            consecutive_successes: 0,
            enabled: true,
        }
    }

    /// Create a fixed (non-adaptive) delay.
    pub fn fixed(ms: u64) -> Self {
        Self {
            current_ms: ms,
            enabled: false,
            ..Self::new(ms)
        }
    }

    /// Get the current delay duration.
    pub fn current(&self) -> Duration {
        Duration::from_millis(self.current_ms)
    }

    /// Get the current delay in milliseconds.
    pub fn current_ms(&self) -> u64 {
        self.current_ms
    }

    /// Report that a command succeeded.
    pub fn report_success(&mut self) {
        if !self.enabled {
            return;
        }
        self.consecutive_successes += 1;
        if self.consecutive_successes >= self.speedup_threshold {
            self.consecutive_successes = 0;
            let new_ms = (self.current_ms as f64 * self.speedup_factor) as u64;
            self.current_ms = new_ms.max(self.min_ms);
        }
    }

    /// Report that a command failed (needs retry or caused issues).
    pub fn report_failure(&mut self) {
        if !self.enabled {
            return;
        }
        self.consecutive_successes = 0;
        let new_ms = (self.current_ms as f64 * self.slowdown_factor) as u64;
        self.current_ms = new_ms.min(self.max_ms);
    }

    /// Whether adaptive mode is enabled.
    pub fn is_adaptive(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_delay() {
        let delay = AdaptiveDelay::new(15);
        assert_eq!(delay.current_ms(), 15);
    }

    #[test]
    fn test_fixed_delay_doesnt_change() {
        let mut delay = AdaptiveDelay::fixed(15);
        for _ in 0..100 {
            delay.report_success();
        }
        assert_eq!(delay.current_ms(), 15);
        delay.report_failure();
        assert_eq!(delay.current_ms(), 15);
    }

    #[test]
    fn test_speedup_after_threshold() {
        let mut delay = AdaptiveDelay::new(100);
        // 50 successes should trigger speedup
        for _ in 0..50 {
            delay.report_success();
        }
        assert!(delay.current_ms() < 100);
        assert_eq!(delay.current_ms(), 90); // 100 * 0.9
    }

    #[test]
    fn test_slowdown_on_failure() {
        let mut delay = AdaptiveDelay::new(20);
        delay.report_failure();
        assert_eq!(delay.current_ms(), 30); // 20 * 1.5
    }

    #[test]
    fn test_min_max_bounds() {
        let mut delay = AdaptiveDelay::new(6);
        // Speed up past minimum
        for _ in 0..50 {
            delay.report_success();
        }
        assert!(delay.current_ms() >= 5);

        let mut delay = AdaptiveDelay::new(180);
        delay.report_failure();
        delay.report_failure();
        delay.report_failure();
        assert!(delay.current_ms() <= 200);
    }

    #[test]
    fn test_failure_resets_success_counter() {
        let mut delay = AdaptiveDelay::new(100);
        for _ in 0..49 {
            delay.report_success();
        }
        delay.report_failure(); // Reset counter
        delay.report_success(); // Only 1 success, no speedup
        assert!(delay.current_ms() >= 100); // Should have slowed down from failure
    }
}
