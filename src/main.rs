#![deny(warnings, rust_2018_idioms)]

use clap::Parser;
use regex::Regex;
use std::{convert::TryInto, error, fmt, io, process::ExitStatus, str::FromStr};
use tokio::time;

#[derive(Clone, Debug, Parser)]
#[clap(about, version)]
/// Wait for linkerd to become ready before running a program.
struct Args {
    #[clap(
        short = 'p',
        long = "port",
        default_value = "4191",
        help = "The port of the local Linkerd proxy admin server"
    )]
    port: u16,

    #[clap(
        short = 'b',
        long = "backoff",
        default_value = "1s",
        parse(try_from_str = parse_duration),
        help = "Time to wait after a failed readiness check",
    )]
    backoff: time::Duration,

    #[clap(
        short = 'S',
        long = "shutdown",
        help = "Forks the program and triggers proxy shutdown on completion",
        requires("CMD")
    )]
    shutdown: bool,

    #[clap(
        short = 'v',
        long = "verbose",
        help = "Causes linkerd-await to print an error message when disabled",
        env = "LINKERD_AWAIT_VERBOSE"
    )]
    verbose: bool,

    #[clap(
        short = 't',
        long = "timeout",
        parse(try_from_str = parse_duration),
        help = "Causes linked-await to fail when the timeout elapses before the proxy becomes ready"
    )]
    timeout: Option<time::Duration>,

    #[clap(name = "CMD", help = "The command to run after linkerd is ready")]
    cmd: Option<String>,

    #[clap(name = "ARGS", help = "Arguments to pass to CMD if specified")]
    args: Vec<String>,
}

// From https://man.netbsd.org/sysexits.3
const EX_OSERR: i32 = 71;
const EX_UNAVAILABLE: i32 = 69;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Args {
        port,
        backoff,
        shutdown,
        verbose,
        timeout,
        cmd,
        args,
    } = Args::parse();

    let authority = http::uri::Authority::from_str(&format!("localhost:{}", port))
        .expect("HTTP authority must be valid");

    // If linkerd is not explicitly disabled, wait until the proxy is ready
    // before running the application.
    match linkerd_disabled_reason() {
        Some(reason) => {
            if verbose {
                eprintln!("Linkerd readiness check skipped: {}", reason);
            }
        }
        None => {
            let await_timeout = async move {
                if let Some(timeout) = timeout {
                    if !timeout.is_zero() {
                        tokio::time::sleep(timeout).await;
                        return timeout;
                    }
                }
                futures::future::pending().await
            };
            tokio::select! {
                () = await_ready(authority.clone(), backoff) => {},
                timeout = await_timeout => {
                    eprintln!(
                        "linkerd-proxy failed to become ready within {:?} timeout",
                        timeout
                    );
                    std::process::exit(EX_UNAVAILABLE)
                }
            }
            if shutdown {
                let cmd = cmd.expect("Command must be specified with --shutdown");

                // If shutdown is configured, fork the process and proxy
                // SIGTERM.
                let ex = fork_with_sigterm(cmd, args).await;

                // Once the process completes, issue a shutdown request to the
                // proxy.
                send_shutdown(authority).await;

                // Try to exit with the process's original exit code
                if let Ok(status) = ex {
                    if let Some(code) = status.code() {
                        std::process::exit(code);
                    }
                }

                // If we didn't get an exit code from the forked program, fail
                // with an OS error.
                std::process::exit(EX_OSERR);
            }
        }
    }

    if let Some(cmd) = cmd {
        // If Linkerd shutdown is not configured, exec the process directly so
        // that the we don't have to bother with signal proxying, etc.
        exec(cmd, args);
    }
}

fn linkerd_disabled_reason() -> Option<String> {
    std::env::var("LINKERD_AWAIT_DISABLED")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::env::var("LINKERD_DISABLED")
                .ok()
                .filter(|v| !v.is_empty())
        })
}

/// Execs the process.
fn exec(cmd: String, args: Vec<String>) {
    use std::{
        os::unix::process::CommandExt,
        process::{self, Command},
    };

    // Execute the command (and never return). If the command could not be
    // executed, just exit with an OS error.
    let err = Command::new(&cmd).args(args).exec();
    eprintln!("Failed to exec child program: {}: {}", cmd, err);
    process::exit(EX_OSERR);
}

