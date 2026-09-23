use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError, TryRecvError};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Returns environment variables configured for PTY sessions.
pub fn shell_env() -> Vec<(String, String)> {
    let mut env = Vec::new();
    env.push(("TERM".to_string(), "xterm-256color".to_string()));
    env.push(("COLORTERM".to_string(), "truecolor".to_string()));
    env.push(("MDTERM".to_string(), "1".to_string()));
    env.push(("TERM_PROGRAM".to_string(), "mdterm".to_string()));

    if std::env::var("LANG").is_err() {
        env.push(("LANG".to_string(), "en_US.UTF-8".to_string()));
    }

    let existing_path = std::env::var("PATH").unwrap_or_default();
    let home_bin = dirs::home_dir()
        .map(|h| h.join(".local/bin").to_string_lossy().to_string())
        .unwrap_or_default();
    let repo_bin = std::env::current_dir()
        .map(|d| d.join("bin").to_string_lossy().to_string())
        .unwrap_or_default();
    let new_path = format!("{}:{}:{}", repo_bin, home_bin, existing_path);
    env.push(("PATH".to_string(), new_path));

    env
}

/// Pump raw bytes from reader to sink with coalescing.
/// Never decodes UTF-8. Coalesces reads into batches up to `max_batch`
/// with a short (~4ms) coalescing window. Calls `sink` with raw bytes
/// in order, unchanged. Returns on EOF or error, flushing any pending batch.
pub fn pump<R: Read + Send + 'static>(
    mut reader: R,
    max_batch: usize,
    mut sink: impl FnMut(&[u8]),
) {
    let max_batch = if max_batch == 0 { 65536 } else { max_batch };
    let (tx, rx) = channel::<Vec<u8>>();

    let reader_handle = std::thread::spawn(move || {
        let mut buf = [0u8; 16384];
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

    let coalesce_timeout = Duration::from_millis(4);
    let mut batch = Vec::with_capacity(max_batch);

    loop {
        if batch.is_empty() {
            match rx.recv() {
                Ok(chunk) => {
                    batch.extend_from_slice(&chunk);
                }
                Err(_) => {
                    // Reader thread closed and no pending batch
                    break;
                }
            }
        }

        let deadline = Instant::now() + coalesce_timeout;
        let mut disconnected = false;

        while batch.len() < max_batch {
            match rx.try_recv() {
                Ok(chunk) => {
                    batch.extend_from_slice(&chunk);
                    continue;
                }
                Err(TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }

            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let remaining = deadline - now;
            match rx.recv_timeout(remaining) {
                Ok(chunk) => {
                    batch.extend_from_slice(&chunk);
                }
                Err(RecvTimeoutError::Timeout) => {
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        while batch.len() >= max_batch {
            sink(&batch[..max_batch]);
            batch.drain(..max_batch);
        }

        if disconnected || Instant::now() >= deadline {
            if !batch.is_empty() {
                sink(&batch);
                batch.clear();
            }
        }

        if disconnected && batch.is_empty() {
            break;
        }
    }

    let _ = reader_handle.join();
}

/// Returns the configured recording directory from MDTERM_RECORD_DIR if set and non-empty.
pub fn record_dir() -> Option<PathBuf> {
    match std::env::var("MDTERM_RECORD_DIR") {
        Ok(dir) if !dir.is_empty() => Some(PathBuf::from(dir)),
        _ => None,
    }
}

/// Helper for recording session raw output and resize events.
#[derive(Clone, Debug)]
pub struct SessionRecorder {
    pub dir: PathBuf,
    pub session_id: String,
    pub unix_ts: u64,
    start_time: Instant,
    bytes_written: Arc<AtomicU64>,
    bin_file: Arc<Mutex<std::fs::File>>,
    events_file: Arc<Mutex<std::fs::File>>,
}

impl SessionRecorder {
    /// Creates a new `SessionRecorder` if `MDTERM_RECORD_DIR` is set.
    pub fn new(session_id: &str) -> Option<Self> {
        let dir = record_dir()?;
        let unix_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self::with_dir(dir, session_id, unix_ts)
    }

    /// Creates a new `SessionRecorder` targeting a specific directory and timestamp.
    pub fn with_dir(dir: PathBuf, session_id: &str, unix_ts: u64) -> Option<Self> {
        let _ = std::fs::create_dir_all(&dir);
        let bin_path = dir.join(format!("{}-{}.bin", session_id, unix_ts));
        let events_path = dir.join(format!("{}-{}.events.jsonl", session_id, unix_ts));

        let bin_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&bin_path)
            .ok()?;
        let events_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)
            .ok()?;

        Some(Self {
            dir,
            session_id: session_id.to_string(),
            unix_ts,
            start_time: Instant::now(),
            bytes_written: Arc::new(AtomicU64::new(0)),
            bin_file: Arc::new(Mutex::new(bin_file)),
            events_file: Arc::new(Mutex::new(events_file)),
        })
    }

    /// Records raw output bytes.
    pub fn record_bytes(&self, bytes: &[u8]) {
        if let Ok(mut f) = self.bin_file.lock() {
            if f.write_all(bytes).is_ok() {
                self.bytes_written.fetch_add(bytes.len() as u64, Ordering::SeqCst);
            }
            let _ = f.flush();
        }
    }

    /// Records a resize event with elapsed milliseconds and the byte offset
    /// into the `.bin` stream at which it took effect (used for replay).
    pub fn record_resize(&self, cols: u16, rows: u16) {
        let ms = self.start_time.elapsed().as_millis();
        let offset = self.bytes_written.load(Ordering::SeqCst);
        let line = format!(
            "{{\"t\":{},\"type\":\"resize\",\"offset\":{},\"cols\":{},\"rows\":{}}}\n",
            ms, offset, cols, rows
        );
        if let Ok(mut f) = self.events_file.lock() {
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn unix_ts(&self) -> u64 {
        self.unix_ts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockChunkReader {
        chunks: Vec<Vec<u8>>,
        index: usize,
    }

    impl MockChunkReader {
        fn new(chunks: Vec<Vec<u8>>) -> Self {
            Self { chunks, index: 0 }
        }
    }

    impl Read for MockChunkReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.index >= self.chunks.len() {
                return Ok(0);
            }
            let chunk = &self.chunks[self.index];
            let n = chunk.len().min(buf.len());
            buf[..n].copy_from_slice(&chunk[..n]);
            if n == chunk.len() {
                self.index += 1;
            } else {
                self.chunks[self.index] = chunk[n..].to_vec();
            }
            Ok(n)
        }
    }

    struct OneByteReader {
        data: Vec<u8>,
        pos: usize,
    }

    impl OneByteReader {
        fn new(data: Vec<u8>) -> Self {
            Self { data, pos: 0 }
        }
    }

    impl Read for OneByteReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.pos >= self.data.len() || buf.is_empty() {
                return Ok(0);
            }
            buf[0] = self.data[self.pos];
            self.pos += 1;
            Ok(1)
        }
    }

    #[test]
    fn test_shell_env() {
        let env = shell_env();
        let map: std::collections::HashMap<_, _> = env.into_iter().collect();
        assert_eq!(map.get("TERM").map(|s| s.as_str()), Some("xterm-256color"));
        assert_eq!(map.get("COLORTERM").map(|s| s.as_str()), Some("truecolor"));
        assert_eq!(map.get("MDTERM").map(|s| s.as_str()), Some("1"));
        assert_eq!(map.get("TERM_PROGRAM").map(|s| s.as_str()), Some("mdterm"));
        assert!(map.contains_key("PATH"));
        let path = map.get("PATH").unwrap();
        assert!(path.contains(".local/bin"));
    }

    #[test]
    fn test_utf8_split_at_every_offset_and_one_byte() {
        let test_str = "─│┌┐└┘ 漢字 🎉👍🏽";
        let orig_bytes = test_str.as_bytes();

        // (a) Returning chunks split at EVERY possible byte offset
        for split_idx in 0..=orig_bytes.len() {
            let chunks = if split_idx == 0 || split_idx == orig_bytes.len() {
                vec![orig_bytes.to_vec()]
            } else {
                vec![
                    orig_bytes[..split_idx].to_vec(),
                    orig_bytes[split_idx..].to_vec(),
                ]
            };

            let reader = MockChunkReader::new(chunks);
            let mut output = Vec::new();
            pump(reader, 65536, |chunk| {
                output.extend_from_slice(chunk);
            });

            assert_eq!(
                output, orig_bytes,
                "Mismatch when split at byte offset {}",
                split_idx
            );
        }

        // (a) 1-byte-per-read variant
        let reader = OneByteReader::new(orig_bytes.to_vec());
        let mut output = Vec::new();
        pump(reader, 65536, |chunk| {
            output.extend_from_slice(chunk);
        });
        assert_eq!(output, orig_bytes, "Failed on 1-byte-per-read variant");
    }

    #[test]
    fn test_batching_coalescing_1000_one_byte_reads() {
        // (b) batching: 1000 one-byte reads produce far fewer sink calls, each <= max_batch
        let data = vec![b'q'; 1000];
        let max_batch = 64 * 1024; // 64 KiB
        let reader = OneByteReader::new(data.clone());

        let mut sink_calls = 0;
        let mut collected = Vec::new();

        pump(reader, max_batch, |chunk| {
            assert!(
                chunk.len() <= max_batch,
                "chunk size {} exceeded max_batch {}",
                chunk.len(),
                max_batch
            );
            sink_calls += 1;
            collected.extend_from_slice(chunk);
        });

        assert_eq!(collected, data);
        assert!(
            sink_calls < 50,
            "1000 one-byte reads produced {} sink calls, expected far fewer",
            sink_calls
        );

        // Also test with smaller max_batch to ensure <= max_batch invariant holds
        let small_max_batch = 120;
        let reader = OneByteReader::new(data.clone());
        let mut small_sink_calls = 0;
        let mut small_collected = Vec::new();

        pump(reader, small_max_batch, |chunk| {
            assert!(
                chunk.len() <= small_max_batch,
                "chunk size {} exceeded small_max_batch {}",
                chunk.len(),
                small_max_batch
            );
            small_sink_calls += 1;
            small_collected.extend_from_slice(chunk);
        });

        assert_eq!(small_collected, data);
        assert!(
            small_sink_calls <= 20,
            "1000 bytes with max_batch=120 produced {} calls",
            small_sink_calls
        );
    }

    #[test]
    fn test_pty_printf_box_drawing_emoji() {
        // (c) integration test using portable_pty to spawn /bin/sh -c with printf printing box drawing + emoji;
        // assert those exact UTF-8 bytes appear in the collected output.
        use portable_pty::{native_pty_system, CommandBuilder, PtySize};

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to open PTY");

        let test_str = "─│┌┐└┘ 漢字 🎉👍🏽";
        let mut cmd = CommandBuilder::new("/bin/sh");
        cmd.args(["-c", &format!("printf '%s' '{}'", test_str)]);

        for (k, v) in shell_env() {
            cmd.env(k, v);
        }

        let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn command");
        drop(pair.slave);

        let reader = pair.master.try_clone_reader().expect("Failed to clone reader");
        let mut collected = Vec::new();

        pump(reader, 65536, |chunk| {
            collected.extend_from_slice(chunk);
        });

        let _ = child.wait();

        let expected_bytes = test_str.as_bytes();
        assert!(
            collected.windows(expected_bytes.len()).any(|w| w == expected_bytes),
            "Expected exact UTF-8 bytes {:?} to appear in collected PTY output {:?}",
            expected_bytes,
            collected
        );
    }

    #[test]
    fn test_optional_recording() {
        let temp_dir = std::env::temp_dir().join(format!(
            "mdterm_rec_test_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        let session_id = "test-session";
        let unix_ts = 1700000000;

        let recorder = SessionRecorder::with_dir(temp_dir.clone(), session_id, unix_ts)
            .expect("Failed to create SessionRecorder");

        recorder.record_bytes(b"sample terminal bytes");
        recorder.record_resize(80, 24);

        let bin_path = temp_dir.join(format!("{}-{}.bin", session_id, unix_ts));
        let events_path = temp_dir.join(format!("{}-{}.events.jsonl", session_id, unix_ts));

        assert!(bin_path.exists());
        assert!(events_path.exists());

        let bin_content = std::fs::read(&bin_path).unwrap();
        assert_eq!(bin_content, b"sample terminal bytes");

        let events_content = std::fs::read_to_string(&events_path).unwrap();
        assert!(events_content.contains("\"type\":\"resize\""));
        assert!(events_content.contains("\"cols\":80"));
        assert!(events_content.contains("\"rows\":24"));
        assert!(events_content.contains("\"offset\":21"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
