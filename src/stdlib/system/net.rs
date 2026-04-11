//! Network socket abstractions — trait-based for mockability.
//!
//! This module provides pure-Rust trait abstractions over TCP and UDP sockets
//! so that Opalescent programs can be tested without opening real network
//! connections.  The traits mirror the essential surface of `std::net` while
//! remaining `alloc`-friendly for the types that live in memory.
//!
//! Production implementations (`StdTcpStream`, `StdUdpSocket`) wrap the
//! corresponding `std::net` types.  Test doubles (`MockTcpStream`,
//! `MockUdpSocket`) operate entirely in memory.

extern crate alloc;
extern crate std;

use alloc::string::String;
use alloc::vec::Vec;

/// Address of a network endpoint expressed as a `(host, port)` pair.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SocketAddr {
    /// Hostname or IP address string (e.g. `"127.0.0.1"` or `"localhost"`).
    pub host: String,
    /// Port number.
    pub port: u16,
}

impl SocketAddr {
    /// Creates a new [`SocketAddr`].
    #[must_use]
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: String::from(host),
            port,
        }
    }
}

/// Error type returned by socket operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetError {
    /// Human-readable description.
    pub message: String,
}

impl NetError {
    /// Creates a new [`NetError`] with the given message.
    #[must_use]
    pub fn new(message: &str) -> Self {
        Self {
            message: String::from(message),
        }
    }
}

/// A byte-stream socket (TCP-like).
pub trait TcpStream {
    /// Reads up to `buf.len()` bytes into `buf`.
    ///
    /// Returns the number of bytes read, or a [`NetError`] on failure.
    /// A return value of `Ok(0)` indicates the connection was closed.
    ///
    /// # Errors
    ///
    /// Returns a [`NetError`] if the underlying transport fails.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, NetError>;

    /// Writes all bytes in `buf` to the stream.
    ///
    /// Returns the number of bytes written, or a [`NetError`] on failure.
    ///
    /// # Errors
    ///
    /// Returns a [`NetError`] if the underlying transport fails.
    fn write(&mut self, buf: &[u8]) -> Result<usize, NetError>;

    /// Closes the connection.
    ///
    /// # Errors
    ///
    /// Returns a [`NetError`] if the underlying transport fails to close.
    fn close(&mut self) -> Result<(), NetError>;

    /// Returns the remote address of this connection.
    fn peer_addr(&self) -> &SocketAddr;
}

/// A datagram socket (UDP-like).
pub trait UdpSocket {
    /// Sends `buf` to `addr`.
    ///
    /// Returns the number of bytes sent, or a [`NetError`] on failure.
    ///
    /// # Errors
    ///
    /// Returns a [`NetError`] if the send operation fails.
    fn send_to(&mut self, buf: &[u8], addr: &SocketAddr) -> Result<usize, NetError>;

    /// Receives a datagram into `buf`.
    ///
    /// Returns `(bytes_read, sender_addr)`, or a [`NetError`] on failure.
    ///
    /// # Errors
    ///
    /// Returns a [`NetError`] if no datagrams are available or the receive fails.
    fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr), NetError>;

    /// Returns the local address this socket is bound to.
    fn local_addr(&self) -> &SocketAddr;
}

/// In-memory [`TcpStream`] for use in tests.
///
/// Reads are served from a pre-loaded `read_data` buffer; writes are
/// appended to `written`.
#[derive(Debug, Default)]
pub struct MockTcpStream {
    /// Data returned by successive `read` calls (consumed from the front).
    pub read_data: Vec<u8>,
    /// Data accumulated by `write` calls.
    pub written: Vec<u8>,
    /// Whether `close` has been called.
    pub closed: bool,
    /// The (fake) remote address reported by `peer_addr`.
    pub addr: SocketAddr,
    /// When `Some`, `read` returns this error instead of data.
    pub read_error: Option<NetError>,
    /// When `Some`, `write` returns this error instead of success.
    pub write_error: Option<NetError>,
    /// Read cursor position into `read_data`.
    pub read_pos: usize,
}

impl MockTcpStream {
    /// Creates a new [`MockTcpStream`] with a fake remote address and initial
    /// read payload.
    #[must_use]
    pub fn new(host: &str, port: u16, read_data: Vec<u8>) -> Self {
        Self {
            read_data,
            written: Vec::new(),
            closed: false,
            addr: SocketAddr::new(host, port),
            read_error: None,
            write_error: None,
            read_pos: 0,
        }
    }
}

impl TcpStream for MockTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, NetError> {
        if let Some(err) = self.read_error.clone() {
            return Err(err);
        }
        let available = self.read_data.len().saturating_sub(self.read_pos);
        if available == 0 {
            return Ok(0);
        }
        let n = available.min(buf.len());
        let end = self.read_pos.saturating_add(n);
        buf[..n].copy_from_slice(&self.read_data[self.read_pos..end]);
        self.read_pos = self.read_pos.saturating_add(n);
        Ok(n)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, NetError> {
        if let Some(err) = self.write_error.clone() {
            return Err(err);
        }
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn close(&mut self) -> Result<(), NetError> {
        self.closed = true;
        Ok(())
    }

    fn peer_addr(&self) -> &SocketAddr {
        &self.addr
    }
}

/// In-memory [`UdpSocket`] for use in tests.
///
/// `send_to` appends `(addr, bytes)` to `sent`; `recv_from` serves from
/// the pre-loaded `incoming` queue.
#[derive(Debug, Default)]
pub struct MockUdpSocket {
    /// Local address this socket is bound to.
    pub local: SocketAddr,
    /// Datagrams to serve from `recv_from`, consumed in order.
    pub incoming: Vec<(SocketAddr, Vec<u8>)>,
    /// Datagrams recorded by `send_to`.
    pub sent: Vec<(SocketAddr, Vec<u8>)>,
    /// Cursor into `incoming`.
    pub recv_pos: usize,
}

impl MockUdpSocket {
    /// Creates a new [`MockUdpSocket`] bound to the given local address.
    #[must_use]
    pub fn new(local_host: &str, local_port: u16) -> Self {
        Self {
            local: SocketAddr::new(local_host, local_port),
            incoming: Vec::new(),
            sent: Vec::new(),
            recv_pos: 0,
        }
    }

    /// Queues a datagram to be returned by the next `recv_from` call.
    pub fn push_incoming(&mut self, from: SocketAddr, data: Vec<u8>) {
        self.incoming.push((from, data));
    }
}

impl UdpSocket for MockUdpSocket {
    fn send_to(&mut self, buf: &[u8], addr: &SocketAddr) -> Result<usize, NetError> {
        self.sent.push((addr.clone(), buf.to_owned()));
        Ok(buf.len())
    }

    fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr), NetError> {
        let pos = self.recv_pos;
        if pos >= self.incoming.len() {
            return Err(NetError::new("no more datagrams"));
        }
        let n = self.incoming[pos].1.len().min(buf.len());
        let addr = self.incoming[pos].0.clone();
        let data_slice = self.incoming[pos].1[..n].to_owned();
        buf[..n].copy_from_slice(&data_slice);
        self.recv_pos = self.recv_pos.saturating_add(1);
        Ok((n, addr))
    }

    fn local_addr(&self) -> &SocketAddr {
        &self.local
    }
}
