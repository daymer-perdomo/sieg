/// Turns raw PTY bytes (which may contain ANSI escape sequences, bare `\r`
/// line-redraws, and backspace bytes) into styled scrollback lines.
///
/// This is intentionally not a real terminal emulator — no cursor
/// positioning, no grid. It only understands enough to make ordinary shell
/// usage (prompts, line editing, `\r`-redrawn progress, basic color) read
/// correctly in a linear scrollback: CSI/OSC sequences are stripped except
/// for SGR (color/bold) ones, `\r` clears the current line instead of
/// starting a new one, and backspace removes the last character of it.
/// One basic set of 8 ANSI colors is understood (no 256-color/truecolor) —
/// enough for a shell prompt to highlight the current directory.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TextStyle {
    pub color: Option<AnsiColor>,
    pub bold: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: TextStyle,
}

pub fn render_pty_lines(raw: &[u8]) -> Vec<Vec<Span>> {
    // Each line is a run-length-encoded sequence of (bytes, style) chunks —
    // adjacent bytes sharing a style are appended to the same chunk instead
    // of allocating one per byte.
    let mut lines: Vec<Vec<(Vec<u8>, TextStyle)>> = vec![Vec::new()];
    let mut style = TextStyle::default();
    let mut i = 0;

    while i < raw.len() {
        match raw[i] {
            0x1b => {
                i += 1;
                match raw.get(i) {
                    Some(b'[') => {
                        i += 1;
                        let params_start = i;
                        while i < raw.len() && !raw[i].is_ascii_alphabetic() {
                            i += 1;
                        }
                        let final_byte = raw.get(i).copied();
                        let params = &raw[params_start..i.min(raw.len())];
                        if i < raw.len() {
                            i += 1;
                        }
                        if final_byte == Some(b'm') {
                            apply_sgr(&mut style, params);
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
            // A PTY ends every ordinary line with "\r\n" (CR then LF), not
            // just "\n" — that CR must NOT erase the line it's terminating.
            // Only a bare `\r` (no following `\n`) is a real redraw, e.g. a
            // progress bar overwriting itself in place.
            b'\r' if raw.get(i + 1) == Some(&b'\n') => {
                i += 1;
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
                pop_byte(&mut lines);
                i += 1;
            }
            // Expand to the next 8-column tab stop, like a real terminal —
            // otherwise `ls`'s tab-separated columns render mashed together
            // (ratatui doesn't interpret raw \t itself).
            b'\t' => {
                let col = line_len(&mut lines);
                let spaces = 8 - (col % 8);
                for _ in 0..spaces {
                    push_byte(&mut lines, style, b' ');
                }
                i += 1;
            }
            b => {
                push_byte(&mut lines, style, b);
                i += 1;
            }
        }
    }

    lines
        .into_iter()
        .map(|line| {
            line.into_iter()
                .map(|(bytes, style)| Span {
                    text: String::from_utf8_lossy(&bytes).into_owned(),
                    style,
                })
                .collect()
        })
        .collect()
}

fn current(lines: &mut [Vec<(Vec<u8>, TextStyle)>]) -> &mut Vec<(Vec<u8>, TextStyle)> {
    lines.last_mut().expect("render_pty_lines always keeps at least one line")
}

fn line_len(lines: &mut [Vec<(Vec<u8>, TextStyle)>]) -> usize {
    current(lines).iter().map(|(bytes, _)| bytes.len()).sum()
}

fn push_byte(lines: &mut [Vec<(Vec<u8>, TextStyle)>], style: TextStyle, b: u8) {
    let line = current(lines);
    match line.last_mut() {
        Some((bytes, s)) if *s == style => bytes.push(b),
        _ => line.push((vec![b], style)),
    }
}

fn pop_byte(lines: &mut [Vec<(Vec<u8>, TextStyle)>]) {
    let line = current(lines);
    if let Some((bytes, _)) = line.last_mut() {
        bytes.pop();
        if bytes.is_empty() {
            line.pop();
        }
    }
}

/// Applies an SGR ("Select Graphic Rendition") parameter list — the part of
/// `ESC [ params m` between `[` and `m` — to the running text style. Only
/// reset, bold, and the 16 standard/bright foreground colors are understood;
/// background colors, underline, 256-color and truecolor codes are silently
/// ignored (no bright/dim distinction in the catppuccin palette either).
fn apply_sgr(style: &mut TextStyle, params: &[u8]) {
    let text = std::str::from_utf8(params).unwrap_or("");
    let codes = if text.is_empty() {
        vec![0]
    } else {
        text.split(';').map(|p| p.parse::<u32>().unwrap_or(0)).collect()
    };

    for code in codes {
        match code {
            0 => *style = TextStyle::default(),
            1 => style.bold = true,
            22 => style.bold = false,
            30 | 90 => style.color = Some(AnsiColor::Black),
            31 | 91 => style.color = Some(AnsiColor::Red),
            32 | 92 => style.color = Some(AnsiColor::Green),
            33 | 93 => style.color = Some(AnsiColor::Yellow),
            34 | 94 => style.color = Some(AnsiColor::Blue),
            35 | 95 => style.color = Some(AnsiColor::Magenta),
            36 | 96 => style.color = Some(AnsiColor::Cyan),
            37 | 97 => style.color = Some(AnsiColor::White),
            39 => style.color = None,
            _ => {}
        }
    }
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

    /// Flattens a rendered line's spans back to plain text, for assertions
    /// that don't care about style.
    fn plain(line: &[Span]) -> String {
        line.iter().map(|s| s.text.as_str()).collect()
    }

    fn plain_lines(raw: &[u8]) -> Vec<String> {
        render_pty_lines(raw).iter().map(|line| plain(line)).collect()
    }

    #[test]
    fn plain_text_splits_on_newline() {
        assert_eq!(plain_lines(b"hello\nworld"), vec!["hello", "world"]);
    }

    #[test]
    fn carriage_return_clears_current_line() {
        // a shell redrawing "progress: 1%" then "progress: 99%" on one line
        assert_eq!(plain_lines(b"progress: 1%\rprogress: 99%"), vec!["progress: 99%"]);
    }

    #[test]
    fn crlf_line_ending_keeps_the_line() {
        // a real PTY terminates ordinary lines with "\r\n", not just "\n" —
        // that \r must not erase the line it's ending
        assert_eq!(
            plain_lines(b"bash-3.2$ ls\r\nfile.txt\r\nbash-3.2$ "),
            vec!["bash-3.2$ ls", "file.txt", "bash-3.2$ "]
        );
    }

    #[test]
    fn tabs_expand_to_the_next_stop() {
        // `ls` separates columns with raw tabs; ratatui won't expand them on
        // its own, so columns would render mashed together otherwise
        assert_eq!(plain_lines(b"a\tbb\tccc\td"), vec!["a       bb      ccc     d"]);
    }

    #[test]
    fn backspace_erases_last_byte() {
        assert_eq!(plain_lines(b"helllo\x7f world"), vec!["helll world"]);
    }

    #[test]
    fn csi_sequences_other_than_color_are_stripped() {
        assert_eq!(plain_lines(b"\x1b[2Kcleared\x1b[1;1H text"), vec!["cleared text"]);
    }

    #[test]
    fn osc_sequences_are_stripped() {
        let mut raw = b"\x1b]0;window title\x07visible".to_vec();
        assert_eq!(plain_lines(&raw), vec!["visible"]);
        raw = b"\x1b]0;window title\x1b\\visible".to_vec();
        assert_eq!(plain_lines(&raw), vec!["visible"]);
    }

    #[test]
    fn sgr_color_becomes_a_styled_span() {
        let lines = render_pty_lines(b"\x1b[36m~/Documents\x1b[0m $");
        assert_eq!(
            lines[0],
            vec![
                Span {
                    text: "~/Documents".to_string(),
                    style: TextStyle { color: Some(AnsiColor::Cyan), bold: false },
                },
                Span {
                    text: " $".to_string(),
                    style: TextStyle::default(),
                },
            ]
        );
    }

    #[test]
    fn sgr_reset_clears_bold_and_color_together() {
        let lines = render_pty_lines(b"\x1b[1;32mbold green\x1b[0mplain");
        assert_eq!(
            lines[0],
            vec![
                Span {
                    text: "bold green".to_string(),
                    style: TextStyle { color: Some(AnsiColor::Green), bold: true },
                },
                Span {
                    text: "plain".to_string(),
                    style: TextStyle::default(),
                },
            ]
        );
    }

    #[test]
    fn ctrl_c_maps_to_end_of_text_byte() {
        assert_eq!(
            key_to_bytes(crossterm::event::KeyCode::Char('c'), true),
            Some(vec![0x03])
        );
    }
}
