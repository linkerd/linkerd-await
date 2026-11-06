#![cfg(unix)]

use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    thread,
};

/// Starts a fake proxy admin server that answers every request with
/// `200 OK`, and returns its port.
fn start_admin_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("must bind");
    let port = listener.local_addr().expect("must have address").port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        }
    });
    port
}

fn run_with_shutdown(cmd: &[&str]) -> std::process::ExitStatus {
    let port = start_admin_server().to_string();
    Command::new(env!("CARGO_BIN_EXE_linkerd-await"))
        .env_remove("LINKERD_AWAIT_DISABLED")
        .env_remove("LINKERD_DISABLED")
        .args(["--port", &port, "--shutdown", "--"])
        .args(cmd)
        .status()
        .expect("linkerd-await must run")
}

#[test]
fn shutdown_preserves_exit_code() {
    let status = run_with_shutdown(&["sh", "-c", "exit 3"]);
    assert_eq!(status.code(), Some(3));
}

#[test]
fn shutdown_reports_signal_termination() {
    let status = run_with_shutdown(&["sh", "-c", "kill -TERM $$"]);
    assert_eq!(status.code(), Some(128 + 15));

    let status = run_with_shutdown(&["sh", "-c", "kill -KILL $$"]);
    assert_eq!(status.code(), Some(128 + 9));
}

#[test]
fn shutdown_reports_spawn_failure() {
    let status = run_with_shutdown(&["/nonexistent/linkerd-await-test-cmd"]);
    assert_eq!(status.code(), Some(71));
}
