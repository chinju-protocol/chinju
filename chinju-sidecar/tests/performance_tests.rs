//! Performance Tests for CHINJU Sidecar Services
//!
//! Tests throughput and latency characteristics for:
//! - C14 CapabilityEvaluator
//! - C15 ValueNeuronMonitor
//! - C16 ContradictionController
//! - C17 SurvivalAttentionService

use chinju_sidecar::services::contradiction_controller::{
    CollapseDetector, ContextLimitConfig, ContradictionConfig, ContradictionStrength,
    ContradictionType, InjectionTiming, PaddingGenerator, PaddingType,
};
use chinju_sidecar::services::survival_attention::RiskLevel;
use chinju_sidecar::services::{
    CapabilityEvaluator, ContradictionController, SurvivalAttentionService, ValueNeuronMonitor,
};
use std::time::{Duration, Instant};

// =============================================================================
// Performance Test Helpers
// =============================================================================

/// Measure execution time and throughput
#[allow(dead_code)]
struct PerfResult {
    total_time: Duration,
    iterations: usize,
    ops_per_sec: f64,
    avg_latency_us: f64,
    p99_latency_us: f64,
}

impl PerfResult {
    fn from_latencies(latencies: &mut [Duration], iterations: usize) -> Self {
        let total_time: Duration = latencies.iter().sum();

        // Sort for percentile calculation
        latencies.sort();

        let ops_per_sec = iterations as f64 / total_time.as_secs_f64();
        let avg_latency_us = total_time.as_micros() as f64 / iterations as f64;
        let p99_idx = (latencies.len() as f64 * 0.99) as usize;
        let p99_latency_us = latencies
            .get(p99_idx)
            .unwrap_or(latencies.last().unwrap_or(&Duration::ZERO))
            .as_micros() as f64;

        Self {
            total_time,
            iterations,
            ops_per_sec,
            avg_latency_us,
            p99_latency_us,
        }
    }

    fn assert_throughput(&self, min_ops_per_sec: f64, test_name: &str) {
        assert!(
            self.ops_per_sec >= min_ops_per_sec,
            "{}: throughput {:.2} ops/sec below minimum {:.2}",
            test_name,
            self.ops_per_sec,
            min_ops_per_sec
        );
    }

    fn assert_latency(&self, max_avg_us: f64, test_name: &str) {
        assert!(
            self.avg_latency_us <= max_avg_us,
            "{}: avg latency {:.2}us exceeds maximum {:.2}us",
            test_name,
            self.avg_latency_us,
            max_avg_us
        );
    }
}

// =============================================================================
// C14 CapabilityEvaluator Performance Tests
// =============================================================================

