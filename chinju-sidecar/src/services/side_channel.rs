//! Side Channel Blocking (C13)
//!
//! Prevents information leakage through timing and power consumption patterns:
//! - Constant time responses (normalized to intervals)
//! - Random delays
//! - Dummy computations for power pattern obfuscation

use rand::Rng;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, trace};

/// Configuration for side channel blocking
#[derive(Debug, Clone)]
pub struct SideChannelConfig {
    /// Minimum response time in milliseconds
    pub min_response_ms: u64,
    /// Response time granularity in milliseconds (responses are rounded to this)
    pub granularity_ms: u64,
    /// Maximum random jitter in milliseconds
    pub max_jitter_ms: u64,
    /// Enable dummy computations for power obfuscation
    pub enable_dummy_load: bool,
    /// Intensity of dummy load (0.0 - 1.0)
    pub dummy_load_intensity: f64,
}

impl Default for SideChannelConfig {
    fn default() -> Self {
        Self {
            min_response_ms: 100,
            granularity_ms: 500,
            max_jitter_ms: 50,
            enable_dummy_load: true,
            dummy_load_intensity: 0.3,
        }
    }
}

/// Side channel blocker
pub struct SideChannelBlocker {
    config: SideChannelConfig,
}

impl SideChannelBlocker {
    /// Create new side channel blocker with default config
    pub fn new() -> Self {
        Self::with_config(SideChannelConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: SideChannelConfig) -> Self {
        tracing::info!(
            "Initializing SideChannelBlocker: min={}ms, granularity={}ms, jitter={}ms, dummy_load={}",
            config.min_response_ms,
            config.granularity_ms,
            config.max_jitter_ms,
            config.enable_dummy_load
        );
        Self { config }
    }

    /// Calculate the target response time based on actual processing time
    pub fn calculate_target_time(&self, actual_ms: u64) -> Duration {
        let mut rng = rand::thread_rng();

        // Ensure minimum response time
        let base_ms = actual_ms.max(self.config.min_response_ms);

        // Round up to next granularity interval
        let rounded_ms = ((base_ms / self.config.granularity_ms) + 1) * self.config.granularity_ms;

        // Add random jitter
        let jitter_ms = rng.gen_range(0..=self.config.max_jitter_ms);

        Duration::from_millis(rounded_ms + jitter_ms)
    }

    /// Wait until target time has elapsed since start
    pub async fn wait_until_target(&self, start: Instant, actual_duration: Duration) {
        let actual_ms = actual_duration.as_millis() as u64;
        let target = self.calculate_target_time(actual_ms);
        let elapsed = start.elapsed();

        if elapsed < target {
            let wait_duration = target - elapsed;
            debug!(
                actual_ms = actual_ms,
                target_ms = target.as_millis(),
                wait_ms = wait_duration.as_millis(),
                "Adding delay for timing normalization"
            );

            // Optionally run dummy computations while waiting
            if self.config.enable_dummy_load && wait_duration > Duration::from_millis(10) {
                self.dummy_computation_async(wait_duration).await;
            } else {
                sleep(wait_duration).await;
            }
        } else {
            trace!(
                actual_ms = actual_ms,
                elapsed_ms = elapsed.as_millis(),
                "No delay needed, already past target time"
            );
        }
    }

    /// Perform dummy computations for the specified duration
    /// This helps obfuscate power consumption patterns
    async fn dummy_computation_async(&self, duration: Duration) {
        let intensity = self.config.dummy_load_intensity;
        let start = Instant::now();

        // Mix sleep and computation based on intensity
        let compute_portion = (duration.as_millis() as f64 * intensity) as u64;

        // Do some dummy computation
        if compute_portion > 0 {
            tokio::task::spawn_blocking(move || {
                Self::dummy_computation_blocking(Duration::from_millis(compute_portion));
            })
            .await
            .ok();
        }

        // Sleep for remaining time
        let elapsed = start.elapsed();
        if elapsed < duration {
            sleep(duration - elapsed).await;
        }
    }

    /// Blocking dummy computation (runs on blocking thread pool)
    fn dummy_computation_blocking(duration: Duration) {
        let start = Instant::now();
        let mut accumulator: u64 = 0;
        let mut rng = rand::thread_rng();

        while start.elapsed() < duration {
            // Random matrix-like operations
            for _ in 0..1000 {
                let a: u64 = rng.gen();
                let b: u64 = rng.gen();
                accumulator = accumulator.wrapping_add(a.wrapping_mul(b));
            }
        }

        // Prevent optimization from removing the computation
        std::hint::black_box(accumulator);
    }

    /// Execute a function with timing normalization
    pub async fn execute_with_timing<F, T>(&self, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = f.await;
        let actual_duration = start.elapsed();

        self.wait_until_target(start, actual_duration).await;

        result
    }

    /// Get config
    pub fn config(&self) -> &SideChannelConfig {
        &self.config
    }
}

impl Default for SideChannelBlocker {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard for automatic timing normalization
pub struct TimingGuard<'a> {
    blocker: &'a SideChannelBlocker,
    start: Instant,
}

impl<'a> TimingGuard<'a> {
    /// Create a new timing guard
    pub fn new(blocker: &'a SideChannelBlocker) -> Self {
        Self {
            blocker,
            start: Instant::now(),
        }
    }

