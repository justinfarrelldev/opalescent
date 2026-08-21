//! Sticky generation-scoped cancellation support for wait sets.

extern crate alloc;
extern crate std;

use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use super::{WaitSetInner, lock_or_recover, next_event_sequence};

/// Monotonic identity source for cancellation generations.
static NEXT_CANCELLATION_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Return the next cancellation generation.
fn next_cancellation_generation() -> u64 {
    NEXT_CANCELLATION_GENERATION.fetch_add(1, Ordering::Relaxed)
}

/// Affine cancellation request authority.
pub struct CancellationSource {
    /// Shared cancellation generation and wake state.
    inner: Arc<CancellationInner>,
}

impl CancellationSource {
    /// Create a new sticky cancellation generation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancellationInner {
                generation: next_cancellation_generation(),
                state: Mutex::new(CancellationState::default()),
            }),
        }
    }

    /// Return an immutable token observing this exact cancellation generation.
    #[must_use]
    pub fn token(&self) -> CancellationToken {
        CancellationToken {
            inner: Arc::clone(&self.inner),
            generation: self.inner.generation,
        }
    }

    /// Request cancellation for this generation.
    pub fn request(&mut self) {
        let watchers = {
            let mut state = lock_or_recover(&self.inner.state);
            if state.request_sequence.is_some() {
                return;
            }
            state.request_sequence = Some(next_event_sequence());
            state
                .watchers
                .retain(|watcher| watcher.wait_set.upgrade().is_some());
            let watchers = state
                .watchers
                .iter()
                .filter_map(|watcher| watcher.wait_set.upgrade())
                .collect::<Vec<_>>();
            drop(state);
            watchers
        };
        for wait_set in watchers {
            if wait_set.synchronize_cancellation_request() {
                wait_set.ready_changed.notify_all();
            }
        }
    }

    /// Return this cancellation source generation.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc deref in this accessor is not accepted as const on the current toolchain"
    )]
    pub fn generation(&self) -> u64 {
        self.inner.generation
    }
}

impl Default for CancellationSource {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for CancellationSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancellationSource").finish_non_exhaustive()
    }
}

/// Immutable cancellation observation token.
#[derive(Clone)]
pub struct CancellationToken {
    /// Shared cancellation generation and wake state.
    inner: Arc<CancellationInner>,
    /// Generation observed by this token.
    generation: u64,
}

impl CancellationToken {
    /// Return whether cancellation has been requested for this token generation.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.request_sequence().is_some()
    }

    /// Return the token's cancellation generation.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Return the cancellation request sequence if requested.
    pub(super) fn request_sequence(&self) -> Option<u64> {
        if self.generation != self.inner.generation {
            return None;
        }
        lock_or_recover(&self.inner.state).request_sequence
    }

    /// Register `wait_set` to be woken when this token is requested.
    pub(super) fn watch_wait_set(&self, wait_set: &Arc<WaitSetInner>) -> CancellationWaitGuard {
        let mut state = lock_or_recover(&self.inner.state);
        if state.request_sequence.is_some() {
            return CancellationWaitGuard::empty();
        }
        let watcher_id = state.next_watcher_id;
        state.next_watcher_id = state.next_watcher_id.saturating_add(1);
        state.watchers.push(CancellationWatcher {
            id: watcher_id,
            wait_set: Arc::downgrade(wait_set),
        });
        drop(state);
        CancellationWaitGuard {
            cancellation: Some(Arc::downgrade(&self.inner)),
            watcher_id,
        }
    }
}

impl fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancellationToken")
            .field("cancelled", &self.is_cancelled())
            .finish_non_exhaustive()
    }
}

/// Shared cancellation generation state.
struct CancellationInner {
    /// Generation identity.
    generation: u64,
    /// Mutable request state.
    state: Mutex<CancellationState>,
}

/// Mutable cancellation state guarded by [`CancellationInner::state`].
#[derive(Default)]
struct CancellationState {
    /// Sequence when cancellation was requested.
    request_sequence: Option<u64>,
    /// Wait sets currently waiting with tokens for this generation.
    watchers: Vec<CancellationWatcher>,
    /// Next watcher identity.
    next_watcher_id: u64,
}

/// A waiting wait-set notification target.
struct CancellationWatcher {
    /// Watcher identity used for removal.
    id: u64,
    /// Wait set to notify.
    wait_set: Weak<WaitSetInner>,
}

/// RAII watcher removal for a single wait call.
pub(super) struct CancellationWaitGuard {
    /// Cancellation state to remove from on drop.
    cancellation: Option<Weak<CancellationInner>>,
    /// Watcher identity.
    watcher_id: u64,
}

impl CancellationWaitGuard {
    /// Return an inert wait guard.
    const fn empty() -> Self {
        Self {
            cancellation: None,
            watcher_id: 0,
        }
    }
}

impl Drop for CancellationWaitGuard {
    fn drop(&mut self) {
        let Some(cancellation) = self.cancellation.as_ref().and_then(Weak::upgrade) else {
            return;
        };
        lock_or_recover(&cancellation.state)
            .watchers
            .retain(|watcher| watcher.id != self.watcher_id);
    }
}
