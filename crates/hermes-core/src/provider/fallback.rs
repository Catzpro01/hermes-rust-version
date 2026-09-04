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
use tokio_util::sync::CancellationToken;

/// A provider backed by an ordered chain. Index 0 is the primary; the rest are
/// tried in order only after the earlier ones fail before producing a stream.
pub struct FallbackProvider {
    /// Each hop's provider name and its built provider. Names are kept only so
    /// an aggregate [`ProviderError::Fallback`] can report which providers were
    /// attempted. Credentials live inside each hop and never leave it.
    hops: Vec<(String, Box<dyn Provider>)>,
}

impl FallbackProvider {
    /// Builds a chain from one or more providers (index 0 is the primary).
    pub fn new(hops: Vec<(String, Box<dyn Provider>)>) -> Self {
        assert!(
            !hops.is_empty(),
            "FallbackProvider requires at least one provider"
        );
        Self { hops }
    }

    /// Returns the names of every hop, in try order (primary first).
    pub fn provider_names(&self) -> Vec<String> {
        self.hops.iter().map(|(name, _)| name.clone()).collect()
    }

    /// Runs each hop in order until one produces a stream. Returns:
    /// - the first `Ok` stream (the caller then owns consumption),
    /// - `ProviderError::Cancelled` immediately if the token fires before or
    ///   during any hop — cancellation never falls through to a later provider,
    /// - an aggregate `ProviderError::Fallback` naming every hop tried.
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
            match provider.chat_with_cancel(turns, cancel.clone()).await {
                Ok(stream) => return Ok(stream),
                Err(ProviderError::Cancelled) => return Err(ProviderError::Cancelled),
                Err(_) => tried.push(name.clone()),
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
}
