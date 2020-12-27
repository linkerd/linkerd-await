#![deny(warnings, rust_2018_idioms)]

use regex::Regex;
use std::{convert::TryInto, error, fmt};
use structopt::StructOpt;
use tokio::time::{delay_for, Duration};

#[derive(Clone, Debug, StructOpt)]
#[structopt()]
/// Wait for linkerd to become ready before running a program.
struct Opt {
    #[structopt(
        short = "u",
        long = "base-url",
        default_value = "http://127.0.0.1:4191/"
    )]
    base_url: http::Uri,

    #[structopt(
        short = "b",
        long = "backoff",
        default_value = "1s",
        parse(try_from_str = parse_duration)
    )]
    backoff: Duration,

    #[structopt(
        short = "S",
        long = "shutdown",
        help = "Causes the program to be forked so that proxy shutdown can be triggered once it completes"
    )]
    shutdown: bool,

    #[structopt(name = "CMD")]
    cmd: Vec<String>,
}

#[tokio::main]
async fn main() {
    let Opt {
        base_url,
        backoff,
        shutdown,
        cmd,
    } = Opt::from_args();

    let disabled_reason = std::env::var("LINKERD_DISABLED")
        .ok()
        .filter(|v| !v.is_empty());
    match disabled_reason {
        Some(reason) => eprintln!("Linkerd readiness check skipped: {}", reason),
        None => {
            await_ready(base_url.clone(), backoff).await;
        }
    }

    let mut args = cmd.into_iter();
    if let Some(command) = args.next() {
        if shutdown {
            let res = fork_and_wait(command, args).await;
            send_shutdown(base_url).await;
            if let Ok(status) = res {
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

    let child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to fork child program: {}: {}", name, e);
            std::process::exit(EX_OSERR);
        }
    };

    // Proxy relevant signals.
    let cid = nix::unistd::Pid::from_raw(child.id().try_into().expect("Invalid PID"));
    tokio::spawn(async move {
        // SIGINT  - To allow Ctrl-c to emulate SIGTERM while developing.
        let mut sigint =
            signal(SignalKind::interrupt()).expect("Failed to register signal handler");
        // SIGTERM - Kubernetes sends this to start a graceful shutdown.
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to register signal handler");
        tokio::select! {
            _ = sigint.recv() => {
                if let Err(e) = nix::sys::signal::kill(cid, nix::sys::signal::Signal::SIGINT) {
                    eprintln!("Failed to forward SIGINT to child process: {}", e);
                }
            }
            _ = sigterm.recv() => {
                if let Err(e) = nix::sys::signal::kill(cid, nix::sys::signal::Signal::SIGTERM) {
                    eprintln!("Failed to forward SIGTERM to child process: {}", e);
                }
            }
        };
    });

    child.await
}

async fn await_ready(base_url: http::Uri, backoff: Duration) {
    let uri = {
        let mut parts = base_url.into_parts();
        parts.path_and_query = Some("/ready".try_into().unwrap());
        http::Uri::from_parts(parts).expect("Ready URI must be valid")
    };

    let client = hyper::Client::default();
    loop {
        match client.get(uri.clone()).await {
            Ok(ref rsp) if rsp.status().is_success() => return,
            _ => delay_for(backoff).await,
        }
    }
}

async fn send_shutdown(base_url: http::Uri) {
    let uri = {
        let mut parts = base_url.into_parts();
        parts.path_and_query = Some("/shutdown".try_into().unwrap());
        http::Uri::from_parts(parts).expect("Shutdown URI must be valid")
    };

    let req = http::Request::builder()
        .method(http::Method::POST)
        .uri(uri)
        .body(Default::default())
        .expect("shutdown request must be valid");

    let _ = hyper::Client::default().request(req).await;
}

fn parse_duration(s: &str) -> Result<Duration, InvalidDuration> {
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
