//! Sans-IO parser for the tmux control-mode protocol.
//!
//! Bytes go in through [`ControlParser::feed`]; typed [`Event`]s come out. The
//! parser never decodes pane output as UTF-8 — `%output` payloads are octal
//! unescaped and passed on as raw bytes, because a single notification can end
//! in the middle of a multi-byte sequence.

/// A parsed control-mode event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// `%output` / `%extended-output`: raw bytes written by the pane's program.
    Output { pane: u32, data: Vec<u8> },
    /// A complete `%begin … %end|%error` block. `from_client` is set when the
    /// command was sent by this client (flags bit 0).
    Reply {
        number: u64,
        from_client: bool,
        ok: bool,
        lines: Vec<String>,
    },
    LayoutChange {
        window: u32,
        layout: String,
        visible_layout: String,
        flags: String,
    },
    WindowAdd { window: u32 },
    WindowClose { window: u32 },
    WindowRenamed { window: u32, name: String },
    WindowPaneChanged { window: u32, pane: u32 },
    SessionChanged { session: u32, name: String },
    SessionRenamed { name: String },
    SessionWindowChanged { session: u32, window: u32 },
    SessionsChanged,
    PaneModeChanged { pane: u32 },
    Pause { pane: u32 },
    Continue { pane: u32 },
    ClientDetached,
    Exit { reason: Option<String> },
    /// Any notification this parser doesn't model (kept for diagnostics).
    Other { line: String },
}

/// Decodes tmux's octal escaping: every byte below 0x20 and `\` is written as
/// `\ooo`. Anything else passes through untouched.
pub fn decode_octal(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        if b == b'\\' && i + 3 < input.len() && is_octal_triplet(&input[i + 1..i + 4]) {
            let v = (input[i + 1] - b'0') as u32 * 64
                + (input[i + 2] - b'0') as u32 * 8
                + (input[i + 3] - b'0') as u32;
            out.push(v as u8);
            i += 4;
        } else {
            out.push(b);
            i += 1;
        }
    }
    out
}

fn is_octal_triplet(s: &[u8]) -> bool {
    s.len() == 3 && s.iter().all(|c| (b'0'..=b'7').contains(c))
}

/// Parses `%12`, `@3`, `$0` (or a bare number) into its numeric id.
pub fn parse_id(token: &str) -> Option<u32> {
    let t = token.trim_start_matches(['%', '@', '$']);
    t.parse().ok()
}

struct OpenBlock {
    number: u64,
    from_client: bool,
    lines: Vec<String>,
}

/// Incremental line-oriented parser. Handles both LF (pipe transport) and CRLF
/// (tmux -CC inside a pty) line endings.
#[derive(Default)]
pub struct ControlParser {
    buf: Vec<u8>,
    block: Option<OpenBlock>,
    exited: bool,
}

/// The result of feeding bytes: parsed events, plus any bytes that followed the
/// end of control mode (`%exit` + `ESC \`), which belong to the outer terminal.
#[derive(Default, Debug)]
pub struct FeedResult {
    pub events: Vec<Event>,
    pub trailing: Option<Vec<u8>>,
}

