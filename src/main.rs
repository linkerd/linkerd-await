extern crate futures;
extern crate http;
extern crate hyper;
extern crate regex;
extern crate structopt;
extern crate tokio;

use futures::Future;
use std::time::Duration;
use std::{error, fmt};
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
#[structopt()]
/// Wait for linkerd to become ready before running a program.
struct Opt {
    #[structopt(
        short = "u",
        long = "uri",
        default_value = "http://127.0.0.1:4191/ready"
    )]
    uri: http::Uri,

    #[structopt(
        short = "b",
        long = "backoff",
        default_value = "1s",
        parse(try_from_str = "parse_duration")
    )]
    backoff: Duration,

    #[structopt(name = "CMD")]
    cmd: Vec<String>,
}

fn main() {
    use std::os::unix::process::CommandExt;
    use std::process::{self, Command};
    use tokio::runtime::Runtime;

    let Opt { uri, backoff, cmd } = Opt::from_args();

    let mut rt = Runtime::new().expect("runtime");
    if rt.block_on(await_ready(uri, backoff)).is_err() {
        process::exit(1);
    }

    let mut args = cmd.into_iter();
    if let Some(command) = args.next() {
        let mut cmd = Command::new(&command);
        cmd.args(args);

        let err = cmd.exec();
        eprintln!("Failed to exec child program: {}: {}", command, err);
        process::exit(1);
    }

    process::exit(0);
}

fn await_ready(uri: http::Uri, backoff: Duration) -> impl Future<Item = (), Error = ()> {
    use futures::future::{self, Either, Loop};
    use std::time::Instant;
    use tokio::timer::Delay;

    let client = hyper::Client::default();
    future::loop_fn((client, uri, backoff), |(client, uri, backoff)| {
        client.get(uri.clone()).then(move |r| match r {
            Ok(ref rsp) if rsp.status().is_success() => Either::A(future::ok(Loop::Break(()))),
            _ => Either::B(
                Delay::new(Instant::now() + backoff)
                    .map_err(|e| panic!("timer failed: {}", e))
                    .map(move |_| Loop::Continue((client, uri, backoff))),
            ),
        })
    })
}

fn parse_duration(s: &str) -> Result<Duration, InvalidDuration> {
    use regex::Regex;

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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid duration")
    }
}

impl error::Error for InvalidDuration {}