    /// Finish and apply timing normalization
    pub async fn finish(self) {
        let actual_duration = self.start.elapsed();
        self.blocker
            .wait_until_target(self.start, actual_duration)
            .await;
    }

    /// Get elapsed time since guard creation
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_time_calculation() {
        let config = SideChannelConfig {
            min_response_ms: 100,
            granularity_ms: 500,
            max_jitter_ms: 0, // No jitter for predictable testing
            enable_dummy_load: false,
            dummy_load_intensity: 0.0,
        };
        let blocker = SideChannelBlocker::with_config(config);

        // 50ms actual -> rounded up to 500ms (first granularity above min 100ms)
        let target = blocker.calculate_target_time(50);
        assert_eq!(target.as_millis(), 500);

        // 100ms actual -> rounded up to 500ms
        let target = blocker.calculate_target_time(100);
        assert_eq!(target.as_millis(), 500);

        // 400ms actual -> rounded up to 500ms
        let target = blocker.calculate_target_time(400);
        assert_eq!(target.as_millis(), 500);

        // 500ms actual -> rounded up to 1000ms (next interval)
        let target = blocker.calculate_target_time(500);
        assert_eq!(target.as_millis(), 1000);

        // 750ms actual -> rounded up to 1000ms
        let target = blocker.calculate_target_time(750);
        assert_eq!(target.as_millis(), 1000);
    }

    #[test]
    fn test_jitter() {
        let config = SideChannelConfig {
            min_response_ms: 100,
            granularity_ms: 500,
            max_jitter_ms: 50,
            enable_dummy_load: false,
            dummy_load_intensity: 0.0,
        };
        let blocker = SideChannelBlocker::with_config(config);

        // Run multiple times to verify jitter is being added
        let mut targets = std::collections::HashSet::new();
        for _ in 0..100 {
            let target = blocker.calculate_target_time(100);
            targets.insert(target.as_millis());
        }

        // With jitter, we should see some variation
        assert!(targets.len() > 1, "Expected jitter to cause variation");

        // All values should be between 500 and 550
        for &t in &targets {
            assert!(t >= 500 && t <= 550, "Target {} out of expected range", t);
        }
    }

    #[tokio::test]
    async fn test_execute_with_timing() {
        let config = SideChannelConfig {
            min_response_ms: 100,
            granularity_ms: 200,
            max_jitter_ms: 0,
            enable_dummy_load: false,
            dummy_load_intensity: 0.0,
        };
        let blocker = SideChannelBlocker::with_config(config);

        let start = Instant::now();
        let result = blocker
            .execute_with_timing(async {
                sleep(Duration::from_millis(50)).await;
                42
            })
            .await;

        let elapsed = start.elapsed();

        assert_eq!(result, 42);
        // Should be at least 200ms (rounded up from 50ms actual)
        assert!(
            elapsed >= Duration::from_millis(200),
            "Expected at least 200ms, got {:?}",
            elapsed
        );
        // But not too much more
        assert!(
            elapsed < Duration::from_millis(300),
            "Expected less than 300ms, got {:?}",
            elapsed
        );
    }
}
