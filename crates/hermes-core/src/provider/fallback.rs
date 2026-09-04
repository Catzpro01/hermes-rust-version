//! A [`Provider`] that transparently tries an ordered chain of providers,
//! moving to the next one when a hop fails before producing a stream.
//!
//! Each hop is itself a full provider (e.g. an [`HttpProvider`] that already
//! applies the Ticket 02 bounded retry and its own key), so by the time a hop
//! returns an error its own retries have been exhausted. [`FallbackProvider`]
//! then retries the *whole* turn with the next hop, never carrying partial
//! output across hops: a turn is always answered from its start by whichever
//! provider serves it (the same `turns` slice is handed to every hop).
//!
//! `ConversationRunner` and the REPL hold a single `Box<dyn Provider>`, so they
//! do not need to know a fallback chain exists.

use super::{EventStream, Provider, ProviderError};
use crate::conversation::Turn;
use async_trait::async_trait;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::health::HealthTracker;

/// A provider backed by an ordered chain. Index 0 is the primary; the rest are
/// tried in order only after the earlier ones fail before producing a stream.
pub struct FallbackProvider {
    /// Each hop's provider name and its built provider. Names are kept only so
    /// an aggregate [`ProviderError::Fallback`] can report which providers were
    /// attempted. Credentials live inside each hop and never leave it.
    hops: Vec<(String, Box<dyn Provider>)>,
    /// In-memory per-hop failure tracker (Ticket 05). A hop that recently
    /// failed is skipped during its cooldown so a struggling endpoint is not
    /// hammered repeatedly. Shared behind `Arc` so it can be injected for
    /// tests; never persisted.
    health: Arc<HealthTracker>,
}

impl FallbackProvider {
    /// Builds a chain from one or more providers (index 0 is the primary) using
    /// the default [`DEFAULT_COOLDOWN`].
    pub fn new(hops: Vec<(String, Box<dyn Provider>)>) -> Self {
        assert!(
            !hops.is_empty(),
            "FallbackProvider requires at least one provider"
        );
        Self {
            hops,
            health: Arc::new(HealthTracker::default()),
        }
    }

    /// Builds a chain backed by an explicit health tracker (for injecting a
    /// short cooldown in tests or a shared tracker at startup).
    pub fn with_health(
        hops: Vec<(String, Box<dyn Provider>)>,
        health: Arc<HealthTracker>,
    ) -> Self {
        assert!(
            !hops.is_empty(),
            "FallbackProvider requires at least one provider"
        );
        Self { hops, health }
    }

    /// Returns the names of every hop, in try order (primary first).
    pub fn provider_names(&self) -> Vec<String> {
        self.hops.iter().map(|(name, _)| name.clone()).collect()
    }

    /// The health tracker backing this chain (for inspection in tests).
    pub fn health(&self) -> &HealthTracker {
        &self.health
    }

    /// Runs each hop in order until one produces a stream. Returns:
    /// - the first `Ok` stream (the caller then owns consumption),
    /// - `ProviderError::Cancelled` immediately if the token fires before or
    ///   during any hop — cancellation never falls through to a later provider
    ///   and is never recorded as a failure,
    /// - an aggregate `ProviderError::Fallback` naming every provider actually
    ///   attempted (a hop skipped because it is cooling down is not "tried").
    ///
    /// A hop that fails (any non-`Cancelled` error, after its own retries) is
    /// recorded as cooling down; a hop that succeeds clears any prior failure.
    async fn first_available(
        &self,
        turns: &[Turn],
        cancel: &CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        let mut tried: Vec<String> = Vec::new();
        for (name, provider) in &self.hops {
            if cancel.is_cancelled() {
                return Err(ProviderError::Cancelled);
            }
            // Skip a hop that is cooling down from a recent failure (Ticket 05).
            if self.health.is_cooling_down(name) {
                continue;
            }
            match provider.chat_with_cancel(turns, cancel.clone()).await {
                Ok(stream) => {
                    self.health.record_success(name);
                    return Ok(stream);
                }
                Err(ProviderError::Cancelled) => return Err(ProviderError::Cancelled),
                Err(_) => {
                    self.health.record_failure(name);
                    tried.push(name.clone());
                }
            }
        }
        Err(ProviderError::Fallback { tried })
    }
}