impl ControlParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// True once `%exit` has been seen.
    pub fn exited(&self) -> bool {
        self.exited
    }

    pub fn feed(&mut self, bytes: &[u8]) -> FeedResult {
        self.buf.extend_from_slice(bytes);
        let mut result = FeedResult::default();
        let mut start = 0;
        loop {
            if self.exited {
                // In-band mode: tmux terminates the DCS with ESC \ after %exit.
                let rest = &self.buf[start..];
                if rest.len() < 2 {
                    break;
                }
                let skip = if rest.starts_with(b"\x1b\\") { 2 } else { 0 };
                result.trailing = Some(rest[skip..].to_vec());
                start = self.buf.len();
                break;
            }
            let Some(nl) = self.buf[start..].iter().position(|&b| b == b'\n') else {
                break;
            };
            let end = start + nl;
            let mut line = &self.buf[start..end];
            if line.last() == Some(&b'\r') {
                line = &line[..line.len() - 1];
            }
            let line = line.to_vec();
            start = end + 1;
            if let Some(ev) = self.parse_line(&line) {
                result.events.push(ev);
            }
        }
        self.buf.drain(..start);
        result
    }

    fn parse_line(&mut self, line: &[u8]) -> Option<Event> {
        if let Some(block) = self.block.as_mut() {
            let is_end = line.starts_with(b"%end ");
            let is_error = line.starts_with(b"%error ");
            if is_end || is_error {
                let number = block_number(line);
                if number == Some(block.number) {
                    let block = self.block.take().unwrap();
                    return Some(Event::Reply {
                        number: block.number,
                        from_client: block.from_client,
                        ok: is_end,
                        lines: block.lines,
                    });
                }
            }
            block.lines.push(String::from_utf8_lossy(line).into_owned());
            return None;
        }

        if line.starts_with(b"%output ") {
            let rest = &line[b"%output ".len()..];
            let sp = rest.iter().position(|&b| b == b' ')?;
            let pane = parse_id(std::str::from_utf8(&rest[..sp]).ok()?)?;
            return Some(Event::Output {
                pane,
                data: decode_octal(&rest[sp + 1..]),
            });
        }
        if line.starts_with(b"%extended-output ") {
            // %extended-output %pane age ... : data
            let rest = &line[b"%extended-output ".len()..];
            let sp = rest.iter().position(|&b| b == b' ')?;
            let pane = parse_id(std::str::from_utf8(&rest[..sp]).ok()?)?;
            let colon = find_subslice(rest, b" : ")?;
            return Some(Event::Output {
                pane,
                data: decode_octal(&rest[colon + 3..]),
            });
        }

        let text = String::from_utf8_lossy(line).into_owned();
        let mut parts = text.splitn(2, ' ');
        let name = parts.next().unwrap_or("");
        let args = parts.next().unwrap_or("");
        let arg = |i: usize| args.split(' ').nth(i).unwrap_or("");

        let ev = match name {
            "%begin" => {
                let mut it = args.split(' ');
                let _time = it.next();
                let number = it.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                let flags: u32 = it.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                self.block = Some(OpenBlock {
                    number,
                    from_client: flags & 1 == 1,
                    lines: Vec::new(),
                });
                return None;
            }
            "%layout-change" => Event::LayoutChange {
                window: parse_id(arg(0))?,
                layout: arg(1).to_string(),
                visible_layout: arg(2).to_string(),
                flags: arg(3).to_string(),
            },
            "%window-add" => Event::WindowAdd { window: parse_id(arg(0))? },
            // kill-window reports the window as already unlinked; either way the
            // frontend drops it if it is one of ours.
            "%window-close" | "%unlinked-window-close" => Event::WindowClose { window: parse_id(arg(0))? },
            "%window-renamed" => Event::WindowRenamed {
                window: parse_id(arg(0))?,
                name: args.splitn(2, ' ').nth(1).unwrap_or("").to_string(),
            },
            "%window-pane-changed" => Event::WindowPaneChanged {
                window: parse_id(arg(0))?,
                pane: parse_id(arg(1))?,
            },
            "%session-changed" => Event::SessionChanged {
                session: parse_id(arg(0))?,
                name: args.splitn(2, ' ').nth(1).unwrap_or("").to_string(),
            },
            "%session-renamed" => Event::SessionRenamed {
                name: args.splitn(2, ' ').nth(1).unwrap_or(args).to_string(),
            },
            "%session-window-changed" => Event::SessionWindowChanged {
                session: parse_id(arg(0))?,
                window: parse_id(arg(1))?,
            },
            "%sessions-changed" => Event::SessionsChanged,
            "%pane-mode-changed" => Event::PaneModeChanged { pane: parse_id(arg(0))? },
            "%pause" => Event::Pause { pane: parse_id(arg(0))? },
            "%continue" => Event::Continue { pane: parse_id(arg(0))? },
            "%client-detached" => Event::ClientDetached,
            "%exit" => {
                self.exited = true;
                Event::Exit {
                    reason: if args.is_empty() { None } else { Some(args.to_string()) },
                }
            }
            _ => Event::Other { line: text },
        };
        Some(ev)
    }
}