#[tokio::test]
async fn perf_capability_evaluator_complexity() {
    let evaluator = CapabilityEvaluator::new();
    let iterations = 1000;
    let mut latencies = Vec::with_capacity(iterations);

    // Warm up
    for _ in 0..10 {
        evaluator.evaluate_complexity("Warm up query", None).await;
    }

    // Measure
    for i in 0..iterations {
        let input = format!("This is test query number {} for complexity evaluation", i);
        let start = Instant::now();
        evaluator.evaluate_complexity(&input, None).await;
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C14 evaluate_complexity: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 5000 ops/sec, avg < 500us
    result.assert_throughput(5000.0, "C14 evaluate_complexity");
    result.assert_latency(500.0, "C14 evaluate_complexity");
}

#[tokio::test]
async fn perf_capability_evaluator_drift_detection() {
    let evaluator = CapabilityEvaluator::new();

    // Pre-populate history
    for i in 0..100 {
        evaluator
            .evaluate_complexity(&format!("Input {}", i), None)
            .await;
    }

    let iterations = 500;
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        evaluator.detect_drift().await;
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C14 detect_drift: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 1000 ops/sec
    result.assert_throughput(1000.0, "C14 detect_drift");
}

#[test]
fn perf_token_complexity_details() {
    let iterations = 5000;
    let mut latencies = Vec::with_capacity(iterations);

    let test_texts = [
        "Simple input text",
        "The quick brown fox jumps over the lazy dog repeatedly in this test",
        "Complex philosophical discourse about epistemological paradigms in contemporary theoretical physics necessitates fundamental reconsideration of deterministic models",
    ];

    for i in 0..iterations {
        let text = test_texts[i % test_texts.len()];
        let start = Instant::now();
        let _ = CapabilityEvaluator::get_token_complexity_details(text);
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C14 token_complexity_details: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 20000 ops/sec (debug build, pure CPU)
    result.assert_throughput(20000.0, "C14 token_complexity_details");
}

// =============================================================================
// C15 ValueNeuronMonitor Performance Tests
// =============================================================================

#[tokio::test]
async fn perf_value_neuron_rpe_recording() {
    let monitor = ValueNeuronMonitor::new();
    let iterations = 1000;
    let mut latencies = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let rpe_value = (i as f64 / 100.0).sin(); // Oscillating value
        let start = Instant::now();
        monitor.record_rpe(rpe_value).await;
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C15 record_rpe: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 10000 ops/sec
    result.assert_throughput(10000.0, "C15 record_rpe");
}

#[tokio::test]
async fn perf_value_neuron_health_check() {
    let monitor = ValueNeuronMonitor::new();

    // Pre-populate RPE history
    for i in 0..50 {
        monitor.record_rpe((i as f64 / 10.0).sin()).await;
    }

    let iterations = 500;
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        monitor.get_health().await;
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C15 get_health: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 5000 ops/sec
    result.assert_throughput(5000.0, "C15 get_health");
}

// =============================================================================
// C16 ContradictionController Performance Tests
// =============================================================================

#[tokio::test]
async fn perf_contradiction_injection() {
    let controller = ContradictionController::new();
    let iterations = 500;
    let mut latencies = Vec::with_capacity(iterations);

    let config = ContradictionConfig {
        contradiction_type: ContradictionType::Direct,
        strength: ContradictionStrength::Medium,
        timing: InjectionTiming::Prepend,
        custom_template: None,
        target_task: Some("calculation".to_string()),
    };

    // Start a session
    controller
        .start_control("perf-session", ContextLimitConfig::default(), config)
        .await;

    for i in 0..iterations {
        let prompt = format!("Calculate {} + {} and provide the result", i, i * 2);
        let start = Instant::now();
        let _ = controller.prepare_injection("perf-session", &prompt).await;
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C16 prepare_injection: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 5000 ops/sec
    result.assert_throughput(5000.0, "C16 prepare_injection");
}

#[test]
fn perf_padding_generator() {
    let generator = PaddingGenerator::with_seed(42);
    let iterations = 2000;
    let mut latencies = Vec::with_capacity(iterations);

    let padding_types = [
        PaddingType::Random,
        PaddingType::Semantic,
        PaddingType::TaskRelevant,
    ];
    let token_counts = [50, 100, 200];

    for i in 0..iterations {
        let padding_type = padding_types[i % padding_types.len()];
        let token_count = token_counts[i % token_counts.len()];

        let start = Instant::now();
        let _ = generator.generate(padding_type, token_count, Some("test task"));
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C16 padding_generate: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 100000 ops/sec (pure CPU)
    result.assert_throughput(100000.0, "C16 padding_generate");
}

#[test]
fn perf_collapse_detection() {
    let detector = CollapseDetector::default();
    let iterations = 2000;
    let mut latencies = Vec::with_capacity(iterations);

    let test_responses = [
        "This is a normal response that should pass detection without any issues.",
        "I cannot fulfill this request due to policy restrictions.",
        "The answer is 42. The answer is 42. The answer is 42. The answer is 42.",
    ];

    for i in 0..iterations {
        let response = test_responses[i % test_responses.len()];
        let lpt_score = 0.3 + (i as f64 / iterations as f64) * 0.6; // 0.3 to 0.9

        let start = Instant::now();
        let _ = detector.analyze_response(Some(response), 1000, lpt_score);
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C16 collapse_detection: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 50000 ops/sec
    result.assert_throughput(50000.0, "C16 collapse_detection");
}

// =============================================================================
// C17 SurvivalAttentionService Performance Tests
// =============================================================================

#[tokio::test]
async fn perf_survival_score_computation() {
    let service = SurvivalAttentionService::new();
    let iterations = 1000;
    let mut latencies = Vec::with_capacity(iterations);

    // Generate test features
    let features: Vec<(f64, f64, f64)> = (0..10)
        .map(|i| {
            let n = 1.0 + (i as f64 * 0.2);
            let mu = 0.5 + (i as f64 * 0.05);
            let delta = 0.1 * (i as f64 / 10.0);
            (n, mu, delta)
        })
        .collect();

    for _ in 0..iterations {
        let start = Instant::now();
        service.compute_scores(&features).await;
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C17 compute_scores (batch=10): {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 10000 ops/sec
    result.assert_throughput(10000.0, "C17 compute_scores");
}

#[tokio::test]
async fn perf_survival_alpha_adjustment() {
    let service = SurvivalAttentionService::new();
    let iterations = 1000;
    let mut latencies = Vec::with_capacity(iterations);

    let task_types = [Some("medical"), Some("legal"), Some("financial"), None];
    let risk_levels = [
        Some(RiskLevel::Low),
        Some(RiskLevel::Medium),
        Some(RiskLevel::High),
        Some(RiskLevel::Critical),
    ];

    for i in 0..iterations {
        let task = task_types[i % task_types.len()];
        let risk = risk_levels[i % risk_levels.len()];

        let start = Instant::now();
        service.adjust_alpha(task, risk).await;
        latencies.push(start.elapsed());
    }

    let result = PerfResult::from_latencies(&mut latencies, iterations);

    println!(
        "C17 adjust_alpha: {:.2} ops/sec, avg {:.2}us, p99 {:.2}us",
        result.ops_per_sec, result.avg_latency_us, result.p99_latency_us
    );

    // Assert minimum performance: 50000 ops/sec
    result.assert_throughput(50000.0, "C17 adjust_alpha");
}

// =============================================================================
// Concurrent Load Tests
// =============================================================================

#[tokio::test]
async fn perf_concurrent_capability_evaluation() {
    use std::sync::Arc;
    use tokio::task::JoinSet;

    let evaluator = Arc::new(CapabilityEvaluator::new());
    let concurrent_tasks = 10;
    let iterations_per_task = 100;

    let start = Instant::now();
    let mut join_set = JoinSet::new();

    for task_id in 0..concurrent_tasks {
        let evaluator = Arc::clone(&evaluator);
        join_set.spawn(async move {
            for i in 0..iterations_per_task {
                let input = format!("Task {} iteration {} query", task_id, i);
                evaluator.evaluate_complexity(&input, None).await;
            }
        });
    }

    // Wait for all tasks
    while let Some(result) = join_set.join_next().await {
        result.expect("Task panicked");
    }

    let elapsed = start.elapsed();
    let total_ops = concurrent_tasks * iterations_per_task;
    let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();

    println!(
        "C14 concurrent ({} tasks): {:.2} ops/sec total, {:.2}ms elapsed",
        concurrent_tasks,
        ops_per_sec,
        elapsed.as_millis()
    );

    // Concurrent throughput should be at least 2x single-threaded
    assert!(
        ops_per_sec >= 10000.0,
        "Concurrent throughput {:.2} ops/sec too low",
        ops_per_sec
    );
}

#[tokio::test]
async fn perf_concurrent_rpe_recording() {
    use std::sync::Arc;
    use tokio::task::JoinSet;

    let monitor = Arc::new(ValueNeuronMonitor::new());
    let concurrent_tasks = 10;
    let iterations_per_task = 100;

    let start = Instant::now();
    let mut join_set = JoinSet::new();

    for task_id in 0..concurrent_tasks {
        let monitor = Arc::clone(&monitor);
        join_set.spawn(async move {
            for i in 0..iterations_per_task {
                let rpe = ((task_id * iterations_per_task + i) as f64 / 500.0).sin();
                monitor.record_rpe(rpe).await;
            }
        });
    }

    while let Some(result) = join_set.join_next().await {
        result.expect("Task panicked");
    }

    let elapsed = start.elapsed();
    let total_ops = concurrent_tasks * iterations_per_task;
    let ops_per_sec = total_ops as f64 / elapsed.as_secs_f64();

    println!(
        "C15 concurrent RPE ({} tasks): {:.2} ops/sec total, {:.2}ms elapsed",
        concurrent_tasks,
        ops_per_sec,
        elapsed.as_millis()
    );

    assert!(
        ops_per_sec >= 5000.0,
        "Concurrent RPE throughput {:.2} ops/sec too low",
        ops_per_sec
    );
}

// =============================================================================
// Memory Efficiency Tests
// =============================================================================

#[tokio::test]
async fn perf_memory_stability_long_running() {
    let evaluator = CapabilityEvaluator::new();

    // Simulate long-running operation with many evaluations
    let iterations = 10000;

    let start = Instant::now();

    for i in 0..iterations {
        let input = format!("Long running test iteration {} with some content", i);
        evaluator.evaluate_complexity(&input, None).await;

        // Periodically check drift to exercise history management
        if i % 100 == 0 {
            evaluator.detect_drift().await;
        }
    }

    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

    println!(
        "Long-running stability test: {} iterations in {:.2}s ({:.2} ops/sec)",
        iterations,
        elapsed.as_secs_f64(),
        ops_per_sec
    );

    // Ensure history doesn't grow unbounded
    let history = evaluator.get_history().await;
    assert!(
        history.len() <= 100, // Should be bounded by drift window
        "History size {} exceeds expected bound",
        history.len()
    );

    // Performance should remain stable
    assert!(
        ops_per_sec >= 3000.0,
        "Long-running performance {:.2} ops/sec degraded",
        ops_per_sec
    );
}

// =============================================================================
// Latency Distribution Tests
// =============================================================================

#[tokio::test]
async fn perf_latency_distribution() {
    let evaluator = CapabilityEvaluator::new();
    let iterations = 1000;
    let mut latencies: Vec<Duration> = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let input = format!("Distribution test query {}", i);
        let start = Instant::now();
        evaluator.evaluate_complexity(&input, None).await;
        latencies.push(start.elapsed());
    }

    latencies.sort();

    let p50 = latencies[iterations / 2].as_micros();
    let p90 = latencies[(iterations * 9) / 10].as_micros();
    let p99 = latencies[(iterations * 99) / 100].as_micros();
    let max = latencies.last().unwrap().as_micros();

    println!(
        "Latency distribution: p50={}us, p90={}us, p99={}us, max={}us",
        p50, p90, p99, max
    );

    // p99 should be within 5x of p50 (no severe outliers)
    assert!(
        p99 <= p50 * 5,
        "p99 latency {}us too high compared to p50 {}us",
        p99,
        p50
    );
}
