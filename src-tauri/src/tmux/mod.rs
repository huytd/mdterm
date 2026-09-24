//! tmux control-mode integration.
//!
//! A connection is a byte stream speaking the control-mode protocol, reached
//! through one of two transports:
//!
//! * **Spawned** — `tmux -C attach|new-session` over pipes (launch-time attach).
//! * **In-band** — a user ran `tmux -CC …` inside an mdterm shell (locally or
//!   over ssh); the PTY reader detects `ESC P1000p` and diverts the stream here.
//!
//! The backend stays thin: it parses the protocol, correlates command replies
//! and forwards everything to the frontend over one ordered Tauri channel.
//! Pane output and replies share that channel, so the frontend sees them in the
//! exact order tmux produced them (needed to seed panes from `capture-pane`
//! without losing or duplicating output).
//!
//! Frame format on the channel:
//! * `[0x01][pane: u32 LE][bytes…]` — pane output
//! * `[0x02][json…]` — everything else (see [`event_json`])

pub mod parser;

use parser::{ControlParser, Event};
use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, Manager, State};

pub type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;

const FRAME_OUTPUT: u8 = 1;
const FRAME_JSON: u8 = 2;
/// Bytes per `send-keys -H` command.
const SEND_KEYS_CHUNK: usize = 256;

pub struct Conn {
    writer: SharedWriter,
    /// Tags of commands sent by the frontend, in send order. tmux answers in
    /// order, so each client-originated reply block pops the front. 0 = untracked.
    pending: VecDeque<u64>,
    sink: Option<Channel<InvokeResponseBody>>,
    backlog: Vec<Vec<u8>>,
    child: Option<std::process::Child>,
    origin: Option<String>,
    session: Option<String>,
}

pub type ConnHandle = Arc<Mutex<Conn>>;

#[derive(Default)]
pub struct TmuxState {
    conns: Mutex<HashMap<u32, ConnHandle>>,
    next_id: AtomicU32,
}

impl TmuxState {
    fn get(&self, id: u32) -> Result<ConnHandle, String> {
        self.conns
            .lock()
            .map_err(|_| "tmux state poisoned".to_string())?
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("No tmux connection {}", id))
    }

    pub fn has_connections(&self) -> bool {
        self.conns.lock().map(|c| !c.is_empty()).unwrap_or(false)
    }

    fn register(&self, conn: Conn) -> (u32, ConnHandle) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let handle = Arc::new(Mutex::new(conn));
        if let Ok(mut map) = self.conns.lock() {
            map.insert(id, handle.clone());
        }
        (id, handle)
    }

    fn remove(&self, id: u32) -> Option<ConnHandle> {
        self.conns.lock().ok()?.remove(&id)
    }
}

impl Conn {
    fn new(writer: SharedWriter, origin: Option<String>, session: Option<String>) -> Self {
        Self {
            writer,
            pending: VecDeque::new(),
            sink: None,
            backlog: Vec::new(),
            child: None,
            origin,
            session,
        }
    }

    fn deliver(&mut self, frame: Vec<u8>) {
        match &self.sink {
            Some(ch) => {
                let _ = ch.send(InvokeResponseBody::Raw(frame));
            }
            None => self.backlog.push(frame),
        }
    }

    /// Writes one command line (several commands joined by ` ; ` produce one
    /// reply block each, so each gets its own tag).
    fn write_commands(&mut self, commands: &[String], tags: &[u64]) -> Result<(), String> {
        if commands.is_empty() {
            return Ok(());
        }
        if commands.iter().any(|c| c.contains('\n') || c.contains('\r')) {
            return Err("tmux commands must be single-line".into());
        }
        for i in 0..commands.len() {
            self.pending.push_back(tags.get(i).copied().unwrap_or(0));
        }
        let mut line = commands.join(" ; ");
        line.push('\n');
        let mut w = self.writer.lock().map_err(|_| "tmux writer poisoned".to_string())?;
        w.write_all(line.as_bytes())
            .and_then(|_| w.flush())
            .map_err(|e| format!("tmux write failed: {}", e))
    }
}

