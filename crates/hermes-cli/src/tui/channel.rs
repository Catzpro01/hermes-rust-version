//! Spec 012 — worker→renderer event queue (Warning B: bounded, drop-oldest).
//!
//! The agentic worker pushes display events via [`EventQueue`]; the renderer
//! drains them each frame. The queue is **bounded** (capacity 256 by default)
//! and implements a **drop-oldest** policy: when full, the oldest event is
//! discarded to make room for the newest. This is a deliberate, documented
//! trade-off (Matt, Warning B): for live TUI display a dropped stale frame is
//! acceptable, but a worker blocked on a full channel would deadlock the turn.
//! The producer therefore **never blocks**.
//!
//! Renderer→worker traffic (user input) travels the other way over an
//! unbounded [`tokio::sync::mpsc`] sender of [`TuiCommand`]; user keystrokes are
//! low-rate and must not be dropped, so they are never lossy.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::event::TuiEvent;

/// Default capacity of the worker→renderer event queue.
pub const DEFAULT_QUEUE_CAPACITY: usize = 256;

/// A bounded, shared, drop-oldest queue of display events. Never blocks.
pub struct EventQueue {
    inner: Mutex<VecDeque<TuiEvent>>,
    cap: usize,
}

impl EventQueue {
    /// Creates a queue with the given capacity (>= 1).
    pub fn new(cap: usize) -> Self {
        EventQueue {
            inner: Mutex::new(VecDeque::with_capacity(cap.max(1))),
            cap: cap.max(1),
        }
    }

    /// Pushes an event. If the queue is full the **oldest** event is dropped so
    /// the newest (freshest display state) always wins. Never blocks.
    pub fn push(&self, event: TuiEvent) {
        let mut inner = self.inner.lock().unwrap();
        if inner.len() >= self.cap {
            inner.pop_front(); // drop-oldest
        }
        inner.push_back(event);
    }

    /// Removes and returns all pending events in FIFO order.
    pub fn drain(&self) -> Vec<TuiEvent> {
        let mut inner = self.inner.lock().unwrap();
        inner.drain(..).collect()
    }

    /// Number of events currently buffered.
    #[allow(dead_code)] // asserted by tests; the renderer drains rather than queries
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// Whether the queue is currently empty.
    #[allow(dead_code)] // asserted by tests
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A user-issued command sent from the renderer to the worker. The worker keeps
/// reading until the channel is closed (all renderer senders dropped on quit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiCommand {
    /// A full line of user input (submitted with Enter).
    Line(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_oldest_when_full_keeps_newest() {
        let q = EventQueue::new(3);
        for i in 0..5 {
            q.push(TuiEvent::Iteration(i));
        }
        // 0 and 1 dropped to make room for 3 and 4.
        assert_eq!(
            q.drain(),
            vec![
                TuiEvent::Iteration(2),
                TuiEvent::Iteration(3),
                TuiEvent::Iteration(4),
            ]
        );
        assert!(q.is_empty());
    }

    #[test]
    fn queue_bounds_at_capacity() {
        let q = EventQueue::new(2);
        q.push(TuiEvent::Notice("a".into()));
        q.push(TuiEvent::Notice("b".into()));
        q.push(TuiEvent::Notice("c".into())); // drops "a"
        assert_eq!(q.len(), 2);
        assert_eq!(
            q.drain(),
            vec![TuiEvent::Notice("b".into()), TuiEvent::Notice("c".into())]
        );
    }

    #[test]
    fn capacity_is_at_least_one() {
        let q = EventQueue::new(0);
        q.push(TuiEvent::Notice("x".into()));
        assert_eq!(q.len(), 1);
    }
}
