//! Tests for the system standard library.
//!
//! All tests use mock / in-memory implementations — no actual processes are
//! spawned, no network connections are opened, and no threads are created.

#[cfg(test)]
mod system_tests {
    use alloc::string::String;
    use alloc::vec;

    use crate::stdlib::system::{
        args::{ArgsProvider as _, MockArgs},
        env::{EnvProvider as _, MockEnv},
        net::{MockTcpStream, MockUdpSocket, NetError, SocketAddr, TcpStream as _, UdpSocket as _},
        platform::{Arch, OsKind, Platform},
        process::{ChildProcess as _, ExitCode, MockProcessManager, ProcessManager as _, Signal},
        thread::{OpalMutex as _, Spawner as _, StdMutex, SyncSpawner},
    };

    extern crate alloc;

    // ── Platform ──────────────────────────────────────────────────────────

    #[test]
    fn platform_current_returns_valid_os() {
        let p = Platform::current();
        // Should be one of the known OS kinds (not an unknown default).
        assert!(matches!(
            p.os,
            OsKind::Linux
                | OsKind::MacOs
                | OsKind::Windows
                | OsKind::FreeBsd
                | OsKind::OpenBsd
                | OsKind::NetBsd
                | OsKind::Other
        ));
    }

    #[test]
    fn platform_current_returns_valid_arch() {
        let p = Platform::current();
        assert!(matches!(
            p.arch,
            Arch::X86_64
                | Arch::X86
                | Arch::Aarch64
                | Arch::Arm
                | Arch::Riscv64
                | Arch::Mips64
                | Arch::Wasm32
                | Arch::Other
        ));
    }

    #[test]
    fn platform_linux_helpers() {
        let linux = Platform {
            os: OsKind::Linux,
            arch: Arch::X86_64,
        };
        assert!(linux.is_linux());
        assert!(!linux.is_macos());
        assert!(!linux.is_windows());
    }

    #[test]
    fn platform_macos_helpers() {
        let mac = Platform {
            os: OsKind::MacOs,
            arch: Arch::Aarch64,
        };
        assert!(mac.is_macos());
        assert!(!mac.is_linux());
        assert!(mac.is_aarch64());
    }

    #[test]
    fn platform_windows_helpers() {
        let win = Platform {
            os: OsKind::Windows,
            arch: Arch::X86_64,
        };
        assert!(win.is_windows());
        assert!(win.is_x86_64());
    }

    #[test]
    fn platform_equality() {
        let a = Platform {
            os: OsKind::Linux,
            arch: Arch::X86_64,
        };
        let b = Platform {
            os: OsKind::Linux,
            arch: Arch::X86_64,
        };
        assert_eq!(a, b);
    }

    // ── Env ───────────────────────────────────────────────────────────────

    #[test]
    fn mock_env_get_existing_var() {
        let env = MockEnv::new(&[("HOME", "/root"), ("PATH", "/usr/bin")]);
        assert_eq!(env.get("HOME"), Some(String::from("/root")));
        assert_eq!(env.get("PATH"), Some(String::from("/usr/bin")));
    }

    #[test]
    fn mock_env_get_missing_var() {
        let env = MockEnv::new(&[("X", "1")]);
        assert_eq!(env.get("Y"), None);
    }

    #[test]
    fn mock_env_contains() {
        let env = MockEnv::new(&[("PRESENT", "yes")]);
        assert!(env.contains("PRESENT"));
        assert!(!env.contains("ABSENT"));
    }

    #[test]
    fn mock_env_all_returns_all_vars() {
        let env = MockEnv::new(&[("A", "1"), ("B", "2")]);
        let all = env.all();
        assert_eq!(all.len(), 2);
        // BTreeMap iteration is sorted
        assert_eq!(all[0], (String::from("A"), String::from("1")));
        assert_eq!(all[1], (String::from("B"), String::from("2")));
    }

    #[test]
    fn mock_env_set_and_remove() {
        let mut env = MockEnv::new(&[("X", "old")]);
        env.set("X", "new");
        assert_eq!(env.get("X"), Some(String::from("new")));
        env.remove("X");
        assert_eq!(env.get("X"), None);
    }

    #[test]
    fn mock_env_empty() {
        let env = MockEnv::new(&[]);
        assert!(env.all().is_empty());
    }

    // ── Args ──────────────────────────────────────────────────────────────