fn output_frame(pane: u32, data: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(5 + data.len());
    f.push(FRAME_OUTPUT);
    f.extend_from_slice(&pane.to_le_bytes());
    f.extend_from_slice(data);
    f
}

fn json_frame(v: &serde_json::Value) -> Vec<u8> {
    let mut f = vec![FRAME_JSON];
    f.extend_from_slice(v.to_string().as_bytes());
    f
}

/// JSON form of a non-output event. Replies carry the frontend's tag.
fn event_json(ev: &Event, tag: Option<u64>) -> Option<serde_json::Value> {
    Some(match ev {
        Event::Output { .. } => return None,
        Event::Reply { ok, lines, .. } => json!({"t": "reply", "tag": tag.unwrap_or(0), "ok": ok, "lines": lines}),
        Event::LayoutChange { window, layout, visible_layout, flags } => json!({
            "t": "layout",
            "window": window,
            "layout": layout,
            "visible": visible_layout,
            "flags": flags,
        }),
        Event::WindowAdd { window } => json!({"t": "window-add", "window": window}),
        Event::WindowClose { window } => json!({"t": "window-close", "window": window}),
        Event::WindowRenamed { window, name } => json!({"t": "window-renamed", "window": window, "name": name}),
        Event::WindowPaneChanged { window, pane } => json!({"t": "window-pane-changed", "window": window, "pane": pane}),
        Event::SessionChanged { session, name } => json!({"t": "session-changed", "session": session, "name": name}),
        Event::SessionRenamed { name } => json!({"t": "session-renamed", "name": name}),
        Event::SessionWindowChanged { session, window } => json!({"t": "session-window-changed", "session": session, "window": window}),
        Event::SessionsChanged => json!({"t": "sessions-changed"}),
        Event::PaneModeChanged { pane } => json!({"t": "pane-mode-changed", "pane": pane}),
        Event::Pause { pane } => json!({"t": "pause", "pane": pane}),
        Event::Continue { pane } => json!({"t": "continue", "pane": pane}),
        Event::ClientDetached => json!({"t": "client-detached"}),
        Event::Exit { reason } => json!({"t": "exit", "reason": reason}),
        Event::Other { .. } => return None,
    })
}

/// Turns parsed events into frames, merging consecutive output for the same
/// pane, correlating replies with tags, and answering flow-control pauses.
fn dispatch(conn: &ConnHandle, events: Vec<Event>) {
    let Ok(mut c) = conn.lock() else { return };
    let mut run: Option<(u32, Vec<u8>)> = None;
    for ev in events {
        if let Event::Output { pane, data } = ev {
            match run.as_mut() {
                Some((p, buf)) if *p == pane => buf.extend_from_slice(&data),
                _ => {
                    if let Some((p, buf)) = run.take() {
                        c.deliver(output_frame(p, &buf));
                    }
                    run = Some((pane, data));
                }
            }
            continue;
        }
        if let Some((p, buf)) = run.take() {
            c.deliver(output_frame(p, &buf));
        }
        let mut tag = None;
        match &ev {
            Event::Reply { from_client: true, .. } => tag = c.pending.pop_front(),
            Event::Pause { pane } => {
                let _ = c.write_commands(&[format!("refresh-client -A '%{}:continue'", pane)], &[0]);
            }
            _ => {}
        }
        // Untracked replies (keystrokes, auto-continue) are not forwarded.
        if matches!(ev, Event::Reply { .. }) && tag.unwrap_or(0) == 0 {
            continue;
        }
        if let Some(v) = event_json(&ev, tag) {
            c.deliver(json_frame(&v));
        }
    }
    if let Some((p, buf)) = run.take() {
        c.deliver(output_frame(p, &buf));
    }
}

/// Where connections live: the Tauri app in production, a fake in tests.
pub trait ConnectionHost {
    /// Registers a connection and tells the frontend about it.
    fn open(&self, conn: Conn) -> Option<(u32, ConnHandle)>;
    /// Forgets a finished connection.
    fn forget(&self, id: u32);
}

