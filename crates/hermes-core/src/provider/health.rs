//! In-memory per-provider failure tracking (lightweight circuit breaker).
//!
//! After a hop exhausts its retries ([`crate::provider::HttpProvider`] Ticket 02)
//! we mark that provider "cooling down" for a bounded [`Duration`]. A
//! [`FallbackProvider`] then skips a cooling-down hop so a recently-failing
//! endpoint is not hammered with repeated requests in one session.
//!
//! State lives only for the process lifetime — it is **never** persisted to
//! `state.db` (which stays the sole canonical store). All access goes through a
//! `std::sync::Mutex`, so the tracker is `Send + Sync`; it is consulted only on
//! hop failure/skip, so it is not a hot-path bottleneck.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Default time a provider stays out of rotation after a failure.
pub const DEFAULT_COOLDOWN: Duration = Duration::from_secs(60);

/// Tracks which providers are currently cooling down after recent failures.
///
/// Failures are recorded by *provider name* (never by credential or value), and
/// a hop re-enters rotation once its cooldown elapses — or immediately if it
/// later succeeds. An explicit `Cancelled` is never recorded as a failure.
pub struct HealthTracker {
    cooldown: Duration,
    /// provider name -> instant it was marked failing. Entries are removed when
    /// the cooldown expires or the provider succeeds.
    cooling: Mutex<HashMap<String, Instant>>,
}

impl HealthTracker {
    /// Builds a tracker with the given cooldown (bounded: any value >= 0).
    pub fn new(cooldown: Duration) -> Self {
        Self {
            cooldown,
            cooling: Mutex::new(HashMap::new()),
        }
    }

    /// The configured cooldown duration.
    pub fn cooldown(&self) -> Duration {
        self.cooldown
    }

    /// Marks `name` as failing *now*, starting its cooldown window.
    pub fn record_failure(&self, name: &str) {
        self.record_failure_at(name, Instant::now());
    }

    fn record_failure_at(&self, name: &str, now: Instant) {
        if let Ok(mut map) = self.cooling.lock() {
            map.insert(name.to_owned(), now);
        }
    }

    /// Clears any cooldown for `name` (e.g. after it succeeds), so a recovered
    /// provider is immediately eligible again.
    pub fn record_success(&self, name: &str) {
        if let Ok(mut map) = self.cooling.lock() {
            map.remove(name);
        }
    }

    /// Whether `name` is still inside its cooldown window right now.
    pub fn is_cooling_down(&self, name: &str) -> bool {
        self.is_cooling_down_at(name, Instant::now())
    }

    fn is_cooling_down_at(&self, name: &str, now: Instant) -> bool {
        let map = match self.cooling.lock() {
            Ok(map) => map,
            Err(_) => return false,
        };
        match map.get(name) {
            Some(since) => now.saturating_duration_since(*since) < self.cooldown,
            None => false,
        }
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new(DEFAULT_COOLDOWN)
    }
}

impl std::fmt::Debug for HealthTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<String> = self
            .cooling
            .lock()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        f.debug_struct("HealthTracker")
            .field("cooldown", &self.cooldown)
            .field("cooling", &names)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A pseudo "now" far enough in the future to be strictly past any cooldown
    // produced in tests. We drive comparisons with explicit instants so the
    // time-based logic is tested without sleeping.
    fn base() -> Instant {
        Instant::now()
    }

    #[test]
    fn default_cooldown_is_documented_and_bounded() {
        assert_eq!(HealthTracker::default().cooldown(), DEFAULT_COOLDOWN);
        assert_eq!(DEFAULT_COOLDOWN, Duration::from_secs(60));
    }

    #[test]
    fn fresh_provider_is_not_cooling() {
        let tracker = HealthTracker::new(Duration::from_secs(10));
        assert!(!tracker.is_cooling_down_at("a", base()));
    }

    #[test]
    fn failure_marks_cooling_until_cooldown_elapses() {
        let tracker = HealthTracker::new(Duration::from_secs(10));
        let t0 = base();
        tracker.record_failure_at("a", t0);
        assert!(tracker.is_cooling_down_at("a", t0 + Duration::from_millis(500)));
        // Just before the window closes it is still cooling...
        assert!(tracker.is_cooling_down_at("a", t0 + Duration::from_secs(9)));
        // ...and once the window passes it is eligible again.
        assert!(!tracker.is_cooling_down_at("a", t0 + Duration::from_secs(10)));
    }

    #[test]
    fn success_clears_cooldown_immediately() {
        let tracker = HealthTracker::new(Duration::from_secs(60));
        let t0 = base();
        tracker.record_failure_at("a", t0);
        assert!(tracker.is_cooling_down_at("a", t0));
        tracker.record_success("a");
        assert!(!tracker.is_cooling_down_at("a", t0));
    }

    #[test]
    fn providers_are_tracked_independently() {
        let tracker = HealthTracker::new(Duration::from_secs(10));
        let t0 = base();
        tracker.record_failure_at("a", t0);
        assert!(tracker.is_cooling_down_at("a", t0));
        // B was never marked, so it is not cooling.
        assert!(!tracker.is_cooling_down_at("b", t0));
    }
}