fn block_number(line: &[u8]) -> Option<u64> {
    let text = std::str::from_utf8(line).ok()?;
    text.split(' ').nth(2)?.parse().ok()
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Detects the `ESC P 1000 p` sequence that `tmux -CC` prints when it enters
/// control mode inside a terminal. Holds back a possible partial match at the
/// end of a chunk so the sequence is found even when split across reads.
#[derive(Default)]
pub struct DcsDetector {
    held: Vec<u8>,
}

pub const CONTROL_MODE_DCS: &[u8] = b"\x1bP1000p";

pub enum Detect {
    /// No control mode start: forward these bytes to the terminal.
    Passthrough(Vec<u8>),
    /// Control mode started: `before` goes to the terminal, `after` to the parser.
    Started { before: Vec<u8>, after: Vec<u8> },
}

impl DcsDetector {
    pub fn feed(&mut self, bytes: &[u8]) -> Detect {
        let mut data = std::mem::take(&mut self.held);
        data.extend_from_slice(bytes);
        if let Some(pos) = find_subslice(&data, CONTROL_MODE_DCS) {
            let after = data[pos + CONTROL_MODE_DCS.len()..].to_vec();
            data.truncate(pos);
            return Detect::Started { before: data, after };
        }
        // Hold back the longest suffix that is a prefix of the DCS.
        let max = CONTROL_MODE_DCS.len().saturating_sub(1).min(data.len());
        for k in (1..=max).rev() {
            if data[data.len() - k..] == CONTROL_MODE_DCS[..k] {
                self.held = data.split_off(data.len() - k);
                break;
            }
        }
        Detect::Passthrough(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed_all(p: &mut ControlParser, s: &[u8]) -> Vec<Event> {
        p.feed(s).events
    }

    #[test]
    fn decodes_octal_escapes() {
        assert_eq!(decode_octal(br"a\033[1mb\134c\015\012"), b"a\x1b[1mb\\c\r\n");
        // UTF-8 passes through raw.
        assert_eq!(decode_octal("─ok".as_bytes()), "─ok".as_bytes());
        // A lone backslash without a valid triplet is kept verbatim.
        assert_eq!(decode_octal(br"x\9"), br"x\9");
    }

    #[test]
    fn parses_output_and_extended_output() {
        let mut p = ControlParser::new();
        let evs = feed_all(
            &mut p,
            b"%output %3 hi\\015\\012\n%extended-output %4 12 : x\\033y\n",
        );
        assert_eq!(
            evs,
            vec![
                Event::Output { pane: 3, data: b"hi\r\n".to_vec() },
                Event::Output { pane: 4, data: b"x\x1by".to_vec() },
            ]
        );
    }

    #[test]
    fn collects_reply_blocks_and_flags() {
        let mut p = ControlParser::new();
        let evs = feed_all(
            &mut p,
            b"%begin 1 277 0\n%end 1 277 0\n%begin 1 282 1\nA 2\n%output %1 not-a-notification\n%end 1 282 1\n%begin 1 283 1\nparse error\n%error 1 283 1\n",
        );
        assert_eq!(
            evs,
            vec![
                Event::Reply { number: 277, from_client: false, ok: true, lines: vec![] },
                Event::Reply {
                    number: 282,
                    from_client: true,
                    ok: true,
                    lines: vec!["A 2".into(), "%output %1 not-a-notification".into()],
                },
                Event::Reply {
                    number: 283,
                    from_client: true,
                    ok: false,
                    lines: vec!["parse error".into()],
                },
            ]
        );
    }

    #[test]
    fn handles_crlf_and_split_reads() {
        let mut p = ControlParser::new();
        let mut evs = feed_all(&mut p, b"%layout-change @0 8205,80x24,0,0{40x24,0,0,0,39x24,41");
        assert!(evs.is_empty());
        evs = feed_all(&mut p, b",0,1} 8205,80x24,0,0{40x24,0,0,0,39x24,41,0,1} *\r\n%window-renamed @0 my shell\r\n");
        assert_eq!(
            evs,
            vec![
                Event::LayoutChange {
                    window: 0,
                    layout: "8205,80x24,0,0{40x24,0,0,0,39x24,41,0,1}".into(),
                    visible_layout: "8205,80x24,0,0{40x24,0,0,0,39x24,41,0,1}".into(),
                    flags: "*".into(),
                },
                Event::WindowRenamed { window: 0, name: "my shell".into() },
            ]
        );
    }

    #[test]
    fn parses_structural_notifications() {
        let mut p = ControlParser::new();
        let evs = feed_all(
            &mut p,
            b"%window-add @2\n%window-close @2\n%unlinked-window-close @3\n%window-pane-changed @0 %1\n%session-changed $0 probe\n%session-window-changed $0 @1\n%sessions-changed\n%pane-mode-changed %1\n%pause %1\n%continue %1\n%unknown thing\n",
        );
        assert_eq!(
            evs,
            vec![
                Event::WindowAdd { window: 2 },
                Event::WindowClose { window: 2 },
                Event::WindowClose { window: 3 },
                Event::WindowPaneChanged { window: 0, pane: 1 },
                Event::SessionChanged { session: 0, name: "probe".into() },
                Event::SessionWindowChanged { session: 0, window: 1 },
                Event::SessionsChanged,
                Event::PaneModeChanged { pane: 1 },
                Event::Pause { pane: 1 },
                Event::Continue { pane: 1 },
                Event::Other { line: "%unknown thing".into() },
            ]
        );
    }

    #[test]
    fn exit_returns_trailing_terminal_bytes() {
        let mut p = ControlParser::new();
        let r = p.feed(b"%exit\r\n\x1b\\$ prompt");
        assert_eq!(r.events, vec![Event::Exit { reason: None }]);
        assert_eq!(r.trailing.as_deref(), Some(&b"$ prompt"[..]));
        assert!(p.exited());
    }

    #[test]
    fn exit_waits_for_string_terminator() {
        let mut p = ControlParser::new();
        let r = p.feed(b"%exit detached\r\n\x1b");
        assert_eq!(r.events, vec![Event::Exit { reason: Some("detached".into()) }]);
        assert!(r.trailing.is_none());
        let r = p.feed(b"\\");
        assert_eq!(r.trailing.as_deref(), Some(&b""[..]));
    }

    #[test]
    fn dcs_detector_finds_split_sequence() {
        let mut d = DcsDetector::default();
        match d.feed(b"hello \x1bP10") {
            Detect::Passthrough(b) => assert_eq!(b, b"hello "),
            _ => panic!("unexpected start"),
        }
        match d.feed(b"00p%begin") {
            Detect::Started { before, after } => {
                assert!(before.is_empty());
                assert_eq!(after, b"%begin");
            }
            _ => panic!("missed start"),
        }
    }

    #[test]
    fn dcs_detector_releases_non_matching_hold() {
        let mut d = DcsDetector::default();
        match d.feed(b"abc\x1b") {
            Detect::Passthrough(b) => assert_eq!(b, b"abc"),
            _ => panic!(),
        }
        match d.feed(b"[1m") {
            Detect::Passthrough(b) => assert_eq!(b, b"\x1b[1m"),
            _ => panic!(),
        }
    }
}
