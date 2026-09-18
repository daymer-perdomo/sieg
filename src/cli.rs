use crate::client::send_request;
use crate::protocol::{PaneStatus, Request, Response};

pub fn print_help() {
    println!("sieg — visual TUI mockup plus a minimal real background pane manager\n");
    println!("usage:");
    println!("  sieg                            launch the visual TUI mockup");
    println!("  sieg spawn <name> -- <cmd...>   spawn a real background pane");
    println!("  sieg list                       list panes and their status");
    println!("  sieg send <name> <text...>      send input to a pane");
    println!("  sieg read <name> [lines]        read a pane's buffered output");
    println!("  sieg kill <name>                terminate a pane's process");
}

fn usage_exit(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(2);
}

fn fail(response: Response) -> ! {
    eprintln!("error: {}", response.error.unwrap_or_else(|| "unknown error".into()));
    std::process::exit(1);
}

pub fn spawn(args: &[String]) -> std::io::Result<()> {
    if args.is_empty() {
        usage_exit("usage: sieg spawn <name> -- <command...>");
    }
    let name = args[0].clone();
    let rest = &args[1..];
    let command: Vec<String> = match rest.first().map(String::as_str) {
        Some("--") => rest[1..].to_vec(),
        _ => rest.to_vec(),
    };
    if command.is_empty() {
        usage_exit("usage: sieg spawn <name> -- <command...>");
    }

    let response = send_request(&Request::Spawn {
        name: name.clone(),
        command,
    })?;
    if !response.ok {
        fail(response);
    }
    println!("spawned pane {name:?}");
    Ok(())
}

pub fn list() -> std::io::Result<()> {
    let response = send_request(&Request::List)?;
    if !response.ok {
        fail(response);
    }
    if response.panes.is_empty() {
        println!("no panes");
        return Ok(());
    }
    println!("{:<20} {:<14} {}", "NAME", "STATUS", "COMMAND");
    for pane in response.panes {
        let status = match pane.status {
            PaneStatus::Running => "running".to_string(),
            PaneStatus::Exited { code } => format!("exited({code})"),
        };
        println!("{:<20} {:<14} {}", pane.name, status, pane.command.join(" "));
    }
    Ok(())
}

pub fn send(args: &[String]) -> std::io::Result<()> {
    if args.len() < 2 {
        usage_exit("usage: sieg send <name> <text...>");
    }
    let name = args[0].clone();
    let mut text = args[1..].join(" ");
    text.push('\n');

    let response = send_request(&Request::Send { name, text })?;
    if !response.ok {
        fail(response);
    }
    Ok(())
}

pub fn read(args: &[String]) -> std::io::Result<()> {
    if args.is_empty() {
        usage_exit("usage: sieg read <name> [lines]");
    }
    let name = args[0].clone();
    let lines = args.get(1).and_then(|s| s.parse::<usize>().ok());

    let response = send_request(&Request::Read { name, lines })?;
    if !response.ok {
        fail(response);
    }
    print!("{}", response.output.unwrap_or_default());
    Ok(())
}

pub fn kill(args: &[String]) -> std::io::Result<()> {
    if args.is_empty() {
        usage_exit("usage: sieg kill <name>");
    }
    let name = args[0].clone();

    let response = send_request(&Request::Kill { name })?;
    if !response.ok {
        fail(response);
    }
    Ok(())
}
