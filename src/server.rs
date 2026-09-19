use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};

use crate::protocol::{PaneInfo, PaneStatus, Request, Response};

/// Cap on buffered pane output so a chatty background process can't grow
/// server memory unbounded; oldest bytes are dropped first.
const MAX_OUTPUT_BYTES: usize = 200_000;

struct Pane {
    command: Vec<String>,
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<Vec<u8>>>,
    exit_code: Arc<Mutex<Option<u32>>>,
    kill_tx: Sender<()>,
    // Kept alive only so the PTY stays open for the lifetime of the pane.
    _master: Box<dyn MasterPty + Send>,
    pid: Option<u32>,
}

type Registry = Arc<Mutex<HashMap<String, Pane>>>;

pub fn socket_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".sieg").join("sieg.sock")
}

/// Runs the background pane server. Blocks forever; intended to run detached
/// (see `client::ensure_server_running`), never invoked directly by users.
pub fn run() -> std::io::Result<()> {
    let path = socket_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // A stale socket file from a crashed server blocks bind(); the caller
    // only spawns us after failing to connect, so it's safe to clear it.
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    let registry: Registry = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming().flatten() {
        let registry = Arc::clone(&registry);
        thread::spawn(move || handle_conn(stream, &registry));
    }
    Ok(())
}

fn handle_conn(stream: UnixStream, registry: &Registry) {
    let mut reader = match stream.try_clone() {
        Ok(clone) => BufReader::new(clone),
        Err(_) => return,
    };
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return;
    }

    let response = match serde_json::from_str::<Request>(&line) {
        Ok(request) => handle_request(request, registry),
        Err(err) => Response::err(format!("bad request: {err}")),
    };

    let mut out = stream;
    let payload = serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":false}".to_string());
    let _ = writeln!(out, "{payload}");
}

fn handle_request(request: Request, registry: &Registry) -> Response {
    match request {
        Request::Ping => Response::ok(),
        Request::Spawn { name, command } => spawn_pane(registry, name, command),
        Request::List => list_panes(registry),
        Request::Send { name, text } => send_pane(registry, &name, &text),
        Request::Read { name, lines } => read_pane(registry, &name, lines),
        Request::Kill { name } => kill_pane(registry, &name),
    }
}

fn spawn_pane(registry: &Registry, name: String, command: Vec<String>) -> Response {
    if command.is_empty() {
        return Response::err("command must not be empty");
    }
    if registry.lock().unwrap().contains_key(&name) {
        return Response::err(format!(
            "pane {name:?} already exists (kill it, then spawn under a new name)"
        ));
    }

    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 40,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(err) => return Response::err(format!("failed to open pty: {err}")),
    };

    let mut cmd = CommandBuilder::new(&command[0]);
    cmd.args(&command[1..]);

    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(child) => child,
        Err(err) => return Response::err(format!("failed to spawn {command:?}: {err}")),
    };
    let pid = child.process_id();
    drop(pair.slave);

    let reader = match pair.master.try_clone_reader() {
        Ok(reader) => reader,
        Err(err) => return Response::err(format!("failed to clone pty reader: {err}")),
    };
    let writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(err) => return Response::err(format!("failed to take pty writer: {err}")),
    };

    let output: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let exit_code: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let (kill_tx, kill_rx) = mpsc::channel::<()>();

    {
        let output = Arc::clone(&output);
        thread::spawn(move || read_loop(reader, output));
    }
    {
        let exit_code = Arc::clone(&exit_code);
        thread::spawn(move || loop {
            if let Ok(Some(status)) = child.try_wait() {
                *exit_code.lock().unwrap() = Some(status.exit_code());
                break;
            }
            if kill_rx.recv_timeout(Duration::from_millis(200)).is_ok() {
                let _ = child.kill();
            }
        });
    }

    let pane = Pane {
        command,
        writer,
        output,
        exit_code,
        kill_tx,
        _master: pair.master,
        pid,
    };
    registry.lock().unwrap().insert(name, pane);
    Response::ok()
}

fn read_loop(mut reader: Box<dyn Read + Send>, output: Arc<Mutex<Vec<u8>>>) {
    let mut chunk = [0u8; 4096];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let mut buf = output.lock().unwrap();
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > MAX_OUTPUT_BYTES {
                    let excess = buf.len() - MAX_OUTPUT_BYTES;
                    buf.drain(0..excess);
                }
            }
        }
    }
}

fn list_panes(registry: &Registry) -> Response {
    let guard = registry.lock().unwrap();
    let panes = guard
        .iter()
        .map(|(name, pane)| {
            let status = match *pane.exit_code.lock().unwrap() {
                Some(code) => PaneStatus::Exited { code },
                None => PaneStatus::Running,
            };
            PaneInfo {
                name: name.clone(),
                command: pane.command.clone(),
                status,
            }
        })
        .collect();
    Response::ok_with_panes(panes)
}

fn send_pane(registry: &Registry, name: &str, text: &str) -> Response {
    let mut guard = registry.lock().unwrap();
    match guard.get_mut(name) {
        Some(pane) => match pane.writer.write_all(text.as_bytes()) {
            Ok(()) => Response::ok(),
            Err(err) => Response::err(format!("write failed: {err}")),
        },
        None => Response::err(format!("no such pane: {name:?}")),
    }
}

fn read_pane(registry: &Registry, name: &str, lines: Option<usize>) -> Response {
    let guard = registry.lock().unwrap();
    let (text, pid) = match guard.get(name) {
        Some(pane) => {
            let buf = pane.output.lock().unwrap();
            (String::from_utf8_lossy(&buf).into_owned(), pane.pid)
        }
        None => return Response::err(format!("no such pane: {name:?}")),
    };
    drop(guard);

    let text = match lines {
        Some(n) => {
            let tail: Vec<&str> = text.lines().rev().take(n).collect();
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        }
        None => text,
    };
    let cwd = pid.and_then(process_cwd);
    Response::ok_with_output(text, cwd)
}

/// Best-effort lookup of a process's current working directory via `lsof`
/// (shipped on macOS) — there's no portable-pty API for this, and it's only
/// needed to show the pane's directory in the TUI, not for correctness.
/// Returns `None` if `lsof` is missing, the process already exited, or its
/// output doesn't parse — callers treat that as "unknown", not an error.
fn process_cwd(pid: u32) -> Option<String> {
    let output = std::process::Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n'))
        .map(str::to_string)
}

fn kill_pane(registry: &Registry, name: &str) -> Response {
    let guard = registry.lock().unwrap();
    match guard.get(name) {
        Some(pane) => {
            let _ = pane.kill_tx.send(());
            Response::ok()
        }
        None => Response::err(format!("no such pane: {name:?}")),
    }
}
