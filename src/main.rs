#![deny(warnings, rust_2018_idioms)]

use std::{error, fmt};
use structopt::StructOpt;
use tokio::time;

#[derive(Clone, Debug, StructOpt)]
#[structopt()]
/// Wait for linkerd to become ready before running a program.
struct Opt {
    #[structopt(
        short = "u",
        long = "uri",
        default_value = "http://127.0.0.1:4191/ready"
    )]
    uri: hyper::Uri,

    #[structopt(
        short = "b",
        long = "backoff",
        default_value = "1s",
        parse(try_from_str = parse_duration)
    )]
    backoff: time::Duration,

    #[structopt(name = "CMD")]
    cmd: Vec<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    use std::os::unix::process::CommandExt;
    use std::process::{self, Command};

    let Opt { uri, backoff, cmd } = Opt::from_args();

    let disabled_reason = std::env::var("LINKERD_DISABLED")
        .ok()
        .filter(|v| !v.is_empty());
    match disabled_reason {
        Some(reason) => eprintln!("Linkerd readiness check skipped: {}", reason),
        None => {
            await_ready(uri, backoff).await;
        }
    }

    let mut args = cmd.into_iter();
    if let Some(command) = args.next() {
        let mut cmd = Command::new(&command);
        cmd.args(args);

        let err = cmd.exec();
        eprintln!("Failed to exec child program: {}: {}", command, err);
        process::exit(1);
    }
}

async fn await_ready(uri: hyper::Uri, backoff: time::Duration) {
    let client = hyper::Client::default();
    loop {
        match client.get(uri.clone()).await {
            Ok(ref rsp) if rsp.status().is_success() => return,
            _ => time::sleep(backoff).await,
        }
    }
}

fn parse_duration(s: &str) -> Result<time::Duration, InvalidDuration> {
    use regex::Regex;
    use time::Duration;

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
