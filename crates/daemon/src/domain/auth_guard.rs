use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct AuthPolicy {
    pub max_failures_before_lockout: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub lockout_seconds: u64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AuthDecision {
    Allowed,
    RetryAfter(Duration),
    LockedOut(Duration),
}

pub struct AuthGuard {
    policy: AuthPolicy,
    failed_attempts: u32,
    backoff_until: Option<Instant>,
    lockout_until: Option<Instant>,
}

impl AuthGuard {
    pub fn new(policy: AuthPolicy) -> Self {
        Self {
            policy,
            failed_attempts: 0,
            backoff_until: None,
            lockout_until: None,
        }
    }

    pub fn precheck(&mut self) -> AuthDecision {
        self.precheck_at(Instant::now())
    }

    pub fn record_success(&mut self) {
        self.failed_attempts = 0;
        self.backoff_until = None;
        self.lockout_until = None;
    }

    pub fn record_failure(&mut self) -> AuthDecision {
        self.record_failure_at(Instant::now())
    }

    fn precheck_at(&mut self, now: Instant) -> AuthDecision {
        self.clear_expired(now);

        if let Some(until) = self.lockout_until {
            return AuthDecision::LockedOut(until.saturating_duration_since(now));
        }

        if let Some(until) = self.backoff_until {
            return AuthDecision::RetryAfter(until.saturating_duration_since(now));
        }

        AuthDecision::Allowed
    }

    fn record_failure_at(&mut self, now: Instant) -> AuthDecision {
        self.clear_expired(now);

        if let Some(until) = self.lockout_until {
            return AuthDecision::LockedOut(until.saturating_duration_since(now));
        }

        self.failed_attempts = self.failed_attempts.saturating_add(1);
        if self.failed_attempts >= self.policy.max_failures_before_lockout {
            let lockout = Duration::from_secs(self.policy.lockout_seconds.max(1));
            self.lockout_until = Some(now + lockout);
            self.backoff_until = None;
            self.failed_attempts = 0;
            return AuthDecision::LockedOut(lockout);
        }

        let exponent = self.failed_attempts.saturating_sub(1);
        let multiplier = 1_u64.checked_shl(exponent.min(20)).unwrap_or(u64::MAX);
        let backoff_ms = self
            .policy
            .initial_backoff_ms
            .max(1)
            .saturating_mul(multiplier)
            .min(self.policy.max_backoff_ms.max(1));
        let backoff = Duration::from_millis(backoff_ms);
        self.backoff_until = Some(now + backoff);
        AuthDecision::RetryAfter(backoff)
    }

    fn clear_expired(&mut self, now: Instant) {
        if let Some(until) = self.lockout_until
            && now >= until
        {
            self.lockout_until = None;
            self.failed_attempts = 0;
        }

        if let Some(until) = self.backoff_until
            && now >= until
        {
            self.backoff_until = None;
        }
    }
}
