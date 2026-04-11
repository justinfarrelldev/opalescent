//! Thread spawning and synchronisation — trait-based for mockability.
//!
//! Opalescent's threading API is deliberately thin: it exposes the minimum
//! surface needed to spawn work, wait for results, share data safely, and
//! send messages between threads.  All real concurrency is delegated to Rust's
//! `std::thread`, `std::sync::Mutex`, and `std::sync::mpsc`.  Tests use the
//! mock variants which execute work synchronously on the calling thread.

extern crate alloc;
extern crate std;

use alloc::boxed::Box;
use alloc::string::String;

/// Error returned by threading operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadError {
    /// Human-readable description.
    pub message: String,
}

impl ThreadError {
    /// Creates a new [`ThreadError`] with the given message.
    #[must_use]
    pub fn new(message: &str) -> Self {
        Self {
            message: String::from(message),
        }
    }
}

/// A handle to a spawned task that can be joined to retrieve its result.
pub trait JoinHandle<T> {
    /// Waits for the associated task to finish and returns its result.
    ///
    /// # Errors
    ///
    /// Returns a [`ThreadError`] if the task panicked.
    fn join(self: Box<Self>) -> Result<T, ThreadError>;
}

/// Spawn a concurrent task.
pub trait Spawner {
    /// Spawns `f` as a concurrent task and returns a [`JoinHandle`] that
    /// produces the return value when joined.
    fn spawn<T, F>(&self, f: F) -> Box<dyn JoinHandle<T>>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static;
}

/// Production [`Spawner`] backed by [`std::thread::spawn`].
#[derive(Debug, Clone, Copy, Default)]
pub struct StdSpawner;

/// Production [`JoinHandle`] wrapping [`std::thread::JoinHandle`].
pub struct StdJoinHandle<T> {
    /// The wrapped standard library join handle.
    inner: std::thread::JoinHandle<T>,
}

impl<T: Send + 'static> JoinHandle<T> for StdJoinHandle<T> {
    fn join(self: Box<Self>) -> Result<T, ThreadError> {
        self.inner
            .join()
            .map_err(|e| ThreadError::new(&format!("thread panicked: {e:?}")))
    }
}

impl Spawner for StdSpawner {
    fn spawn<T, F>(&self, f: F) -> Box<dyn JoinHandle<T>>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        Box::new(StdJoinHandle {
            inner: std::thread::spawn(f),
        })
    }
}

/// Synchronous (inline) [`Spawner`] that runs the closure on the calling
/// thread — suitable for deterministic tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct SyncSpawner;

/// [`JoinHandle`] returned by [`SyncSpawner`] — the value is already
/// computed.
pub struct SyncJoinHandle<T> {
    /// The pre-computed result value.
    value: T,
}

impl<T: Send + 'static> JoinHandle<T> for SyncJoinHandle<T> {
    fn join(self: Box<Self>) -> Result<T, ThreadError> {
        Ok(self.value)
    }
}

impl Spawner for SyncSpawner {
    fn spawn<T, F>(&self, f: F) -> Box<dyn JoinHandle<T>>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        Box::new(SyncJoinHandle { value: f() })
    }
}

/// A fair, blocking mutual-exclusion lock.
pub trait OpalMutex<T> {
    /// Acquires the lock and returns a guard that releases it on drop.
    ///
    /// # Errors
    ///
    /// Returns a [`ThreadError`] if the mutex is poisoned.
    fn lock(&self) -> Result<MutexGuard<'_, T>, ThreadError>;
}

/// RAII guard returned by [`OpalMutex::lock`].
///
/// Dereferencing the guard gives mutable access to the protected value.
pub struct MutexGuard<'guard, T> {
    /// The wrapped standard library mutex guard.
    inner: std::sync::MutexGuard<'guard, T>,
}

impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

/// Production [`OpalMutex`] backed by [`std::sync::Mutex`].
pub struct StdMutex<T> {
    /// The wrapped standard library mutex.
    inner: std::sync::Mutex<T>,
}

impl<T> StdMutex<T> {
    /// Creates a new [`StdMutex`] protecting `value`.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            inner: std::sync::Mutex::new(value),
        }
    }
}

impl<T> OpalMutex<T> for StdMutex<T> {
    fn lock(&self) -> Result<MutexGuard<'_, T>, ThreadError> {
        self.inner
            .lock()
            .map(|g| MutexGuard { inner: g })
            .map_err(|e| ThreadError::new(&e.to_string()))
    }
}

/// A multi-producer, single-consumer channel end-point pair.
pub struct Channel<T> {
    /// The sending half.
    pub sender: std::sync::mpsc::Sender<T>,
    /// The receiving half.
    pub receiver: std::sync::mpsc::Receiver<T>,
}

impl<T> Channel<T> {
    /// Creates a new unbounded MPSC channel.
    #[must_use]
    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self { sender, receiver }
    }
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Self::new()
    }
}