/// Forks the specified process, proxying SIGTERM.
async fn fork_with_sigterm(cmd: String, args: Vec<String>) -> io::Result<ExitStatus> {
    use nix::{
        sys::signal::{kill, Signal::SIGTERM},
        unistd::Pid,
    };
    use tokio::{
        process::Command,
        signal::unix::{signal, SignalKind},
    };

    let mut child = match Command::new(&cmd).args(args).spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to fork child program: {}: {}", cmd, e);
            std::process::exit(EX_OSERR);
        }
    };

    // If the process is running, wait until we receive a SIGTERM, which kubelet
    // uses to initiate graceful shutdown.
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");

    // Wait for the process to exit on its own or, if a SIGTERM is received,
    // proxy the signal so it begins shutdown.
    tokio::select! {
        ex = child.wait() => ex,
        _ = sigterm.recv() => {
            if let Some(pid) = child.id() {
                // If the child hasn't already completed, send a SIGTERM.
                if let Err(e) = kill(Pid::from_raw(pid.try_into().expect("Invalid PID")), SIGTERM) {
                    eprintln!("Failed to forward SIGTERM to child process: {}", e);
                }
            }
            // Wait to get the child's exit code.
            child.wait().await
        }
    }
}

async fn await_ready(auth: http::uri::Authority, backoff: time::Duration) {
    const TIMEOUT: time::Duration = time::Duration::from_secs(5);

    let uri = hyper::Uri::builder()
        .scheme(http::uri::Scheme::HTTP)
        .authority(auth)
        .path_and_query("/ready")
        .build()
        .unwrap();

    let client = hyper::Client::default();
    loop {
        match time::timeout(TIMEOUT, client.get(uri.clone())).await {
            Ok(Ok(ref rsp)) if rsp.status().is_success() => return,
            _ => time::sleep(backoff).await,
        }
    }
}

async fn send_shutdown(auth: http::uri::Authority) {
    let uri = hyper::Uri::builder()
        .scheme(http::uri::Scheme::HTTP)
        .authority(auth)
        .path_and_query("/shutdown")
        .build()
        .unwrap();

    let req = http::Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .body(Default::default())
        .expect("shutdown request must be valid");

    let _ = hyper::Client::default().request(req).await;
}

fn parse_duration(s: &str) -> Result<time::Duration, InvalidDuration> {
    use tokio::time::Duration;
    let re = Regex::new(r"^\s*(\d+)(ms|s|m|h|d)?\s*$").expect("duration regex");
    let cap = re.captures(s).ok_or(InvalidDuration)?;
    let magnitude = cap[1].parse().map_err(|_| InvalidDuration)?;
    match cap.get(2).map(|m| m.as_str()) {
        None if magnitude == 0 => Ok(Duration::from_secs(0)),
        Some("ms") => Ok(Duration::from_millis(magnitude)),
        Some("s") => Ok(Duration::from_secs(magnitude)),
        Some("m") => Ok(Duration::from_secs(magnitude * 60)),
        Some("h") => Ok(Duration::from_secs(magnitude * 60 * 60)),
        Some("d") => Ok(Duration::from_secs(magnitude * 60 * 60 * 24)),
        _ => Err(InvalidDuration),
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct InvalidDuration;

impl fmt::Display for InvalidDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid duration")
    }
}

impl error::Error for InvalidDuration {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_invalid() {
        assert_eq!(parse_duration(""), Err(InvalidDuration));
        assert_eq!(parse_duration("  "), Err(InvalidDuration));
        assert_eq!(parse_duration("\t\n"), Err(InvalidDuration));
        assert_eq!(parse_duration("x"), Err(InvalidDuration));
        assert_eq!(parse_duration("1"), Err(InvalidDuration));
        assert_eq!(parse_duration("0x"), Err(InvalidDuration));
        assert_eq!(parse_duration("123x"), Err(InvalidDuration));
        assert_eq!(parse_duration("  123x  "), Err(InvalidDuration));
    }

    #[test]
    fn test_parse_duration_valid() {
        use tokio::time::Duration;
        assert_eq!(parse_duration("0"), Ok(Duration::from_secs(0)));
        assert_eq!(parse_duration("0s"), Ok(Duration::from_secs(0)));
        assert_eq!(parse_duration("1ms"), Ok(Duration::from_millis(1)));
        assert_eq!(parse_duration("1s"), Ok(Duration::from_secs(1)));
        assert_eq!(parse_duration(" \n12s  \t"), Ok(Duration::from_secs(12)));
        assert_eq!(parse_duration("10s"), Ok(Duration::from_secs(10)));
        assert_eq!(parse_duration("10m"), Ok(Duration::from_secs(10 * 60)));
        assert_eq!(parse_duration("10h"), Ok(Duration::from_secs(10 * 60 * 60)));
        assert_eq!(
            parse_duration("10d"),
            Ok(Duration::from_secs(10 * 60 * 60 * 24))
        );
    }
}
