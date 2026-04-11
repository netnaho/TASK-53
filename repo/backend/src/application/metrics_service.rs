/// In-process metrics service with a sliding 10-minute error-rate window.
///
/// Thread-safe via Arc<Mutex> so it can be cloned and shared across the
/// TracingFairing (which runs outside the normal Rocket State system) as
/// well as route handlers.
///
/// Design:
///   - Records each request outcome (success vs error) with a timestamp.
///   - Prunes observations older than 10 minutes on every read.
///   - No external dependencies; purely in-process.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A single request observation in the sliding window.
#[derive(Debug, Clone)]
struct Observation {
    recorded_at: Instant,
    is_error: bool,
}

/// Thread-safe metrics state shared across the application.
#[derive(Debug)]
struct MetricsInner {
    window: VecDeque<Observation>,
    total_requests: u64,
    total_errors: u64,
    window_duration: Duration,
}

impl MetricsInner {
    fn new(window_duration: Duration) -> Self {
        Self {
            window: VecDeque::with_capacity(1024),
            total_requests: 0,
            total_errors: 0,
            window_duration,
        }
    }

    /// Remove observations older than the window duration.
    fn prune(&mut self) {
        let cutoff = Instant::now() - self.window_duration;
        while let Some(front) = self.window.front() {
            if front.recorded_at < cutoff {
                self.window.pop_front();
            } else {
                break;
            }
        }
    }

    /// Error rate within the sliding window (0.0 – 1.0).
    fn window_error_rate(&mut self) -> f64 {
        self.prune();
        if self.window.is_empty() {
            return 0.0;
        }
        let errors = self.window.iter().filter(|o| o.is_error).count();
        errors as f64 / self.window.len() as f64
    }

    /// Count of requests in the current window.
    fn window_request_count(&mut self) -> usize {
        self.prune();
        self.window.len()
    }

    /// Count of errors in the current window.
    fn window_error_count(&mut self) -> usize {
        self.prune();
        self.window.iter().filter(|o| o.is_error).count()
    }
}

/// Cloneable handle to the metrics service.
#[derive(Clone, Debug)]
pub struct MetricsService {
    inner: Arc<Mutex<MetricsInner>>,
}

impl MetricsService {
    /// Create a service with the default 10-minute error-rate window.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MetricsInner::new(Duration::from_secs(600)))),
        }
    }

    /// Create a service with a custom window (useful for tests).
    pub fn with_window(window: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MetricsInner::new(window))),
        }
    }

    /// Record a request outcome.
    /// `is_error` = true for HTTP 5xx; false for all others (including 4xx — those are
    /// client errors, not service errors, so they should NOT inflate the error rate).
    pub fn record(&self, is_error: bool) {
        if let Ok(mut m) = self.inner.lock() {
            m.total_requests += 1;
            if is_error {
                m.total_errors += 1;
            }
            m.window.push_back(Observation {
                recorded_at: Instant::now(),
                is_error,
            });
        }
    }

    /// Current error rate within the sliding window.
    pub fn window_error_rate(&self) -> f64 {
        if let Ok(mut m) = self.inner.lock() {
            m.window_error_rate()
        } else {
            0.0
        }
    }

    /// Total lifetime request count.
    pub fn total_requests(&self) -> u64 {
        if let Ok(m) = self.inner.lock() { m.total_requests } else { 0 }
    }

    /// Total lifetime error count.
    pub fn total_errors(&self) -> u64 {
        if let Ok(m) = self.inner.lock() { m.total_errors } else { 0 }
    }

    /// Requests in the current window.
    pub fn window_request_count(&self) -> usize {
        if let Ok(mut m) = self.inner.lock() { m.window_request_count() } else { 0 }
    }

    /// Errors in the current window.
    pub fn window_error_count(&self) -> usize {
        if let Ok(mut m) = self.inner.lock() { m.window_error_count() } else { 0 }
    }
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_metrics_zero_error_rate() {
        let svc = MetricsService::new();
        assert_eq!(svc.window_error_rate(), 0.0);
        assert_eq!(svc.total_requests(), 0);
    }

    #[test]
    fn test_all_success_zero_error_rate() {
        let svc = MetricsService::new();
        for _ in 0..10 {
            svc.record(false);
        }
        assert_eq!(svc.window_error_rate(), 0.0);
        assert_eq!(svc.total_requests(), 10);
    }

    #[test]
    fn test_partial_errors_rate() {
        let svc = MetricsService::new();
        for _ in 0..8 { svc.record(false); }
        for _ in 0..2 { svc.record(true); }
        // 2 errors out of 10 = 20%
        let rate = svc.window_error_rate();
        assert!((rate - 0.20).abs() < 0.001, "expected ~0.20 got {rate}");
    }

    #[test]
    fn test_window_expiry() {
        // Use a 100ms window so we can test expiry quickly
        let svc = MetricsService::with_window(Duration::from_millis(100));
        for _ in 0..5 { svc.record(true); }
        assert_eq!(svc.window_error_count(), 5);

        // Wait for window to expire
        std::thread::sleep(Duration::from_millis(150));
        svc.record(false); // trigger a prune
        assert_eq!(svc.window_error_count(), 0, "old errors should have expired");
    }

    #[test]
    fn test_5xx_increments_total_errors() {
        let svc = MetricsService::new();
        svc.record(true);
        svc.record(false);
        assert_eq!(svc.total_errors(), 1);
        assert_eq!(svc.total_requests(), 2);
    }
}
