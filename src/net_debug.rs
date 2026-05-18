//! Lightweight, reusable structured logging for network events.
//!
//! Two output sinks share the same call site:
//!
//! 1. A `log::debug!` record under the `mantle::network` target, so it flows
//!    through the existing `log4rs` configuration alongside the rest of the
//!    application logs.
//! 2. An optional newline-delimited JSON (NDJSON) file, configured at runtime
//!    via [`set_path`] (typically wired to a Settings checkbox) and/or
//!    bootstrapped from the `MANTLE_NET_DEBUG` environment variable. The file
//!    is opened in append mode and each record is emitted in a single atomic
//!    write so concurrent threads don't corrupt the framing.
//!
//! The fast path when the sink is disabled is one atomic load, so leaving the
//! call sites in production has negligible overhead.
//!
//! # Usage
//!
//! ```ignore
//! use mantle::net_log;
//!
//! net_log!("discover.send", "broadcast" => addr.to_string(), "bytes" => n);
//! net_log!("worker.recv",  "from" => addr.to_string(), "nbytes" => nbytes);
//! net_log!("worker.start"); // no payload
//! ```
//!
//! # Enabling the NDJSON sink
//!
//! From code (e.g. when applying user settings):
//!
//! ```ignore
//! mantle::net_debug::set_path(Some("log/network.ndjson".into()));
//! ```
//!
//! Or bootstrap it from the environment for ad-hoc bug reports:
//!
//! ```sh
//! MANTLE_NET_DEBUG=net.ndjson cargo run --release
//! ```

use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Environment variable used to bootstrap the NDJSON sink at process start,
/// before any user settings have been loaded.
pub const ENV_VAR: &str = "MANTLE_NET_DEBUG";

/// Log target used for all network events. Configure `log4rs` against this
/// target to route or filter network logs independently of the rest of the app.
pub const LOG_TARGET: &str = "mantle::network";

/// Default path used when the user enables the sink without specifying one.
/// Lives next to the existing `log/output.log` sink.
pub const DEFAULT_PATH: &str = "log/network.ndjson";

static PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static ENABLED: AtomicBool = AtomicBool::new(false);

fn path_cell() -> &'static Mutex<Option<PathBuf>> {
    PATH.get_or_init(|| {
        let initial = std::env::var(ENV_VAR)
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        if initial.is_some() {
            ENABLED.store(true, Ordering::Release);
        }
        Mutex::new(initial)
    })
}

/// Returns true when an NDJSON sink path is configured. The check is a single
/// atomic load on the fast (disabled) path, so callers can leave the
/// instrumentation enabled in release builds without measurable overhead.
pub fn file_sink_enabled() -> bool {
    let _ = path_cell();
    ENABLED.load(Ordering::Acquire)
}

/// Returns the currently-configured NDJSON sink path, if any.
pub fn current_path() -> Option<PathBuf> {
    path_cell().lock().ok().and_then(|g| g.clone())
}

/// Configure (or clear) the NDJSON sink path at runtime. Pass `None` (or an
/// empty path) to disable. The parent directory, if any, is created on the
/// first write — this function itself does not touch the filesystem.
pub fn set_path<P: Into<PathBuf>>(path: Option<P>) {
    let new = path.map(Into::into).filter(|p| !p.as_os_str().is_empty());
    let now_enabled = new.is_some();
    if let Ok(mut guard) = path_cell().lock() {
        *guard = new;
    }
    ENABLED.store(now_enabled, Ordering::Release);
}

/// Emit a structured network event.
///
/// `location` should describe the call site (the [`net_log!`] macro fills
/// this with `file!():line!()` automatically). `operation` is a short, stable
/// identifier such as `"discover.send"` or `"worker.recv"`. `details` is an
/// arbitrary JSON value carrying the event payload.
pub fn log_event(location: &str, operation: &str, details: serde_json::Value) {
    log::debug!(
        target: LOG_TARGET,
        "{} @ {} :: {}",
        operation,
        location,
        details
    );

    if !file_sink_enabled() {
        return;
    }
    let Some(path) = current_path() else { return };
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    write_ndjson(&path, location, operation, &details, timestamp_ms);
}

#[derive(Serialize)]
struct Event<'a> {
    timestamp_ms: u128,
    location: &'a str,
    operation: &'a str,
    details: &'a serde_json::Value,
}

fn write_ndjson(
    path: &std::path::Path,
    location: &str,
    operation: &str,
    details: &serde_json::Value,
    timestamp_ms: u128,
) {
    use std::io::Write;
    let event = Event {
        timestamp_ms,
        location,
        operation,
        details,
    };
    // Build the full record (JSON body + trailing newline) in memory first so
    // the file is touched with a single `write_all`. On both POSIX (`O_APPEND`)
    // and Windows (`FILE_APPEND_DATA`) a single write to an append-mode handle
    // is atomic w.r.t. concurrent writers; splitting body and newline across
    // two syscalls (as `writeln!` does) lets other threads' bytes splice in
    // between them and corrupts the NDJSON framing.
    let mut buf = Vec::with_capacity(256);
    if serde_json::to_writer(&mut buf, &event).is_err() {
        return;
    }
    buf.push(b'\n');
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        // Best-effort: a failed write to the debug sink should never disturb
        // the running application.
        let _ = f.write_all(&buf);
    }
}

