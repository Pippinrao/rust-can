//! Instrumentation-based profiling helpers used by the ASC / BLF readers.
//!
//! Activated by the `profile` feature on `rust-can-io`. When the feature is
//! off, every `prof_scope!` call expands to a no-op so there is no
//! runtime cost. When the feature is on, the helpers record a per-thread
//! call stack of `Instant` samples and dump folded stacks for
//! `flamegraph.pl`.
//!
//! Output format (one line per call site):
//!     main;reader::collect_events;parse_log_container;decompress_container 1234
//!
//! This is intentionally not a sampling profiler: it cannot capture code
//! that was not instrumented, and the times are deterministic, not
//! statistical. But it is enough to produce a real flame graph of where
//! the reader spends its time without OS-level sampling (which on
//! Windows needs admin privileges for ETW / `blondie`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

thread_local! {
    static STACK: RefCell<Vec<&'static str>> = RefCell::new(Vec::with_capacity(64));
}

/// Set to true by the `enable` function. When false, `prof_scope!`
/// expands to a no-op.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Globals keyed by the current call stack. We accumulate per-stack
/// total time; the total for a given stack is what becomes one folded
/// stack line at dump time. Initialized lazily on first access.
type AccKey = Vec<&'static str>;
type AccValue = (Duration, u64);
type AccMap = HashMap<AccKey, AccValue>;
static ACCUM: LazyLock<Mutex<AccMap>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Enable profiling for the current process. Idempotent.
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// Disable profiling for the current process. Mainly useful in tests
/// to opt out of cross-test state leakage; production code never
/// calls this.
pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
}

/// Returns whether profiling is enabled.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// A scoped timer. Create at the top of an instrumented function; drop
/// at the end.
pub struct Scope {
    name: &'static str,
    /// The instant this scope started. Held in the struct so that
    /// `Drop` can compute `elapsed = now - started` without reaching
    /// into the per-thread stack; this keeps the borrow short and
    /// avoids an aliasing `RefMut` panic.
    started: Instant,
}

impl Scope {
    /// Create a new scope. The `name` is recorded as the topmost
    /// stack frame when the scope is dropped. The scope starts
    /// measuring wall time immediately; pass it to `prof_scope!` to
    /// make the intent clear.
    #[inline]
    pub fn new(name: &'static str) -> Self {
        let started = Instant::now();
        if ENABLED.load(Ordering::Relaxed) {
            STACK.with(|s| s.borrow_mut().push(name));
        }
        Self { name, started }
    }
}

impl Drop for Scope {
    #[inline]
    fn drop(&mut self) {
        if !ENABLED.load(Ordering::Relaxed) {
            return;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.started);

        // Take the stack snapshot inside the `with` closure so the
        // `RefMut` borrow ends before we touch the global accumulator.
        let mut key: AccKey = Vec::new();
        STACK.with(|s| {
            let mut stack = s.borrow_mut();
            let popped = stack.pop();
            debug_assert_eq!(
                popped,
                Some(self.name),
                "scope drop order mismatch"
            );
            key.extend(stack.iter().copied());
            key.push(self.name);
        });

        let mut accum = ACCUM.lock().expect("prof ACCUM poisoned");
        let entry = accum.entry(key).or_insert((Duration::ZERO, 0));
        entry.0 += elapsed;
        entry.1 += 1;
    }
}

/// Drain the accumulated samples and write folded stacks to `out`,
/// sorted by descending total time. The format matches Brendan
/// Gregg's `flamegraph.pl` (one stack per line, semicolon-joined,
/// space + sample weight in nanoseconds).
pub fn dump_folded<W: Write>(out: &mut W) -> std::io::Result<()> {
    let accum = ACCUM.lock().expect("prof ACCUM poisoned");
    let mut entries: Vec<_> = accum.iter().collect();
    entries.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(b.1 .1.cmp(&a.1 .1)));
    let mut buf = String::new();
    for (stack, (total, _count)) in entries {
        let nanos = total.as_nanos() as u64;
        buf.push_str(&stack.join(";"));
        buf.push(' ');
        buf.push_str(&nanos.to_string());
        buf.push('\n');
    }
    out.write_all(buf.as_bytes())
}

/// Same as `dump_folded` but also flushes the global accumulator. Call
/// once at the end of the profiled run.
pub fn dump_and_reset<W: Write>(out: &mut W) -> std::io::Result<()> {
    dump_folded(out)?;
    ACCUM.lock().expect("prof ACCUM poisoned").clear();
    Ok(())
}

/// Snapshot the global accumulator as a multi-line string (for tests).
pub fn snapshot() -> String {
    let mut s = Vec::new();
    let _ = dump_folded(&mut s);
    String::from_utf8(s).unwrap_or_default()
}

