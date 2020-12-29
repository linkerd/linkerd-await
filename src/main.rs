#![deny(warnings, rust_2018_idioms)]

use regex::Regex;
use std::{convert::TryInto, error, fmt, str::FromStr};
use structopt::StructOpt;
use tokio::time;

#[derive(Clone, Debug, StructOpt)]
#[structopt()]
/// Wait for linkerd to become ready before running a program.
struct Opt {
    #[structopt(
        short = "p",
        long = "port",
        default_value = "4191",
        help = "The port of the local Linkerd proxy admin server"
    )]
    port: u16,

    #[structopt(
        short = "b",
        long = "backoff",
        default_value = "1s",
        parse(try_from_str = parse_duration),
        help = "Time to wait after a failed readiness check",
    )]
    backoff: time::Duration,

    #[structopt(
        short = "S",
        long = "shutdown",
        help = "Causes the program to be forked so that proxy shutdown can be triggered once it completes"
    )]
    shutdown: bool,

    #[structopt(name = "CMD")]
    cmd: Vec<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Opt {
        port,
        backoff,
        shutdown,
        cmd,
    } = Opt::from_args();

    let authority = http::uri::Authority::from_str(&format!("localhost:{}", port)).unwrap();

    if cmd.is_empty() {
        std::process::exit(0);
    }

    let disabled_reason = std::env::var("LINKERD_DISABLED")
        .ok()
        .filter(|v| !v.is_empty());
    match disabled_reason {
        Some(ref reason) => eprintln!("Linkerd readiness check skipped: {}", reason),
        None => {
            await_ready(authority.clone(), backoff).await;
        }
    }

    let mut args = cmd.into_iter();
    if let Some(command) = args.next() {
        if shutdown {
            let ex = fork_and_wait(command, args).await;
            if disabled_reason.is_none() {
                send_shutdown(authority).await;
            }
            if let Ok(status) = ex {
                if let Some(code) = status.code() {
                    std::process::exit(code);
                }
            }
            std::process::exit(EX_OSERR);
        } else {
            exec(command, args);
        }
    }
}

const EX_OSERR: i32 = 71;

fn exec(name: String, args: impl IntoIterator<Item = String>) {
    use std::{
        os::unix::process::CommandExt,
        process::{self, Command},
    };

    let mut cmd = Command::new(&name);
    cmd.args(args);

    let err = cmd.exec();
    eprintln!("Failed to exec child program: {}: {}", name, err);
    process::exit(EX_OSERR);
}

#[allow(warnings)]
async fn fork_and_wait(
    name: String,
    args: impl IntoIterator<Item = String>,
) -> std::io::Result<std::process::ExitStatus> {
    use tokio::{
        process::Command,
        signal::unix::{signal, SignalKind},
    };

    let mut cmd = Command::new(&name);
    cmd.args(args);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to fork child program: {}: {}", name, e);
            std::process::exit(EX_OSERR);
        }
    };

    if let Some(id) = child.id() {
        // Proxy relevant signals.
        let cid = nix::unistd::Pid::from_raw(id.try_into().expect("Invalid PID"));
        tokio::spawn(async move {
            // SIGTERM - Kubernetes sends this to start a graceful shutdown.
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to register signal handler");
            sigterm.recv().await;
            if let Err(e) = nix::sys::signal::kill(cid, nix::sys::signal::Signal::SIGTERM) {
                eprintln!("Failed to forward SIGTERM to child process: {}", e);
            }
        });
    }

    child.wait().await
}

async fn await_ready(auth: http::uri::Authority, backoff: time::Duration) {
    let uri = hyper::Uri::builder()
        .scheme(http::uri::Scheme::HTTP)
        .authority(auth)
        .path_and_query("/ready")
        .build()
        .unwrap();

    let client = hyper::Client::default();
    loop {
        match client.get(uri.clone()).await {
            Ok(ref rsp) if rsp.status().is_success() => return,
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

#[derive(Copy, Clone, Debug)]
struct InvalidDuration;

impl fmt::Display for InvalidDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid duration")
    }
}

impl error::Error for InvalidDuration {}
