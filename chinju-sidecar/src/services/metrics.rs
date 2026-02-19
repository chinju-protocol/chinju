//! Metrics Service - Prometheus Compatible Metrics
//!
//! Provides a /metrics endpoint for Prometheus scraping.
//! Tracks key operational metrics for the CHINJU Protocol.

use crate::services::lpt_monitor::LptMonitor;
use crate::services::token::TokenService;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

// =============================================================================
// Metrics Collector
// =============================================================================

/// Collects and exposes metrics for Prometheus
pub struct MetricsCollector {
    /// Total requests processed
    requests_total: AtomicU64,
    /// Successful requests
    requests_success: AtomicU64,
    /// Failed requests
    requests_failed: AtomicU64,
    /// Total tokens consumed
    tokens_consumed_total: AtomicU64,
    /// Request processing time sum (for average calculation)
    request_duration_sum_ms: AtomicU64,
    /// LPT monitor reference
    lpt_monitor: Option<Arc<LptMonitor>>,
    /// Token service reference
    token_service: Option<Arc<RwLock<TokenService>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            requests_success: AtomicU64::new(0),
            requests_failed: AtomicU64::new(0),
            tokens_consumed_total: AtomicU64::new(0),
            request_duration_sum_ms: AtomicU64::new(0),
            lpt_monitor: None,
            token_service: None,
        }
    }

    /// Set LPT monitor reference
    pub fn with_lpt_monitor(mut self, monitor: Arc<LptMonitor>) -> Self {
        self.lpt_monitor = Some(monitor);
        self
    }

    /// Set token service reference
    pub fn with_token_service(mut self, service: Arc<RwLock<TokenService>>) -> Self {
        self.token_service = Some(service);
        self
    }

    /// Record a successful request
    pub fn record_request_success(&self, duration_ms: u64, tokens_consumed: u64) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.requests_success.fetch_add(1, Ordering::Relaxed);
        self.request_duration_sum_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.tokens_consumed_total
            .fetch_add(tokens_consumed, Ordering::Relaxed);
    }

    /// Record a failed request
    pub fn record_request_failed(&self, duration_ms: u64) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.requests_failed.fetch_add(1, Ordering::Relaxed);
        self.request_duration_sum_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
    }

    /// Generate Prometheus-format metrics
    pub async fn render_metrics(&self) -> String {
        let mut output = String::new();

        // Request metrics
        let total = self.requests_total.load(Ordering::Relaxed);
        let success = self.requests_success.load(Ordering::Relaxed);
        let failed = self.requests_failed.load(Ordering::Relaxed);
        let duration_sum = self.request_duration_sum_ms.load(Ordering::Relaxed);
        let tokens_total = self.tokens_consumed_total.load(Ordering::Relaxed);

        output.push_str("# HELP chinju_requests_total Total number of requests processed\n");
        output.push_str("# TYPE chinju_requests_total counter\n");
        output.push_str(&format!("chinju_requests_total {}\n", total));

        output
            .push_str("# HELP chinju_requests_success_total Total number of successful requests\n");
        output.push_str("# TYPE chinju_requests_success_total counter\n");
        output.push_str(&format!("chinju_requests_success_total {}\n", success));

        output.push_str("# HELP chinju_requests_failed_total Total number of failed requests\n");
        output.push_str("# TYPE chinju_requests_failed_total counter\n");
        output.push_str(&format!("chinju_requests_failed_total {}\n", failed));

        output.push_str("# HELP chinju_request_duration_milliseconds_sum Sum of request processing times in milliseconds\n");
        output.push_str("# TYPE chinju_request_duration_milliseconds_sum counter\n");
        output.push_str(&format!(
            "chinju_request_duration_milliseconds_sum {}\n",
            duration_sum
        ));

        // Average request duration
        if total > 0 {
            let avg_duration = duration_sum as f64 / total as f64;
            output.push_str("# HELP chinju_request_duration_milliseconds_avg Average request processing time in milliseconds\n");
            output.push_str("# TYPE chinju_request_duration_milliseconds_avg gauge\n");
            output.push_str(&format!(
                "chinju_request_duration_milliseconds_avg {:.2}\n",
                avg_duration
            ));
        }

        output.push_str("# HELP chinju_tokens_consumed_total Total tokens consumed\n");
        output.push_str("# TYPE chinju_tokens_consumed_total counter\n");
        output.push_str(&format!("chinju_tokens_consumed_total {}\n", tokens_total));

        // LPT metrics
        if let Some(ref lpt) = self.lpt_monitor {
            let score = lpt.get_score().await;
            let state = lpt.get_state().await;

            output.push_str("# HELP chinju_lpt_score_total Current LPT total score (0.0-1.0)\n");
            output.push_str("# TYPE chinju_lpt_score_total gauge\n");
            output.push_str(&format!("chinju_lpt_score_total {:.4}\n", score.total));

            output.push_str("# HELP chinju_lpt_score_coherence LPT coherence score (0.0-1.0)\n");
            output.push_str("# TYPE chinju_lpt_score_coherence gauge\n");
            output.push_str(&format!(
                "chinju_lpt_score_coherence {:.4}\n",
                score.coherence
            ));

            output.push_str("# HELP chinju_lpt_score_efficiency LPT efficiency score (0.0-1.0)\n");
            output.push_str("# TYPE chinju_lpt_score_efficiency gauge\n");
            output.push_str(&format!(
                "chinju_lpt_score_efficiency {:.4}\n",
                score.efficiency
            ));

            output.push_str("# HELP chinju_lpt_score_latency LPT latency score (0.0-1.0)\n");
            output.push_str("# TYPE chinju_lpt_score_latency gauge\n");
            output.push_str(&format!("chinju_lpt_score_latency {:.4}\n", score.latency));

            output.push_str("# HELP chinju_lpt_score_repetition LPT repetition score (0.0-1.0)\n");
            output.push_str("# TYPE chinju_lpt_score_repetition gauge\n");
            output.push_str(&format!(
                "chinju_lpt_score_repetition {:.4}\n",
                score.repetition
            ));

            output.push_str(
                "# HELP chinju_lpt_sample_count Number of samples used for LPT calculation\n",
            );
            output.push_str("# TYPE chinju_lpt_sample_count gauge\n");
            output.push_str(&format!("chinju_lpt_sample_count {}\n", score.sample_count));

            // State as numeric (0=healthy, 1=warning, 2=critical, 3=degraded)
            let state_num = match state {
                crate::services::lpt_monitor::LptState::Healthy => 0,
                crate::services::lpt_monitor::LptState::Warning => 1,
                crate::services::lpt_monitor::LptState::Critical => 2,
                crate::services::lpt_monitor::LptState::Degraded => 3,
            };
            output.push_str("# HELP chinju_lpt_state Current LPT state (0=healthy, 1=warning, 2=critical, 3=degraded)\n");
            output.push_str("# TYPE chinju_lpt_state gauge\n");
            output.push_str(&format!("chinju_lpt_state {}\n", state_num));
        }

        // Token balance metrics
        if let Some(ref token_svc) = self.token_service {
            let svc = token_svc.read().await;
            let balance = svc.get_balance();

            output.push_str("# HELP chinju_token_balance Current token balance\n");
            output.push_str("# TYPE chinju_token_balance gauge\n");
            output.push_str(&format!("chinju_token_balance {}\n", balance));
        }

        output
    }

    /// Get current stats (non-Prometheus format)
    pub async fn get_stats(&self) -> MetricsStats {
        let total = self.requests_total.load(Ordering::Relaxed);
        let success = self.requests_success.load(Ordering::Relaxed);
        let failed = self.requests_failed.load(Ordering::Relaxed);
        let duration_sum = self.request_duration_sum_ms.load(Ordering::Relaxed);
        let tokens_total = self.tokens_consumed_total.load(Ordering::Relaxed);

        let avg_duration = if total > 0 {
            duration_sum as f64 / total as f64
        } else {
            0.0
        };

        let lpt_score = if let Some(ref lpt) = self.lpt_monitor {
            Some(lpt.get_score().await.total)
        } else {
            None
        };

        let token_balance = if let Some(ref token_svc) = self.token_service {
            Some(token_svc.read().await.get_balance())
        } else {
            None
        };

        MetricsStats {
            requests_total: total,
            requests_success: success,
            requests_failed: failed,
            avg_duration_ms: avg_duration,
            tokens_consumed_total: tokens_total,
            lpt_score,
            token_balance,
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics statistics struct
#[derive(Debug, Clone)]
pub struct MetricsStats {
    pub requests_total: u64,
    pub requests_success: u64,
    pub requests_failed: u64,
    pub avg_duration_ms: f64,
    pub tokens_consumed_total: u64,
    pub lpt_score: Option<f64>,
    pub token_balance: Option<u64>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.record_request_success(100, 50);
        collector.record_request_success(200, 75);
        collector.record_request_failed(50);

        let stats = collector.get_stats().await;
        assert_eq!(stats.requests_total, 3);
        assert_eq!(stats.requests_success, 2);
        assert_eq!(stats.requests_failed, 1);
        assert_eq!(stats.tokens_consumed_total, 125);
    }

    #[tokio::test]
    async fn test_prometheus_output() {
        let collector = MetricsCollector::new();
        collector.record_request_success(100, 50);

        let output = collector.render_metrics().await;
        assert!(output.contains("chinju_requests_total 1"));
        assert!(output.contains("chinju_tokens_consumed_total 50"));
    }
}