    #[test]
    fn mock_args_get_by_index() {
        let args = MockArgs::new(&["opal", "--release", "build"]);
        assert_eq!(args.get(0), Some(String::from("opal")));
        assert_eq!(args.get(1), Some(String::from("--release")));
        assert_eq!(args.get(2), Some(String::from("build")));
        assert_eq!(args.get(3), None);
    }

    #[test]
    fn mock_args_len() {
        let args = MockArgs::new(&["a", "b", "c"]);
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn mock_args_is_empty() {
        let empty = MockArgs::new(&[]);
        assert!(empty.is_empty());
        let nonempty = MockArgs::new(&["prog"]);
        assert!(!nonempty.is_empty());
    }

    #[test]
    fn mock_args_full_list() {
        let args = MockArgs::new(&["opal", "run", "main.op"]);
        let list = args.args();
        assert_eq!(list.len(), 3);
        assert_eq!(list[2], String::from("main.op"));
    }

    // ── Net — TcpStream ───────────────────────────────────────────────────

    #[test]
    fn mock_tcp_read_all_bytes() {
        let payload = vec![1_u8, 2, 3, 4, 5];
        let mut stream = MockTcpStream::new("127.0.0.1", 8080, payload.clone());
        let mut buf = [0_u8; 5];
        let n = stream.read(&mut buf).expect("read should succeed");
        assert_eq!(n, 5);
        assert_eq!(&buf[..n], payload.as_slice());
    }

    #[test]
    fn mock_tcp_read_partial() {
        let mut stream = MockTcpStream::new("127.0.0.1", 8080, vec![10_u8, 20, 30]);
        let mut buf = [0_u8; 2];
        let n = stream.read(&mut buf).expect("read should succeed");
        assert_eq!(n, 2);
        assert_eq!(buf, [10, 20]);
    }

    #[test]
    fn mock_tcp_read_eof() {
        let mut stream = MockTcpStream::new("127.0.0.1", 8080, vec![]);
        let mut buf = [0_u8; 4];
        let n = stream.read(&mut buf).expect("read should succeed");
        assert_eq!(n, 0);
    }

    #[test]
    fn mock_tcp_write_accumulates() {
        let mut stream = MockTcpStream::new("127.0.0.1", 8080, vec![]);
        stream.write(b"hello").expect("write should succeed");
        stream.write(b" world").expect("write should succeed");
        assert_eq!(stream.written, b"hello world");
    }

    #[test]
    fn mock_tcp_close() {
        let mut stream = MockTcpStream::new("127.0.0.1", 8080, vec![]);
        stream.close().expect("close should succeed");
        assert!(stream.closed);
    }

    #[test]
    fn mock_tcp_peer_addr() {
        let stream = MockTcpStream::new("10.0.0.1", 9000, vec![]);
        assert_eq!(stream.peer_addr().host, "10.0.0.1");
        assert_eq!(stream.peer_addr().port, 9000);
    }

    #[test]
    fn mock_tcp_read_error() {
        let mut stream = MockTcpStream::new("127.0.0.1", 80, vec![1, 2]);
        stream.read_error = Some(NetError::new("connection reset"));
        let result = stream.read(&mut [0_u8; 4]);
        assert_eq!(result, Err(NetError::new("connection reset")));
    }

    #[test]
    fn mock_tcp_write_error() {
        let mut stream = MockTcpStream::new("127.0.0.1", 80, vec![]);
        stream.write_error = Some(NetError::new("broken pipe"));
        let result = stream.write(b"data");
        assert_eq!(result, Err(NetError::new("broken pipe")));
    }

    // ── Net — UdpSocket ───────────────────────────────────────────────────

    #[test]
    fn mock_udp_send_to_records_packet() {
        let mut socket = MockUdpSocket::new("0.0.0.0", 5000);
        let dest = SocketAddr::new("1.2.3.4", 5001);
        socket
            .send_to(b"ping", &dest)
            .expect("send_to should succeed");
        assert_eq!(socket.sent.len(), 1);
        assert_eq!(socket.sent[0].1, b"ping");
        assert_eq!(socket.sent[0].0.host, "1.2.3.4");
    }

    #[test]
    fn mock_udp_recv_from_serves_queued() {
        let mut socket = MockUdpSocket::new("0.0.0.0", 5000);
        let from = SocketAddr::new("9.9.9.9", 9);
        socket.push_incoming(from, vec![42_u8, 43]);
        let mut buf = [0_u8; 8];
        let (n, sender) = socket
            .recv_from(&mut buf)
            .expect("recv_from should succeed");
        assert_eq!(n, 2);
        assert_eq!(buf[0], 42);
        assert_eq!(sender.host, "9.9.9.9");
    }

    #[test]
    fn mock_udp_recv_from_empty_queue() {
        let mut socket = MockUdpSocket::new("0.0.0.0", 5000);
        let mut buf = [0_u8; 8];
        let result = socket.recv_from(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn mock_udp_local_addr() {
        let socket = MockUdpSocket::new("127.0.0.1", 4321);
        assert_eq!(socket.local_addr().port, 4321);
    }

    // ── Thread ────────────────────────────────────────────────────────────

    #[test]
    fn sync_spawner_runs_closure_inline() {
        let spawner = SyncSpawner;
        let handle = spawner.spawn(|| 42_u32);
        let result = handle.join().expect("join should succeed");
        assert_eq!(result, 42);
    }

    #[test]
    fn sync_spawner_closure_with_capture() {
        let spawner = SyncSpawner;
        let x = 7_u32;
        let handle = spawner.spawn(move || x * 6);
        let result = handle.join().expect("join should succeed");
        assert_eq!(result, 42);
    }

    #[test]
    fn std_mutex_lock_and_mutate() {
        let m = StdMutex::new(0_u32);
        {
            let mut g = m.lock().expect("lock should succeed");
            *g = 99;
        }
        let val = {
            let g = m.lock().expect("lock should succeed");
            *g
        };
        assert_eq!(val, 99);
    }

    #[test]
    fn channel_send_receive() {
        use crate::stdlib::system::thread::Channel;
        let ch: Channel<u32> = Channel::new();
        ch.sender.send(1).expect("send should succeed");
        ch.sender.send(2).expect("send should succeed");
        assert_eq!(ch.receiver.recv().expect("recv should succeed"), 1);
        assert_eq!(ch.receiver.recv().expect("recv should succeed"), 2);
    }

    // ── Process ───────────────────────────────────────────────────────────

    #[test]
    fn mock_process_spawn_records_call() {
        let mut pm = MockProcessManager::new();
        let mut child = pm.spawn("echo", &["hello"]).expect("spawn should succeed");
        let code = child.wait().expect("wait should succeed");
        assert!(code.is_success());
        assert_eq!(pm.spawn_calls.len(), 1);
        assert_eq!(pm.spawn_calls[0].program, "echo");
        assert_eq!(pm.spawn_calls[0].args, vec![String::from("hello")]);
    }

    #[test]
    fn mock_process_current_pid() {
        let pm = MockProcessManager::new();
        assert_eq!(pm.current_pid(), 1234);
    }

    #[test]
    fn mock_process_current_dir() {
        let pm = MockProcessManager::new();
        assert_eq!(
            pm.current_dir().expect("current_dir should succeed"),
            String::from("/mock/dir")
        );
    }

    #[test]
    fn exit_code_success_and_failure() {
        assert!(ExitCode::SUCCESS.is_success());
        assert!(!ExitCode::FAILURE.is_success());
        assert_eq!(ExitCode::SUCCESS.code(), 0_i32);
        assert_eq!(ExitCode::FAILURE.code(), 1_i32);
    }

    #[test]
    fn signal_constants() {
        assert_eq!(Signal::TERM.0, 15_i32);
        assert_eq!(Signal::KILL.0, 9_i32);
        assert_eq!(Signal::INT.0, 2_i32);
        assert_eq!(Signal::HUP.0, 1_i32);
    }

    #[test]
    fn mock_child_send_signal_records() {
        use crate::stdlib::system::process::MockChildProcess;
        let mut child = MockChildProcess::new(999, ExitCode::SUCCESS);
        child
            .send_signal(Signal::TERM)
            .expect("send_signal should succeed");
        assert_eq!(child.received_signal, Some(Signal::TERM));
    }

    #[test]
    fn mock_process_spawn_multiple() {
        let mut pm = MockProcessManager::new();
        let _c1 = pm.spawn("ls", &["-la"]).expect("spawn 1");
        let _c2 = pm.spawn("cat", &["file.txt"]).expect("spawn 2");
        assert_eq!(pm.spawn_calls.len(), 2);
        assert_eq!(pm.spawn_calls[1].program, "cat");
    }
}