impl ConnectionHost for AppHandle {
    fn open(&self, conn: Conn) -> Option<(u32, ConnHandle)> {
        let state = self.try_state::<TmuxState>()?;
        let (origin, session) = (conn.origin.clone(), conn.session.clone());
        let (id, handle) = state.register(conn);
        announce(self, id, origin, session);
        Some((id, handle))
    }

    fn forget(&self, id: u32) {
        if let Some(state) = self.try_state::<TmuxState>() {
            state.remove(id);
        }
    }
}

fn finish(host: &dyn ConnectionHost, id: u32, conn: &ConnHandle, exited: bool) {
    if let Ok(mut c) = conn.lock() {
        if !exited {
            c.deliver(json_frame(&json!({"t": "exit", "reason": "connection closed"})));
        }
        if let Some(mut child) = c.child.take() {
            let _ = child.wait();
        }
    }
    host.forget(id);
}

#[derive(Serialize, Clone)]
struct ConnectionAnnounce {
    conn_id: u32,
    origin: Option<String>,
    session: Option<String>,
}

fn announce(app: &AppHandle, id: u32, origin: Option<String>, session: Option<String>) {
    let _ = app.emit("tmux-connection", ConnectionAnnounce { conn_id: id, origin, session });
}

// ---------------------------------------------------------------------------
// In-band transport (tmux -CC inside a PTY)
// ---------------------------------------------------------------------------

/// Per-PTY state for detecting and running an in-band control-mode session.
pub struct InbandTracker {
    pub session_id: String,
    writer: SharedWriter,
    detector: parser::DcsDetector,
    active: Option<(u32, ConnHandle, ControlParser)>,
    control_flag: Arc<std::sync::atomic::AtomicBool>,
}