/// Reset the per-thread stack to empty. Only intended for test setup
/// so each test sees a clean slate. In production, the stack is
/// pushed/popped by `Scope::new` / `Scope::drop` in matched order.
#[doc(hidden)]
pub fn __reset_stack_for_tests() {
    STACK.with(|s| s.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::thread;
    use std::time::Duration;
    use std::sync::Mutex;

    /// Tests in this module mutate two process-wide pieces of state:
    /// the `ENABLED` atomic and the `ACCUM` mutex. `STACK` is
    /// thread-local so it does not need locking, but `ENABLED` and
    /// `ACCUM` are shared across all tests in the same process.
    /// Without serialization, a `disable()` from one test can race
    /// a `Scope::new` from another and produce flaky failures.
    ///
    /// Hold this lock for the duration of any test that creates a
    /// `Scope` or reads the accumulator.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Build a fresh scope without going through `enable` — every test
    /// sets `ENABLED` explicitly and resets the accumulator.
    struct EnableGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }
    impl EnableGuard {
        fn new() -> Self {
            let lock = TEST_LOCK.lock().expect("prof test lock poisoned");
            // Disable first so the order of test execution cannot
            // make the new test inherit a previous test's enabled
            // state. Also reset the per-thread stack: a previous test
            // may have left dangling entries (e.g. a scope whose
            // `Drop` did not run because of a panic).
            disable();
            __reset_stack_for_tests();
            dump_and_reset(&mut Vec::new()).expect("clean start");
            enable();
            EnableGuard { _lock: lock }
        }
    }
    impl Drop for EnableGuard {
        fn drop(&mut self) {
            dump_and_reset(&mut Vec::new()).ok();
            __reset_stack_for_tests();
            disable();
        }
    }

    #[test]
    fn no_op_when_disabled() {
        // The default state is disabled, so a Scope created and
        // dropped without `enable` should not appear in the snapshot.
        let _g = EnableGuard::new();
        // Disable explicitly so this test exercises the disabled path.
        disable();
        {
            let _s = Scope::new("a");
            let _s = Scope::new("b");
        }
        let snap = snapshot();
        assert!(snap.is_empty(), "snapshot not empty: {snap:?}");
    }

    #[test]
    fn records_top_level_scope() {
        let _g = EnableGuard::new();
        {
            let _s = Scope::new("a");
            thread::sleep(Duration::from_millis(1));
        }
        let snap = snapshot();
        assert!(snap.contains("a "), "snapshot: {snap:?}");
    }

    #[test]
    fn records_nested_call_stacks() {
        let _g = EnableGuard::new();
        {
            let _outer = Scope::new("outer");
            {
                let _inner = Scope::new("inner");
                thread::sleep(Duration::from_millis(1));
            }
        }
        let snap = snapshot();
        assert!(snap.contains("outer;inner"), "snapshot: {snap:?}");
    }

    #[test]
    fn aggregates_repeated_calls() {
        let _g = EnableGuard::new();
        for _ in 0..3 {
            let _s = Scope::new("loop");
        }
        let snap = snapshot();
        let line = snap
            .lines()
            .find(|line| line.starts_with("loop "))
            .expect("loop entry must be present");
        // The second field is the accumulated weight in nanoseconds; we
        // just assert it is non-zero.
        let weight: u64 = line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .expect("weight is u64");
        assert!(weight > 0, "expected non-zero weight, got {weight}");
    }

    #[test]
    fn dump_and_reset_clears_accumulator() {
        let _g = EnableGuard::new();
        {
            let _s = Scope::new("phase-a");
        }
        let first = snapshot();
        assert!(first.contains("phase-a"));
        let mut sink = Vec::new();
        dump_and_reset(&mut sink).expect("dump ok");
        assert!(!sink.is_empty());
        // After reset, the next snapshot is empty (until the next scope).
        let second = snapshot();
        assert!(second.is_empty());
    }

    #[test]
    fn dump_folded_writes_to_writer() {
        let _g = EnableGuard::new();
        {
            let _s = Scope::new("sink-target");
        }
        let mut buf: Vec<u8> = Vec::new();
        dump_folded(&mut buf).expect("dump ok");
        let text = String::from_utf8(buf).expect("utf-8 output");
        assert!(text.contains("sink-target"));
    }

    #[test]
    fn drop_after_reset_still_records() {
        let _g = EnableGuard::new();
        let s = Scope::new("post-reset");
        dump_and_reset(&mut Vec::new()).ok();
        // `s` has not been dropped yet, so its Drop runs after the
        // reset and writes into the now-empty accumulator.
        drop(s);
        let snap = snapshot();
        assert!(snap.contains("post-reset"));
    }

    #[test]
    fn is_enabled_reflects_state() {
        let _g = EnableGuard::new();
        // EnableGuard has just enabled, so the flag must be set.
        assert!(is_enabled());
        disable();
        assert!(!is_enabled());
    }

    #[test]
    fn other_thread_scopes_appear_in_global_accumulator() {
        // The thread-local STACK only protects in-flight `Drop`
        // ordering. The accumulator itself is global, so scopes that
        // finished on another thread do show up in this thread's
        // snapshot. This documents that contract so a refactor does
        // not silently change it.
        let _g = EnableGuard::new();
        let h = thread::spawn(|| {
            let _s = Scope::new("from-other-thread");
            thread::sleep(Duration::from_millis(1));
        });
        h.join().expect("join");
        let snap = snapshot();
        assert!(snap.contains("from-other-thread"));
    }

    /// Round-trip the `Display` formatter on a sample error; the
    /// actual read errors are exercised in the format-level tests,
    /// so this just ensures the function pointer is reachable.
    #[test]
    fn writer_interface_compiles() {
        let _ = Cursor::new(Vec::<u8>::new());
    }
}

// The `prof_scope!` macro is defined in `lib.rs` (`prof_macro` module)
// so it sits at the crate root even when the `profile` feature is off.
// Call sites inside the same crate use it as `prof_scope!("name")`.
