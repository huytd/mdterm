use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize, Deserialize)]
struct ResizeEvent {
    #[serde(rename = "type")]
    type_name: String,
    offset: u64,
    cols: u16,
    rows: u16,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaJson {
    cols: u16,
    rows: u16,
    command: String,
    tmux: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    xterm_version: Option<String>,
    events: Vec<ResizeEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TmuxPane {
    id: String,
    left: u16,
    top: u16,
    width: u16,
    height: u16,
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TmuxOracle {
    panes: Vec<TmuxPane>,
    status: String,
}

// Duplicated from src-tauri/src/pty_stream.rs::shell_env with PTY recording adjustments
fn shell_env() -> Vec<(String, String)> {
    let mut env = Vec::new();
    env.push(("TERM".to_string(), "xterm-256color".to_string()));
    env.push(("COLORTERM".to_string(), "truecolor".to_string()));
    env.push(("MDTERM".to_string(), "1".to_string()));
    env.push(("TERM_PROGRAM".to_string(), "mdterm".to_string()));
    env.push(("LANG".to_string(), "C.UTF-8".to_string()));

    let existing_path = env::var("PATH").unwrap_or_default();
    let home = env::var("HOME").unwrap_or_default();
    let home_bin = if !home.is_empty() {
        format!("{}/.local/bin", home)
    } else {
        String::new()
    };
    let repo_bin = env::current_dir()
        .map(|d| d.join("bin").to_string_lossy().to_string())
        .unwrap_or_default();
    let new_path = format!("{}:{}:{}", repo_bin, home_bin, existing_path);
    env.push(("PATH".to_string(), new_path));

    env
}

fn read_xterm_version() -> Option<String> {
    let candidates = [
        "frontend/vendor/VERSIONS.json",
        "../../frontend/vendor/VERSIONS.json",
    ];
    for path in &candidates {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(v) = json.get("@xterm/xterm").and_then(|v| v.as_str()) {
                    return Some(v.to_string());
                }
                if let Some(v) = json.get("@xterm/headless").and_then(|v| v.as_str()) {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn strip_tmux_styles(s: &str) -> String {
    let mut result = String::new();
    let mut inside = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !inside && chars[i] == '#' && i + 1 < chars.len() && chars[i + 1] == '[' {
            inside = true;
            i += 2;
            continue;
        }
        if inside {
            if chars[i] == ']' {
                inside = false;
            }
            i += 1;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    // Also remove tmux window list overflow markers '<' and '>' if present
    result.replace(['<', '>'], "").trim().to_string()
}

fn capture_tmux_oracle(out_path: &Path) -> std::io::Result<()> {
    // 1. List panes
    let list_output = StdCommand::new("tmux")
        .args([
            "-L",
            "mdterm-harness",
            "list-panes",
            "-a",
            "-F",
            "#{pane_id} #{pane_left} #{pane_top} #{pane_width} #{pane_height}",
        ])
        .output()?;

    let list_str = String::from_utf8_lossy(&list_output.stdout);
    let mut panes = Vec::new();

    for line in list_str.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }
        let pane_id = parts[0].to_string();
        let left: u16 = parts[1].parse().unwrap_or(0);
        let top: u16 = parts[2].parse().unwrap_or(0);
        let width: u16 = parts[3].parse().unwrap_or(0);
        let height: u16 = parts[4].parse().unwrap_or(0);

        // 2. Capture pane text
        let cap_output = StdCommand::new("tmux")
            .args([
                "-L",
                "mdterm-harness",
                "capture-pane",
                "-p",
                "-N",
                "-t",
                &pane_id,
            ])
            .output()?;

        let text = String::from_utf8_lossy(&cap_output.stdout).to_string();

        panes.push(TmuxPane {
            id: pane_id,
            left,
            top,
            width,
            height,
            text,
        });
    }

    // 3. Capture status line text
    let status_output = StdCommand::new("tmux")
        .args([
            "-L",
            "mdterm-harness",
            "display-message",
            "-p",
            "#{status-left}#I:#W*",
        ])
        .output()?;

    let mut status_str = String::from_utf8_lossy(&status_output.stdout)
        .trim_end_matches(['\r', '\n'])
        .to_string();

    if status_str.is_empty() {
        let fmt_output = StdCommand::new("tmux")
            .args([
                "-L",
                "mdterm-harness",
                "display-message",
                "-p",
                "#{T:status-format[0]}",
            ])
            .output()?;
        status_str = strip_tmux_styles(&String::from_utf8_lossy(&fmt_output.stdout));
    }

    let oracle = TmuxOracle {
        panes,
        status: status_str,
    };

    let oracle_file = out_path.join("tmux-oracle.json");
    let json_bytes = serde_json::to_vec_pretty(&oracle)?;
    fs::write(oracle_file, json_bytes)?;

    // 4. Kill server for this socket
    let _ = StdCommand::new("tmux")
        .args(["-L", "mdterm-harness", "kill-server"])
        .output();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_args: Vec<String> = env::args().collect();

    let mut cols: u16 = 80;
    let mut rows: u16 = 24;
    let mut out_dir: Option<PathBuf> = None;
    let mut is_tmux = false;
    let mut quiet_ms: u64 = 500;
    let mut timeout_ms: u64 = 10000;
    let mut resize_after_ms: Option<u64> = None;
    let mut resize_dims: Option<(u16, u16)> = None;
    let mut cmd_args: Vec<String> = Vec::new();

    let mut i = 1;
    while i < raw_args.len() {
        match raw_args[i].as_str() {
            "--cols" => {
                i += 1;
                cols = raw_args[i].parse()?;
            }
            "--rows" => {
                i += 1;
                rows = raw_args[i].parse()?;
            }
            "--out" => {
                i += 1;
                out_dir = Some(PathBuf::from(&raw_args[i]));
            }
            "--tmux" => {
                is_tmux = true;
            }
            "--quiet-ms" => {
                i += 1;
                quiet_ms = raw_args[i].parse()?;
            }
            "--timeout-ms" => {
                i += 1;
                timeout_ms = raw_args[i].parse()?;
            }
            "--resize-after-ms" => {
                i += 1;
                resize_after_ms = Some(raw_args[i].parse()?);
            }
            "--resize" => {
                i += 1;
                let s = &raw_args[i];
                if let Some((w, h)) = s.split_once('x') {
                    resize_dims = Some((w.parse()?, h.parse()?));
                } else {
                    eprintln!("Invalid resize format, expected COLSxROWS: {}", s);
                    std::process::exit(1);
                }
            }
            "--" => {
                i += 1;
                while i < raw_args.len() {
                    cmd_args.push(raw_args[i].clone());
                    i += 1;
                }
                break;
            }
            other => {
                if !other.starts_with('-') && cmd_args.is_empty() {
                    // Positional command
                    while i < raw_args.len() {
                        cmd_args.push(raw_args[i].clone());
                        i += 1;
                    }
                    break;
                } else {
                    eprintln!("Unknown argument: {}", other);
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    let out_dir = match out_dir {
        Some(d) => d,
        None => {
            eprintln!("Error: --out <path> is required");
            std::process::exit(1);
        }
    };

    if cmd_args.is_empty() {
        eprintln!("Error: no command provided after --");
        std::process::exit(1);
    }

    fs::create_dir_all(&out_dir)?;

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = if is_tmux {
        // Kill any leftover server on mdterm-harness socket before starting
        let _ = StdCommand::new("tmux")
            .args(["-L", "mdterm-harness", "kill-server"])
            .output();

        let conf_path = if Path::new("tests/terminal/tmux.conf").exists() {
            Path::new("tests/terminal/tmux.conf")
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from("tests/terminal/tmux.conf"))
        } else {
            PathBuf::from("tests/terminal/tmux.conf")
        };

        let mut c = CommandBuilder::new("tmux");
        c.cwd(env::current_dir()?);
        c.arg("-L");
        c.arg("mdterm-harness");
        c.arg("-f");
        c.arg(conf_path.to_string_lossy().as_ref());
        c.arg("new-session");
        c.arg("-n");
        c.arg("mdterm");
        c.arg("-x");
        c.arg(cols.to_string());
        c.arg("-y");
        c.arg(rows.to_string());
        c.arg("--");
        for a in &cmd_args {
            c.arg(a);
        }
        c
    } else {
        let mut c = CommandBuilder::new(&cmd_args[0]);
        c.cwd(env::current_dir()?);
        for a in &cmd_args[1..] {
            c.arg(a);
        }
        c
    };

    for (k, v) in shell_env() {
        cmd.env(k, v);
    }

    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let (tx, rx) = channel::<Vec<u8>>();

    let reader_handle = thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let output_bin_path = out_dir.join("output.bin");
    let mut bin_file = File::create(&output_bin_path)?;

    let start_time = Instant::now();
    let mut last_activity = Instant::now();
    let mut bytes_written: u64 = 0;
    let mut resized = false;
    let mut events = Vec::new();

    let check_interval = Duration::from_millis(20);

    loop {
        // Drain any available chunks
        let mut got_bytes = false;
        while let Ok(chunk) = rx.try_recv() {
            bin_file.write_all(&chunk)?;
            bytes_written += chunk.len() as u64;
            got_bytes = true;
        }

        if got_bytes {
            bin_file.flush()?;
            last_activity = Instant::now();
        }

        // Handle mid-recording resize if requested
        if let (Some(after_ms), Some((new_cols, new_rows))) = (resize_after_ms, resize_dims) {
            if !resized && start_time.elapsed() >= Duration::from_millis(after_ms) {
                pair.master.resize(PtySize {
                    rows: new_rows,
                    cols: new_cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })?;
                events.push(ResizeEvent {
                    type_name: "resize".to_string(),
                    offset: bytes_written,
                    cols: new_cols,
                    rows: new_rows,
                });
                resized = true;
            }
        }

        // Determine if we should stop
        let quiet_duration = last_activity.elapsed();
        let total_duration = start_time.elapsed();

        let resize_pending = resize_after_ms.is_some() && !resized;

        // If quiet threshold met (and resize completed if one was scheduled)
        if !resize_pending && quiet_duration >= Duration::from_millis(quiet_ms) {
            break;
        }

        if total_duration >= Duration::from_millis(timeout_ms) {
            eprintln!("Warning: term-rec hit timeout of {}ms", timeout_ms);
            break;
        }

        thread::sleep(check_interval);
    }

    // If tmux mode, capture oracle BEFORE killing server and before closing child
    if is_tmux {
        if let Err(e) = capture_tmux_oracle(&out_dir) {
            eprintln!("Warning: failed to capture tmux oracle: {}", e);
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    drop(rx);
    let _ = reader_handle.join();

    let xterm_version = read_xterm_version();
    let meta = MetaJson {
        cols,
        rows,
        command: cmd_args.join(" "),
        tmux: is_tmux,
        xterm_version,
        events,
    };

    let meta_json_path = out_dir.join("meta.json");
    let meta_bytes = serde_json::to_vec_pretty(&meta)?;
    fs::write(meta_json_path, meta_bytes)?;

    println!(
        "term-rec finished: recorded {} bytes to {}",
        bytes_written,
        output_bin_path.display()
    );

    Ok(())
}