impl InbandTracker {
    pub fn new(
        session_id: String,
        writer: SharedWriter,
        control_flag: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            session_id,
            writer,
            detector: parser::DcsDetector::default(),
            active: None,
            control_flag,
        }
    }

    /// Processes a chunk of PTY output. Returns the bytes that belong to the
    /// terminal emulator (everything outside control mode).
    pub fn process(&mut self, host: &dyn ConnectionHost, bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut input = bytes.to_vec();
        loop {
            if let Some((id, conn, parser)) = self.active.as_mut() {
                let r = parser.feed(&input);
                dispatch(conn, r.events);
                match r.trailing {
                    Some(rest) => {
                        let (id, conn) = (*id, conn.clone());
                        self.active = None;
                        self.control_flag.store(false, Ordering::SeqCst);
                        finish(host, id, &conn, true);
                        input = rest;
                        continue;
                    }
                    None => return out,
                }
            }
            match self.detector.feed(&input) {
                parser::Detect::Passthrough(b) => {
                    out.extend_from_slice(&b);
                    return out;
                }
                parser::Detect::Started { before, after } => {
                    out.extend_from_slice(&before);
                    let conn = Conn::new(self.writer.clone(), Some(self.session_id.clone()), None);
                    let Some((id, handle)) = host.open(conn) else {
                        out.extend_from_slice(&after);
                        return out;
                    };
                    self.control_flag.store(true, Ordering::SeqCst);
                    self.active = Some((id, handle, ControlParser::new()));
                    input = after;
                }
            }
        }
    }

    /// Called when the PTY closes while control mode may still be active.
    pub fn close(&mut self, host: &dyn ConnectionHost) {
        if let Some((id, conn, parser)) = self.active.take() {
            finish(host, id, &conn, parser.exited());
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct SessionInfo {
    name: String,
    windows: u32,
    attached: u32,
    activity: u64,
}

#[derive(Serialize)]
pub struct TmuxInfo {
    installed: bool,
    version: Option<String>,
    supported: bool,
    sessions: Vec<SessionInfo>,
    integration: String,
    session: String,
}

/// `3.4` → (3, 4); `next-3.5` / `3.5a` handled; `master` → None.
pub fn parse_version(v: &str) -> Option<(u32, u32)> {
    let num = v.trim().rsplit(' ').next()?;
    let num = num.trim_start_matches(|c: char| !c.is_ascii_digit());
    let mut it = num.split('.');
    let major = it.next()?.parse().ok()?;
    let minor_str: String = it.next().unwrap_or("0").chars().take_while(|c| c.is_ascii_digit()).collect();
    Some((major, minor_str.parse().unwrap_or(0)))
}

pub fn version_supported(v: &str) -> bool {
    // 3.2 introduced pause-after flow control; builds without a number are accepted.
    parse_version(v).map_or(true, |ver| ver >= (3, 2))
}

fn tmux_process() -> Command {
    let mut cmd = Command::new("tmux");
    for (k, v) in crate::pty_stream::shell_env() {
        if k != "TERM" {
            cmd.env(k, v);
        }
    }
    cmd
}

#[tauri::command]
pub fn tmux_detect(state: State<TmuxState>) -> TmuxInfo {
    let cfg = crate::config::load_terminal_config();
    let tmux_cfg = cfg.tmux.unwrap_or_default();
    let mut info = TmuxInfo {
        installed: false,
        version: None,
        supported: false,
        sessions: vec![],
        integration: tmux_cfg.integration.unwrap_or_else(|| "ask".into()),
        session: tmux_cfg.session.unwrap_or_default(),
    };
    let Ok(out) = tmux_process().arg("-V").output() else { return info };
    if !out.status.success() {
        return info;
    }
    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    info.installed = true;
    info.supported = version_supported(&version);
    info.version = Some(version);
    // Avoid talking to the server out-of-band while a control client is attached.
    if state.has_connections() {
        return info;
    }
    if let Ok(out) = tmux_process()
        .args(["list-sessions", "-F", "#{session_name}\t#{session_windows}\t#{session_attached}\t#{session_activity}"])
        .output()
    {
        if out.status.success() {
            info.sessions = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| {
                    let f: Vec<&str> = l.split('\t').collect();
                    (f.len() >= 4).then(|| SessionInfo {
                        name: f[0].to_string(),
                        windows: f[1].parse().unwrap_or(0),
                        attached: f[2].parse().unwrap_or(0),
                        activity: f[3].parse().unwrap_or(0),
                    })
                })
                .collect();
            info.sessions.sort_by(|a, b| b.activity.cmp(&a.activity));
        }
    }
    info
}

/// Spawns `tmux -C` and registers the connection. The frontend learns about it
/// through the `tmux-connection` event and then calls [`tmux_subscribe`].
#[tauri::command]
pub fn tmux_attach(
    app: AppHandle,
    session: Option<String>,
    create: Option<bool>,
) -> Result<u32, String> {
    let session = session.filter(|s| !s.trim().is_empty());
    let mut cmd = tmux_process();
    cmd.arg("-C");
    match (&session, create.unwrap_or(false)) {
        (Some(name), true) => cmd.args(["new-session", "-A", "-s", name]),
        (Some(name), false) => cmd.args(["attach-session", "-t", name]),
        (None, true) => cmd.arg("new-session"),
        (None, false) => cmd.arg("attach-session"),
    };
    if let Ok(dir) = std::env::current_dir() {
        cmd.current_dir(dir);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start tmux: {}", e))?;
    let stdin = child.stdin.take().ok_or("tmux stdin unavailable")?;
    let stdout = child.stdout.take().ok_or("tmux stdout unavailable")?;
    let writer: SharedWriter = Arc::new(Mutex::new(Box::new(stdin)));
    let mut conn = Conn::new(writer, None, session.clone());
    conn.child = Some(child);
    let (id, handle) = app.open(conn).ok_or("tmux state unavailable")?;
    std::thread::spawn(move || run_reader(&app, id, handle, stdout));
    Ok(id)
}

fn run_reader<R: Read + Send + 'static>(host: &dyn ConnectionHost, id: u32, conn: ConnHandle, reader: R) {
    let mut parser = ControlParser::new();
    crate::pty_stream::pump(reader, 65536, |bytes| {
        let r = parser.feed(bytes);
        dispatch(&conn, r.events);
    });
    finish(host, id, &conn, parser.exited());
}

#[derive(Serialize)]
pub struct SubscribeInfo {
    origin: Option<String>,
    session: Option<String>,
}

#[tauri::command]
pub fn tmux_subscribe(
    state: State<TmuxState>,
    conn_id: u32,
    on_event: Channel<InvokeResponseBody>,
) -> Result<SubscribeInfo, String> {
    let conn = state.get(conn_id)?;
    let mut c = conn.lock().map_err(|_| "tmux connection poisoned".to_string())?;
    for frame in c.backlog.drain(..) {
        let _ = on_event.send(InvokeResponseBody::Raw(frame));
    }
    c.sink = Some(on_event);
    Ok(SubscribeInfo { origin: c.origin.clone(), session: c.session.clone() })
}

/// Sends one or more commands on a single line; each command's reply is
/// delivered as a `reply` frame carrying the matching tag (0 = no reply wanted).
#[tauri::command]
pub fn tmux_command(
    state: State<TmuxState>,
    conn_id: u32,
    commands: Vec<String>,
    tags: Vec<u64>,
) -> Result<(), String> {
    let conn = state.get(conn_id)?;
    let mut c = conn.lock().map_err(|_| "tmux connection poisoned".to_string())?;
    c.write_commands(&commands, &tags)
}

pub fn send_keys_commands(pane: u32, data: &[u8]) -> Vec<String> {
    data.chunks(SEND_KEYS_CHUNK)
        .map(|chunk| {
            let mut s = format!("send-keys -H -t %{}", pane);
            for b in chunk {
                s.push_str(&format!(" {:02x}", b));
            }
            s
        })
        .collect()
}

#[tauri::command]
pub fn tmux_send_keys(
    state: State<TmuxState>,
    conn_id: u32,
    pane: u32,
    data: Vec<u8>,
) -> Result<(), String> {
    let conn = state.get(conn_id)?;
    let mut c = conn.lock().map_err(|_| "tmux connection poisoned".to_string())?;
    // One command per line keeps each write small and the reply count exact.
    for cmd in send_keys_commands(pane, &data) {
        c.write_commands(&[cmd], &[0])?;
    }
    Ok(())
}

#[tauri::command]
pub fn tmux_detach(state: State<TmuxState>, conn_id: u32) -> Result<(), String> {
    let conn = state.get(conn_id)?;
    let mut c = conn.lock().map_err(|_| "tmux connection poisoned".to_string())?;
    c.write_commands(&["detach-client".to_string()], &[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_version("tmux 3.4"), Some((3, 4)));
        assert_eq!(parse_version("tmux 3.5a"), Some((3, 5)));
        assert_eq!(parse_version("tmux next-3.6"), Some((3, 6)));
        assert_eq!(parse_version("tmux master"), None);
        assert!(version_supported("tmux 3.2a"));
        assert!(!version_supported("tmux 2.9"));
        assert!(version_supported("tmux master"));
    }

    #[test]
    fn send_keys_is_hex_and_chunked() {
        assert_eq!(send_keys_commands(3, b"hi\r"), vec!["send-keys -H -t %3 68 69 0d"]);
        let big = vec![b'a'; SEND_KEYS_CHUNK * 2 + 1];
        assert_eq!(send_keys_commands(1, &big).len(), 3);
    }

    fn test_conn() -> (ConnHandle, Arc<Mutex<Vec<u8>>>) {
        struct Sink(Arc<Mutex<Vec<u8>>>);
        impl Write for Sink {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let written = Arc::new(Mutex::new(Vec::new()));
        let writer: SharedWriter = Arc::new(Mutex::new(Box::new(Sink(written.clone()))));
        (Arc::new(Mutex::new(Conn::new(writer, None, None))), written)
    }

    /// Test host recording what the backend registers and forgets.
    #[derive(Default)]
    struct FakeHost {
        opened: Mutex<Vec<(u32, ConnHandle)>>,
        forgotten: Mutex<Vec<u32>>,
    }

    impl ConnectionHost for FakeHost {
        fn open(&self, conn: Conn) -> Option<(u32, ConnHandle)> {
            let mut opened = self.opened.lock().unwrap();
            let id = opened.len() as u32 + 1;
            let handle = Arc::new(Mutex::new(conn));
            opened.push((id, handle.clone()));
            Some((id, handle))
        }
        fn forget(&self, id: u32) {
            self.forgotten.lock().unwrap().push(id);
        }
    }

    fn tmux_installed() -> bool {
        Command::new("tmux").arg("-V").output().map(|o| o.status.success()).unwrap_or(false)
    }

    fn json_frames(conn: &ConnHandle) -> Vec<serde_json::Value> {
        conn.lock()
            .unwrap()
            .backlog
            .iter()
            .filter(|f| f[0] == FRAME_JSON)
            .map(|f| serde_json::from_slice(&f[1..]).unwrap())
            .collect()
    }

    fn output_bytes(conn: &ConnHandle) -> Vec<u8> {
        conn.lock()
            .unwrap()
            .backlog
            .iter()
            .filter(|f| f[0] == FRAME_OUTPUT)
            .flat_map(|f| f[5..].to_vec())
            .collect()
    }

    fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !cond() {
            assert!(std::time::Instant::now() < deadline, "timed out waiting for {}", what);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    fn kill_server(socket: &str) {
        let path = Command::new("tmux")
            .args(["-L", socket, "display", "-p", "#{socket_path}"])
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let _ = Command::new("tmux").args(["-L", socket, "kill-server"]).stderr(Stdio::null()).status();
        if !path.is_empty() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn spawned_connection_against_real_tmux() {
        if !tmux_installed() {
            return;
        }
        let socket = "mdterm-rs-spawned";
        kill_server(socket);
        let mut child = Command::new("tmux")
            .args(["-L", socket, "-f", "/dev/null", "-C", "new-session", "-s", "t", "-x", "80", "-y", "24", "sh"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let writer: SharedWriter = Arc::new(Mutex::new(Box::new(child.stdin.take().unwrap())));
        let stdout = child.stdout.take().unwrap();
        let host = Arc::new(FakeHost::default());
        let mut conn = Conn::new(writer, None, Some("t".into()));
        conn.child = Some(child);
        let (id, handle) = host.open(conn).unwrap();
        let (h2, host2) = (handle.clone(), host.clone());
        let reader = std::thread::spawn(move || run_reader(&*host2, id, h2, stdout));

        handle.lock().unwrap().write_commands(&["display -p 'hello #{session_name}'".into()], &[5]).unwrap();
        wait_until("display reply", || {
            json_frames(&handle).iter().any(|j| j["tag"] == 5 && j["lines"][0] == "hello t")
        });

        for cmd in send_keys_commands(0, b"echo from-rust-$((40+2))\r") {
            handle.lock().unwrap().write_commands(&[cmd], &[0]).unwrap();
        }
        wait_until("pane output", || {
            String::from_utf8_lossy(&output_bytes(&handle)).contains("from-rust-42")
        });

        handle.lock().unwrap().write_commands(&["split-window -h".into()], &[0]).unwrap();
        wait_until("layout change", || {
            json_frames(&handle).iter().any(|j| j["t"] == "layout" && j["layout"].as_str().unwrap_or("").contains('{'))
        });

        handle.lock().unwrap().write_commands(&["detach-client".into()], &[0]).unwrap();
        reader.join().unwrap();
        assert!(json_frames(&handle).iter().any(|j| j["t"] == "exit"));
        assert_eq!(*host.forgotten.lock().unwrap(), vec![id]);
        kill_server(socket);
    }

    #[test]
    fn inband_control_mode_in_a_pty() {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize};
        if !tmux_installed() {
            return;
        }
        let socket = "mdterm-rs-inband";
        kill_server(socket);
        let pair = native_pty_system()
            .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
            .unwrap();
        let mut cmd = CommandBuilder::new("sh");
        cmd.args([
            "-c",
            &format!("printf BEFORE; tmux -L {} -f /dev/null -CC new-session -s ib -x 80 -y 24 sh; printf AFTER; sleep 0.3", socket),
        ]);
        let mut child = pair.slave.spawn_command(cmd).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let writer: SharedWriter = Arc::new(Mutex::new(pair.master.take_writer().unwrap()));
        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let host = FakeHost::default();
        let mut tracker = InbandTracker::new("7".into(), writer, flag.clone());
        let mut terminal = Vec::new();
        let mut buf = [0u8; 4096];
        let mut sent_display = false;
        let mut sent_detach = false;
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            terminal.extend(tracker.process(&host, &buf[..n]));
            let opened = host.opened.lock().unwrap().first().map(|(_, h)| h.clone());
            let Some(handle) = opened else { continue };
            if !sent_display {
                assert!(flag.load(Ordering::SeqCst));
                assert_eq!(handle.lock().unwrap().origin.as_deref(), Some("7"));
                handle.lock().unwrap().write_commands(&["display -p inband-ok".into()], &[9]).unwrap();
                sent_display = true;
            } else if !sent_detach && json_frames(&handle).iter().any(|j| j["tag"] == 9) {
                assert_eq!(json_frames(&handle).iter().find(|j| j["tag"] == 9).unwrap()["lines"][0], "inband-ok");
                handle.lock().unwrap().write_commands(&["detach-client".into()], &[0]).unwrap();
                sent_detach = true;
            }
        }
        tracker.close(&host);
        let _ = child.wait();
        let text = String::from_utf8_lossy(&terminal);
        assert!(sent_detach, "never got the in-band reply; terminal saw {:?}", text);
        assert!(text.starts_with("BEFORE"), "terminal saw {:?}", text);
        assert!(text.contains("AFTER"), "terminal saw {:?}", text);
        assert!(!text.contains("%begin"), "control protocol leaked into the terminal: {:?}", text);
        assert!(!flag.load(Ordering::SeqCst));
        assert_eq!(*host.forgotten.lock().unwrap(), vec![1]);
        kill_server(socket);
    }

    #[test]
    fn dispatch_merges_output_and_tags_replies() {
        let (conn, written) = test_conn();
        conn.lock()
            .unwrap()
            .write_commands(&["display -p x".into(), "capture-pane -p".into()], &[7, 8])
            .unwrap();
        assert_eq!(&*written.lock().unwrap(), b"display -p x ; capture-pane -p\n");

        let mut p = ControlParser::new();
        let evs = p
            .feed(b"%begin 1 1 0\n%end 1 1 0\n%output %1 a\n%output %1 b\n%output %2 c\n%begin 1 2 1\nx\n%end 1 2 1\n%begin 1 3 1\n%error 1 3 1\n%pause %2\n")
            .events;
        dispatch(&conn, evs);
        let c = conn.lock().unwrap();
        let frames = &c.backlog;
        assert_eq!(frames[0], output_frame(1, b"ab"));
        assert_eq!(frames[1], output_frame(2, b"c"));
        let j1: serde_json::Value = serde_json::from_slice(&frames[2][1..]).unwrap();
        assert_eq!(j1, json!({"t":"reply","tag":7,"ok":true,"lines":["x"]}));
        let j2: serde_json::Value = serde_json::from_slice(&frames[3][1..]).unwrap();
        assert_eq!(j2["tag"], 8);
        assert_eq!(j2["ok"], false);
        assert!(String::from_utf8_lossy(&written.lock().unwrap()).ends_with("refresh-client -A '%2:continue'\n"));
        assert!(c.pending.front() == Some(&0));
    }
}
