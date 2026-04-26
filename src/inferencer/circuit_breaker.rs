use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub open_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            open_duration: Duration::from_secs(30),
        }
    }
}

struct InnerState {
    state: CircuitState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    opened_at: Option<Instant>,
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    inner: Mutex<InnerState>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(InnerState {
                state: CircuitState::Closed,
                consecutive_failures: 0,
                consecutive_successes: 0,
                opened_at: None,
            }),
        }
    }

    pub fn allow_request(&self) -> bool {
        let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");
        self.maybe_transition_to_half_open(&mut inner);

        match inner.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");

        inner.consecutive_failures = 0;
        inner.consecutive_successes += 1;

        if inner.state == CircuitState::HalfOpen
            && inner.consecutive_successes >= self.config.success_threshold
        {
            tracing::info!("circuit breaker closed after successful probes");
            inner.state = CircuitState::Closed;
            inner.consecutive_successes = 0;
            inner.opened_at = None;
        }
    }

    pub fn record_failure(&self) {
        let mut inner = self.inner.lock().expect("circuit breaker lock poisoned");

        inner.consecutive_successes = 0;
        inner.consecutive_failures += 1;

        match inner.state {
            CircuitState::Closed => {
                if inner.consecutive_failures >= self.config.failure_threshold {
                    tracing::warn!(
                        failures = inner.consecutive_failures,
                        "circuit breaker opened due to consecutive failures"
                    );
                    inner.state = CircuitState::Open;
                    inner.opened_at = Some(Instant::now());
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!("circuit breaker reopened after half-open probe failed");
                inner.state = CircuitState::Open;
                inner.opened_at = Some(Instant::now());
                inner.consecutive_failures = 0;
            }
            CircuitState::Open => {}
        }
    }

    fn maybe_transition_to_half_open(&self, inner: &mut InnerState) {
        if inner.state == CircuitState::Open
            && inner
                .opened_at
                .is_some_and(|opened_at| opened_at.elapsed() >= self.config.open_duration)
        {
            tracing::info!("circuit breaker transitioning to half-open");
            inner.state = CircuitState::HalfOpen;
            inner.consecutive_successes = 0;
            inner.consecutive_failures = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_allowing_requests() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert!(cb.allow_request());
    }

    #[test]
    fn opens_after_threshold_failures() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        });

        cb.record_failure();
        cb.record_failure();
        assert!(cb.allow_request(), "should allow before threshold");

        cb.record_failure();
        assert!(!cb.allow_request(), "should block after threshold");
    }

    #[test]
    fn success_resets_failure_count() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        });

        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        cb.record_failure();
        cb.record_failure();

        assert!(cb.allow_request(), "success should reset failure count");
    }

    #[test]
    fn allows_request_after_open_duration() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_millis(10),
            ..Default::default()
        });

        cb.record_failure();
        assert!(!cb.allow_request(), "should be open after failure");

        std::thread::sleep(Duration::from_millis(15));
        assert!(
            cb.allow_request(),
            "should allow after open_duration (half-open)"
        );
    }

    #[test]
    fn closes_after_success_threshold_in_half_open() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            open_duration: Duration::from_millis(1),
        });

        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(!cb.allow_request(), "should be open");

        std::thread::sleep(Duration::from_millis(5));

        assert!(cb.allow_request(), "should be half-open");
        cb.record_success();
        cb.record_success();

        // Now closed, single failure should not reopen
        cb.record_failure();
        assert!(
            cb.allow_request(),
            "should be closed, single failure not enough"
        );
    }

    #[test]
    fn reopens_on_failure_in_half_open() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 2,
            open_duration: Duration::from_millis(1),
        });

        cb.record_failure();
        std::thread::sleep(Duration::from_millis(5));

        assert!(cb.allow_request(), "should be half-open");
        cb.record_failure();
        assert!(!cb.allow_request(), "should reopen on half-open failure");
    }
}
