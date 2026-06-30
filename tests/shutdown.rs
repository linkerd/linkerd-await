use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

#[test]
fn disabled_mode_still_triggers_shutdown() {
    let (port, shutdown_hits, stop) = start_admin_server();

    let status = Command::new(env!("CARGO_BIN_EXE_linkerd-await"))
        .env("LINKERD_AWAIT_DISABLED", "1")
        .args(["--port", &port.to_string(), "--shutdown", "--", "/bin/true"])
        .status()
        .expect("binary should run");

    assert!(status.success());
    assert_shutdown_hits(&shutdown_hits, 1);

    stop.store(true, Ordering::Relaxed);
}

#[test]
fn shutdown_mode_triggers_shutdown_after_readiness() {
    let (port, shutdown_hits, stop) = start_admin_server();

    let status = Command::new(env!("CARGO_BIN_EXE_linkerd-await"))
        .args(["--port", &port.to_string(), "--shutdown", "--", "/bin/true"])
        .status()
        .expect("binary should run");

    assert!(status.success());
    assert_shutdown_hits(&shutdown_hits, 1);

    stop.store(true, Ordering::Relaxed);
}

fn start_admin_server() -> (u16, Arc<AtomicUsize>, Arc<AtomicBool>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
    listener
        .set_nonblocking(true)
        .expect("listener should become nonblocking");
    let port = listener
        .local_addr()
        .expect("listener should have a local address")
        .port();

    let shutdown_hits = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));

    let thread_shutdown_hits = Arc::clone(&shutdown_hits);
    let thread_stop = Arc::clone(&stop);
    thread::spawn(move || {
        while !thread_stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut buf = [0; 1024];
                    let n = stream.read(&mut buf).expect("request should be readable");
                    let req = std::str::from_utf8(&buf[..n]).expect("request must be utf-8");

                    let response = if req.starts_with("GET /ready ") {
                        "HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nready"
                    } else if req.starts_with("POST /shutdown ") {
                        thread_shutdown_hits.fetch_add(1, Ordering::Relaxed);
                        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"
                    } else {
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"
                    };

                    stream
                        .write_all(response.as_bytes())
                        .expect("response should be writable");
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("listener accept failed: {e}"),
            }
        }
    });

    (port, shutdown_hits, stop)
}

fn assert_shutdown_hits(shutdown_hits: &AtomicUsize, expected: usize) {
    let deadline = Instant::now() + Duration::from_secs(1);

    while Instant::now() < deadline {
        if shutdown_hits.load(Ordering::Relaxed) == expected {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(shutdown_hits.load(Ordering::Relaxed), expected);
}