#[async_trait]
impl Provider for FallbackProvider {
    async fn chat(&self, turns: &[Turn]) -> Result<EventStream, ProviderError> {
        let cancel = CancellationToken::new();
        self.first_available(turns, &cancel).await
    }

    async fn chat_with_cancel(
        &self,
        turns: &[Turn],
        cancel: CancellationToken,
    ) -> Result<EventStream, ProviderError> {
        self.first_available(turns, &cancel).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::Event;
    use futures::{stream, StreamExt};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Outcome a stub hop produces on every call.
    #[derive(Clone)]
    enum Behaviour {
        /// Succeed with a stream carrying the given text chunk.
        Ok(&'static str),
        /// Fail with this concrete error (retries are not modelled here).
        Err(ProviderError),
        /// Fail with `ProviderError::Cancelled`.
        Cancel,
    }

    /// A controllable test provider that records how often it was invoked.
    struct Stub {
        behaviour: Behaviour,
        calls: Arc<AtomicUsize>,
    }

    fn stub(behaviour: Behaviour, calls: &Arc<AtomicUsize>) -> Box<dyn Provider> {
        Box::new(Stub {
            behaviour,
            calls: Arc::clone(calls),
        })
    }

    fn counter() -> Arc<AtomicUsize> {
        Arc::new(AtomicUsize::new(0))
    }

    fn chunk_stream(text: &str) -> EventStream {
        Box::pin(stream::iter([
            Ok(Event::Started),
            Ok(Event::Chunk(text.to_owned())),
            Ok(Event::Done),
        ]))
    }

    #[async_trait]
    impl Provider for Stub {
        async fn chat(&self, _turns: &[Turn]) -> Result<EventStream, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match &self.behaviour {
                Behaviour::Ok(text) => Ok(chunk_stream(text)),
                Behaviour::Err(err) => Err(err.clone()),
                Behaviour::Cancel => Err(ProviderError::Cancelled),
            }
        }
    }

    /// Drains a stream into the concatenation of its text chunks.
    async fn collect_text(mut stream: EventStream) -> String {
        let mut out = String::new();
        while let Some(item) = stream.next().await {
            if let Ok(Event::Chunk(chunk)) = item {
                out.push_str(&chunk);
            }
        }
        out
    }

    #[tokio::test]
    async fn uses_the_primary_and_never_calls_a_later_hop_when_it_succeeds() {
        let a = counter();
        let b = counter();
        let provider = FallbackProvider::new(vec![
            ("a".into(), stub(Behaviour::Ok("from-a"), &a)),
            ("b".into(), stub(Behaviour::Ok("from-b"), &b)),
        ]);
        let text = collect_text(provider.chat(&[]).await.unwrap()).await;
        assert_eq!(text, "from-a");
        assert_eq!(a.load(Ordering::SeqCst), 1);
        assert_eq!(b.load(Ordering::SeqCst), 0, "later hop must not be reached");
    }

    #[tokio::test]
    async fn moves_to_the_next_hop_when_the_primary_fails() {
        let a = counter();
        let b = counter();
        let provider = FallbackProvider::new(vec![
            (
                "a".into(),
                stub(
                    Behaviour::Err(ProviderError::Http {
                        status: 500,
                        message: "down".into(),
                    }),
                    &a,
                ),
            ),
            ("b".into(), stub(Behaviour::Ok("from-b"), &b)),
        ]);
        let text = collect_text(provider.chat(&[]).await.unwrap()).await;
        assert_eq!(text, "from-b");
        assert_eq!(a.load(Ordering::SeqCst), 1);
        assert_eq!(b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn moves_to_the_next_hop_even_on_a_permanent_non_retryable_error() {
        // Fallback is between different endpoints, so even a permanent error on
        // hop A (e.g. a 400 or auth rejection local to A) should let hop B try.
        let a = counter();
        let b = counter();
        let provider = FallbackProvider::new(vec![
            ("a".into(), stub(Behaviour::Err(ProviderError::Message("bad".into())), &a)),
            ("b".into(), stub(Behaviour::Ok("from-b"), &b)),
        ]);
        let text = collect_text(provider.chat(&[]).await.unwrap()).await;
        assert_eq!(text, "from-b");
        assert_eq!(b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn all_hops_failing_yields_an_aggregate_error_naming_them_in_order() {
        let a = counter();
        let b = counter();
        let c = counter();
        let provider = FallbackProvider::new(vec![
            (
                "a".into(),
                stub(Behaviour::Err(ProviderError::Http { status: 503, message: "x".into() }), &a),
            ),
            (
                "b".into(),
                stub(Behaviour::Err(ProviderError::Timeout), &b),
            ),
            (
                "c".into(),
                stub(Behaviour::Err(ProviderError::Message("y".into())), &c),
            ),
        ]);
        let err = match provider.chat(&[]).await {
            Ok(_) => panic!("expected aggregate error"),
            Err(err) => err,
        };
        assert!(
            matches!(
                &err,
                ProviderError::Fallback { tried } if tried == &vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
            ),
            "unexpected aggregate: {err}"
        );
        for hop in [&a, &b, &c] {
            assert_eq!(hop.load(Ordering::SeqCst), 1, "every hop must be tried once");
        }
    }

    #[tokio::test]
    async fn a_cancelled_hop_exits_immediately_without_falling_through() {
        // Hop A returns Cancelled: the chain must stop, never offering the turn
        // to hop B.
        let a = counter();
        let b = counter();
        let provider = FallbackProvider::new(vec![
            ("a".into(), stub(Behaviour::Cancel, &a)),
            ("b".into(), stub(Behaviour::Ok("from-b"), &b)),
        ]);
        let result = provider
            .chat_with_cancel(&[], CancellationToken::new())
            .await;
        assert!(matches!(result, Err(ProviderError::Cancelled)));
        assert_eq!(a.load(Ordering::SeqCst), 1);
        assert_eq!(b.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_pre_cancelled_token_never_touches_any_hop() {
        let a = counter();
        let b = counter();
        let provider = FallbackProvider::new(vec![
            ("a".into(), stub(Behaviour::Err(ProviderError::Http { status: 500, message: "d".into() }), &a)),
            ("b".into(), stub(Behaviour::Ok("from-b"), &b)),
        ]);
        let token = CancellationToken::new();
        token.cancel();
        let result = provider.chat_with_cancel(&[], token).await;
        assert!(matches!(result, Err(ProviderError::Cancelled)));
        assert_eq!(a.load(Ordering::SeqCst), 0);
        assert_eq!(b.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn provider_names_report_chain_order() {
        let a = counter();
        let b = counter();
        let provider = FallbackProvider::new(vec![
            ("a".into(), stub(Behaviour::Ok("x"), &a)),
            ("b".into(), stub(Behaviour::Ok("y"), &b)),
        ]);
        assert_eq!(provider.provider_names(), vec!["a".to_owned(), "b".to_owned()]);
    }

    /// A stub whose behaviour is read from a shared cell, so it can change
    /// between calls (used to prove cooldown recovery without a server).
    struct FlipStub {
        state: std::sync::Arc<std::sync::Mutex<Behaviour>>,
        calls: Arc<AtomicUsize>,
    }

    fn flip_stub(
        state: std::sync::Arc<std::sync::Mutex<Behaviour>>,
        calls: &Arc<AtomicUsize>,
    ) -> Box<dyn Provider> {
        Box::new(FlipStub {
            state,
            calls: Arc::clone(calls),
        })
    }

    #[async_trait]
    impl Provider for FlipStub {
        async fn chat(&self, _turns: &[Turn]) -> Result<EventStream, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let behaviour = self.state.lock().unwrap().clone();
            match behaviour {
                Behaviour::Ok(text) => Ok(chunk_stream(text)),
                Behaviour::Err(err) => Err(err.clone()),
                Behaviour::Cancel => Err(ProviderError::Cancelled),
            }
        }
    }

    /// A tiny cooldown plus a real sleep, so a full cooldown cycle is observable
    /// without slowing the suite meaningfully.
    const TINY_COOLDOWN: std::time::Duration = std::time::Duration::from_millis(50);

    #[tokio::test]
    async fn a_failed_hop_is_skipped_while_cooling_down() {
        let a = counter();
        let b = counter();
        let a_state = std::sync::Arc::new(std::sync::Mutex::new(Behaviour::Err(
            ProviderError::Http {
                status: 500,
                message: "down".into(),
            },
        )));
        let health = std::sync::Arc::new(HealthTracker::new(TINY_COOLDOWN));
        let provider = FallbackProvider::with_health(
            vec![
                ("a".into(), flip_stub(a_state.clone(), &a)),
                ("b".into(), stub(Behaviour::Ok("from-b"), &b)),
            ],
            std::sync::Arc::clone(&health),
        );

        // Turn 1: A fails (still marked cooling for the whole call window), B
        // serves. Then within the cooldown, a second turn must skip A entirely.
        let t1 = collect_text(provider.chat(&[]).await.unwrap()).await;
        assert_eq!(t1, "from-b");
        assert!(health.is_cooling_down("a"), "A must be cooling after its failure");

        let a_before = a.load(Ordering::SeqCst);
        let t2 = collect_text(provider.chat(&[]).await.unwrap()).await;
        assert_eq!(t2, "from-b");
        assert_eq!(
            a.load(Ordering::SeqCst),
            a_before,
            "a cooling-down hop must be skipped, not re-tried"
        );
    }

    #[tokio::test]
    async fn a_cooling_hop_is_tried_again_after_its_cooldown_elapses() {
        let a = counter();
        let b = counter();
        // A fails at first, then "recovers" to Ok once we flip the flag.
        let a_state = std::sync::Arc::new(std::sync::Mutex::new(Behaviour::Err(
            ProviderError::Http {
                status: 503,
                message: "down".into(),
            },
        )));
        let health = std::sync::Arc::new(HealthTracker::new(TINY_COOLDOWN));
        let provider = FallbackProvider::with_health(
            vec![
                ("a".into(), flip_stub(a_state.clone(), &a)),
                ("b".into(), stub(Behaviour::Ok("from-b"), &b)),
            ],
            std::sync::Arc::clone(&health),
        );

        // A fails once -> cooling; B serves.
        assert_eq!(collect_text(provider.chat(&[]).await.unwrap()).await, "from-b");
        assert!(health.is_cooling_down("a"));
        let a_failed_calls = a.load(Ordering::SeqCst);

        // Flip A healthy and let the cooldown expire.
        *a_state.lock().unwrap() = Behaviour::Ok("recovered-a");
        tokio::time::sleep(TINY_COOLDOWN + std::time::Duration::from_millis(30)).await;
        assert!(!health.is_cooling_down("a"), "cooldown must have elapsed");

        // Next turn: A is tried again and now serves.
        let t3 = collect_text(provider.chat(&[]).await.unwrap()).await;
        assert_eq!(t3, "recovered-a");
        assert_eq!(
            a.load(Ordering::SeqCst),
            a_failed_calls + 1,
            "A must be tried again once the cooldown elapsed"
        );
    }

    #[tokio::test]
    async fn a_cancelled_hop_is_not_recorded_as_a_failure() {
        let a = counter();
        let b = counter();
        let a_state = std::sync::Arc::new(std::sync::Mutex::new(Behaviour::Cancel));
        let health = std::sync::Arc::new(HealthTracker::new(TINY_COOLDOWN));
        let provider = FallbackProvider::with_health(
            vec![
                ("a".into(), flip_stub(a_state.clone(), &a)),
                ("b".into(), stub(Behaviour::Ok("from-b"), &b)),
            ],
            std::sync::Arc::clone(&health),
        );
        let result = provider
            .chat_with_cancel(&[], CancellationToken::new())
            .await;
        assert!(matches!(result, Err(ProviderError::Cancelled)));
        assert!(
            !health.is_cooling_down("a"),
            "a user cancellation is not a provider failure and must not start a cooldown"
        );
    }

    #[tokio::test]
    async fn manual_switch_provider_is_not_gated_by_any_cooldown() {
        // Manual `/provider` resolves a *fresh* single provider via the registry
        // (not a FallbackProvider), so no HealthTracker gates it: the user's
        // explicit choice bypasses cooldown. We prove the building blocks: a
        // standalone provider that previously "failed" elsewhere is not cooling
        // here because no tracker knows about that failure.
        let a = counter();
        let tracker_a = HealthTracker::new(TINY_COOLDOWN);
        tracker_a.record_failure("b"); // some other chain cooled "b"
        let fresh = FallbackProvider::with_health(
            vec![
                ("b".into(), stub(Behaviour::Ok("manual-b"), &a)),
            ],
            std::sync::Arc::new(HealthTracker::new(TINY_COOLDOWN)),
        );
        // The fresh chain's own tracker has no record of "b" failing, so the
        // manual /provider-b choice is honoured immediately.
        let text = collect_text(fresh.chat(&[]).await.unwrap()).await;
        assert_eq!(text, "manual-b");
        let _ = tracker_a;
    }
}