/// Emit a structured network event, automatically capturing the call site.
///
/// Two forms are supported:
///
/// ```ignore
/// net_log!("worker.start");
/// net_log!("worker.recv", "from" => addr.to_string(), "nbytes" => nbytes);
/// ```
#[macro_export]
macro_rules! net_log {
    ($operation:expr) => {
        $crate::net_debug::log_event(
            concat!(file!(), ":", line!()),
            $operation,
            ::serde_json::Value::Null,
        )
    };
    ($operation:expr, $($key:tt => $value:expr),+ $(,)?) => {
        $crate::net_debug::log_event(
            concat!(file!(), ":", line!()),
            $operation,
            ::serde_json::json!({ $( $key: $value ),+ }),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_event_does_not_panic_without_sink() {
        log_event(
            "net_debug.rs:test",
            "test.event",
            serde_json::json!({ "k": 1 }),
        );
    }

    #[test]
    fn macro_zero_payload_compiles() {
        crate::net_log!("test.empty");
    }

    #[test]
    fn macro_with_payload_compiles() {
        let n: u32 = 42;
        crate::net_log!("test.payload", "n" => n, "label" => "hello");
    }

    #[test]
    fn ndjson_sink_round_trips() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "mantle-net-debug-test-{}.ndjson",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        write_ndjson(
            &path,
            "net_debug.rs:test",
            "test.write",
            &serde_json::json!({ "k": "v" }),
            12345,
        );

        let body = std::fs::read_to_string(&path).expect("file written");
        let line = body.trim_end();
        let parsed: serde_json::Value = serde_json::from_str(line).expect("valid json");
        assert_eq!(parsed["operation"], "test.write");
        assert_eq!(parsed["location"], "net_debug.rs:test");
        assert_eq!(parsed["timestamp_ms"], 12345_u64);
        assert_eq!(parsed["details"]["k"], "v");

        let _ = std::fs::remove_file(&path);
    }

    /// `write_ndjson` should create any missing parent directories so users
    /// who enable the sink via Settings don't have to pre-create `log/`.
    #[test]
    fn ndjson_sink_creates_parent_dirs() {
        let dir =
            std::env::temp_dir().join(format!("mantle-net-debug-mkdir-{}", std::process::id()));
        let path = dir.join("nested/network.ndjson");
        let _ = std::fs::remove_dir_all(&dir);

        write_ndjson(
            &path,
            "net_debug.rs:test",
            "test.mkdir",
            &serde_json::Value::Null,
            1,
        );

        assert!(path.exists(), "expected sink file at {:?}", path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Regression test: concurrent threads must not interleave bytes within a
    /// single NDJSON record. A previous implementation used `writeln!` which
    /// emits the body and the trailing `\n` in two separate `write` syscalls,
    /// allowing other threads to splice in and corrupt the framing.
    #[test]
    fn ndjson_sink_is_concurrency_safe() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "mantle-net-debug-concurrent-{}.ndjson",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        const THREADS: usize = 8;
        const PER_THREAD: usize = 200;
        let mut handles = Vec::with_capacity(THREADS);
        for tid in 0..THREADS {
            let p = path.clone();
            handles.push(std::thread::spawn(move || {
                for i in 0..PER_THREAD {
                    write_ndjson(
                        &p,
                        "net_debug.rs:concurrent",
                        "test.concurrent",
                        &serde_json::json!({ "tid": tid, "i": i }),
                        i as u128,
                    );
                }
            }));
        }
        for h in handles {
            h.join().expect("thread panicked");
        }

        let body = std::fs::read_to_string(&path).expect("file written");
        let mut total = 0;
        for (lineno, raw) in body.lines().enumerate() {
            assert!(!raw.is_empty(), "blank line at {}", lineno);
            serde_json::from_str::<serde_json::Value>(raw).unwrap_or_else(|e| {
                panic!(
                    "interleaved NDJSON record at line {}: {} :: {:?}",
                    lineno, e, raw
                )
            });
            total += 1;
        }
        assert_eq!(
            total,
            THREADS * PER_THREAD,
            "wrong number of records written"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// `set_path` should toggle the `file_sink_enabled` flag and update the
    /// path queryable via `current_path`.
    #[test]
    fn set_path_toggles_runtime_state() {
        // Snapshot current state so this test doesn't disturb others sharing
        // the process-wide statics.
        let prior = current_path();

        set_path(Some("temp/mantle-network.ndjson"));
        assert!(file_sink_enabled());
        assert_eq!(
            current_path().as_deref(),
            Some(std::path::Path::new("temp/mantle-network.ndjson"))
        );

        set_path::<&str>(None);
        assert!(!file_sink_enabled());
        assert_eq!(current_path(), None);

        set_path(Some(""));
        assert!(!file_sink_enabled(), "empty path should disable sink");

        // Restore (best-effort).
        set_path(prior);
    }
}
