/// Turns raw PTY bytes (which may contain ANSI escape sequences, bare `\r`
/// line-redraws, and backspace bytes) into plain scrollback lines.
///
/// This is intentionally not a real terminal emulator — no cursor
/// positioning, no grid, no color. It only understands enough to make
/// ordinary shell usage (prompts, line editing, `\r`-redrawn progress) read
/// correctly in a linear scrollback: CSI/OSC sequences are stripped, `\r`
/// clears the current line instead of starting a new one, and backspace
/// removes the last byte of it.
pub fn render_pty_lines(raw: &[u8]) -> Vec<String> {
    let mut lines: Vec<Vec<u8>> = vec![Vec::new()];
    let mut i = 0;

    while i < raw.len() {
        match raw[i] {
            0x1b => {
                i += 1;
                match raw.get(i) {
                    Some(b'[') => {
                        i += 1;
                        while i < raw.len() && !raw[i].is_ascii_alphabetic() {
                            i += 1;
                        }
                        if i < raw.len() {
                            i += 1;
                        }
                    }
                    Some(b']') => {
                        i += 1;
                        while i < raw.len() && raw[i] != 0x07 {
                            if raw[i] == 0x1b && raw.get(i + 1) == Some(&b'\\') {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                        if raw.get(i) == Some(&0x07) {
                            i += 1;
                        }
                    }
                    Some(_) => i += 1,
                    None => {}
                }
            }
            b'\r' => {
                current(&mut lines).clear();
                i += 1;
            }
            b'\n' => {
                lines.push(Vec::new());
                i += 1;
            }
            0x7f | 0x08 => {
                current(&mut lines).pop();
                i += 1;
            }
            b => {
                current(&mut lines).push(b);
                i += 1;
            }
        }
    }

    lines
        .into_iter()
        .map(|line| String::from_utf8_lossy(&line).into_owned())
        .collect()
}

fn current(lines: &mut [Vec<u8>]) -> &mut Vec<u8> {
    lines.last_mut().expect("render_pty_lines always keeps at least one line")
}

/// Maps a pressed key (already decided to be forwarded to a pane) to the raw
/// bytes a real terminal would send for it. Returns `None` for keys with no
/// sensible raw-byte mapping (function keys, etc.) — those are dropped
/// rather than guessed at.
pub fn key_to_bytes(code: crossterm::event::KeyCode, ctrl: bool) -> Option<Vec<u8>> {
    use crossterm::event::KeyCode;

    if ctrl {
        if let KeyCode::Char(c) = code {
            let upper = c.to_ascii_uppercase();
            if upper.is_ascii_alphabetic() {
                return Some(vec![upper as u8 - b'A' + 1]);
            }
        }
    }

    match code {
        KeyCode::Char(c) => Some(c.to_string().into_bytes()),
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(b"\x7f".to_vec()),
        KeyCode::Tab => Some(b"\t".to_vec()),
        KeyCode::Esc => Some(b"\x1b".to_vec()),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_splits_on_newline() {
        assert_eq!(render_pty_lines(b"hello\nworld"), vec!["hello", "world"]);
    }

    #[test]
    fn carriage_return_clears_current_line() {
        // a shell redrawing "progress: 1%" then "progress: 99%" on one line
        assert_eq!(render_pty_lines(b"progress: 1%\rprogress: 99%"), vec!["progress: 99%"]);
    }

    #[test]
    fn backspace_erases_last_byte() {
        assert_eq!(render_pty_lines(b"helllo\x7f world"), vec!["helll world"]);
    }

    #[test]
    fn csi_sequences_are_stripped() {
        assert_eq!(render_pty_lines(b"\x1b[1;32mgreen\x1b[0m text"), vec!["green text"]);
    }

    #[test]
    fn osc_sequences_are_stripped() {
        let mut raw = b"\x1b]0;window title\x07visible".to_vec();
        assert_eq!(render_pty_lines(&raw), vec!["visible"]);
        raw = b"\x1b]0;window title\x1b\\visible".to_vec();
        assert_eq!(render_pty_lines(&raw), vec!["visible"]);
    }

    #[test]
    fn ctrl_c_maps_to_end_of_text_byte() {
        assert_eq!(
            key_to_bytes(crossterm::event::KeyCode::Char('c'), true),
            Some(vec![0x03])
        );
    }
}
