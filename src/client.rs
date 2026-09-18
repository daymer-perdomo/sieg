use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::protocol::{Request, Response};
use crate::server;

pub fn send_request(request: &Request) -> std::io::Result<Response> {
    ensure_server_running()?;
    let mut stream = connect()?;
    let payload = serde_json::to_string(request)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    writeln!(stream, "{payload}")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str(&line)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

fn connect() -> std::io::Result<UnixStream> {
    UnixStream::connect(server::socket_path())
}

fn ensure_server_running() -> std::io::Result<()> {
    if connect().is_ok() {
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg("__serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // Detach from this process's session so the server survives after the
    // CLI invocation that spawned it exits (and isn't SIGHUP'd if the
    // parent terminal closes).
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn()?;

    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if connect().is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "sieg server did not start in time",
    ))
}
