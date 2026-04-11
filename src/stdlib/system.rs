//! Opalescent system standard library — OS interfaces, networking, threading,
//! and process management.
//!
//! All modules expose trait-based abstractions so that Opalescent programs
//! remain testable without requiring a live OS, network, or thread scheduler.
//!
//! # Sub-modules
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`platform`] | Compile-time OS / CPU detection |
//! | [`env`] | Environment-variable access (`EnvProvider` trait + mocks) |
//! | [`args`] | Command-line argument access (`ArgsProvider` trait + mocks) |
//! | [`net`] | TCP / UDP socket abstractions (`TcpStream`, `UdpSocket` traits) |
//! | [`thread`] | Thread spawning, mutex, and MPSC channel |
//! | [`process`] | Child-process spawning, signal delivery, and exit |

#[path = "system/args.rs"]
pub mod args;
#[path = "system/env.rs"]
pub mod env;
#[path = "system/net.rs"]
pub mod net;
#[path = "system/platform.rs"]
pub mod platform;
#[path = "system/process.rs"]
pub mod process;
#[path = "system/tests.rs"]
pub mod tests;
#[path = "system/thread.rs"]
pub mod thread;
