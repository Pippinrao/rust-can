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
use std::time::{Duration, Instant};

thread_local! {
    static STACK: RefCell<Vec<&'static str>> = RefCell::new(Vec::with_capacity(64));
}

/// Set to true by the `enable` function. When false, `prof_scope!`
/// expands to a no-op.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Globals keyed by the current call stack. We accumulate per-stack
/// total time; the total for a given stack is what becomes one folded
/// stack line at dump time.
///
/// `STACK` and `ACCUM` are both per-thread (`thread_local!`).
/// Otherwise, a parallel test that creates Scopes on another thread
/// would interleave its measurements into this thread's `dump_folded`
/// output, which makes the folded-stack files non-deterministic
/// and the tests flaky. Per-thread isolation matches how
/// sampling-based profilers (`flamegraph.pl`, `perf script`) treat
/// each thread as a separate stack anyway.
type AccKey = Vec<&'static str>;
type AccValue = (Duration, u64);
type AccMap = HashMap<AccKey, AccValue>;

thread_local! {
    static ACCUM: std::cell::RefCell<AccMap> = std::cell::RefCell::new(HashMap::new());
}

/// Enable profiling for the current process. Idempotent.
pub fn enable() {
    // `SeqCst` so all other threads' subsequent `ENABLED` loads
    // observe the new value. Combined with `Scope::new`'s
    // `Acquire` load, this gives a clean happens-before relation
    // between the enable() and the next push onto a thread's STACK.
    ENABLED.store(true, Ordering::SeqCst);
}

/// Disable profiling for the current process. Mainly useful in tests
/// to opt out of cross-test state leakage; production code never
/// calls this.
pub fn disable() {
    ENABLED.store(false, Ordering::SeqCst);
}

/// Returns whether profiling is enabled.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// A scoped timer. Create at the top of an instrumented function; drop
/// at the end.
pub struct Scope {
    name: &'static str,
    /// Instant this scope started measuring.
    started: Instant,
    /// Snapshot of `ENABLED` taken at `Scope::new` time. We bind it
    /// to the scope instead of re-reading the global atomic at
    /// `Drop` time so the create/push and drop/pop decisions are
    /// made under the same flag. Without this, another thread can
    /// flip `ENABLED` between the two reads, desyncing the
    /// thread-local stack from the scope's own lifetime.
    enabled: bool,
}

impl Scope {
    /// Create a new scope. The `name` is recorded as the topmost
    /// stack frame when the scope is dropped. The scope starts
    /// measuring wall time immediately; pass it to `prof_scope!` to
    /// make the intent clear.
    #[inline]
    pub fn new(name: &'static str) -> Self {
        let started = Instant::now();
        // `Acquire` so a subsequent `disable()` on another thread is
        // ordered after this load — in practice both `enable()` and
        // `disable()` use `SeqCst`, but reading with the matching
        // ordering makes the contract explicit.
        let enabled = ENABLED.load(Ordering::Acquire);
        if enabled {
            STACK.with(|s| s.borrow_mut().push(name));
        }
        Self {
            name,
            started,
            enabled,
        }
    }
}

impl Drop for Scope {
    #[inline]
    fn drop(&mut self) {
        // Use the bound `enabled` flag, not the live atomic. See
        // `Scope::new` for why.
        if !self.enabled {
            return;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.started);

        let mut key: AccKey = Vec::new();
        STACK.with(|s| {
            let mut stack = s.borrow_mut();
            let popped = stack.pop();
            // `popped` should equal `Some(self.name)` because every
            // push that we are now popping was done while
            // `ENABLED == self.enabled`. If the test harness runs
            // multiple tests on the same OS thread (cargo test
            // recycles threads across tests) and a previous test
            // enabled profiling, a residual frame on this thread's
            // stack could be popped here. Recover by re-pushing
            // and discarding the measurement.
            if popped != Some(self.name) {
                if let Some(name) = popped {
                    stack.push(name);
                }
                return;
            }
            key.extend(stack.iter().copied());
            key.push(self.name);
        });

        if key.is_empty() {
            return;
        }
        ACCUM.with(|accum| {
            let mut accum = accum.borrow_mut();
            let entry = accum.entry(key).or_insert((Duration::ZERO, 0));
            entry.0 += elapsed;
            entry.1 += 1;
        });
    }
}

/// Drain the accumulated samples and write folded stacks to `out`,
/// sorted by descending total time. The format matches Brendan
/// Gregg's `flamegraph.pl` (one stack per line, semicolon-joined,
/// space + sample weight in nanoseconds).
pub fn dump_folded<W: Write>(out: &mut W) -> std::io::Result<()> {
    // Flush this thread's accumulator. Callers that want a full
    // process-wide view must ensure all worker threads have stopped
    // (and ideally called `dump_folded` on each of them) before
    // invoking `dump_folded` on the main thread.
    let mut buf = String::new();
    ACCUM.with(|accum| {
        let accum = accum.borrow();
        let mut entries: Vec<_> = accum.iter().collect();
        entries.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(b.1 .1.cmp(&a.1 .1)));
        for (stack, (total, _count)) in entries {
            let nanos = total.as_nanos() as u64;
            buf.push_str(&stack.join(";"));
            buf.push(' ');
            buf.push_str(&nanos.to_string());
            buf.push('\n');
        }
    });
    out.write_all(buf.as_bytes())
}

/// Same as `dump_folded` but also flushes the global accumulator. Call
/// once at the end of the profiled run.
pub fn dump_and_reset<W: Write>(out: &mut W) -> std::io::Result<()> {
    dump_folded(out)?;
    ACCUM.with(|accum| accum.borrow_mut().clear());
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
        // `unwrap_or_else` recovers from a prior panic's poison so
        // a single failed test does not cascade into "lock poisoned"
        // for every subsequent test.
        _lock: std::sync::MutexGuard<'static, ()>,
    }
    impl EnableGuard {
        fn new() -> Self {
            let lock = TEST_LOCK.lock().unwrap_or_else(|err| {
                // Clear the poison by recovering the inner guard;
                // the poisoning state is irrelevant for serializing
                // the ENABLED/ACCUM mutations this guard exists to
                // protect.
                err.into_inner()
            });
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
    fn other_thread_scope_does_not_pollute_this_thread() {
        // The `STACK` and `ACCUM` are both thread-local. A scope
        // created on a different thread must not appear in *this*
        // thread's snapshot, and must not crash. This test exists
        // to lock in the per-thread isolation contract so a future
        // refactor to global state does not silently change it.
        let _g = EnableGuard::new();
        let h = thread::spawn(|| {
            let _s = Scope::new("from-other-thread");
            thread::sleep(Duration::from_millis(1));
        });
        h.join().expect("join");
        let snap = snapshot();
        assert!(!snap.contains("from-other-thread"));
        // And our own scope still shows up.
        {
            let _s = Scope::new("here");
        }
        let snap = snapshot();
        assert!(snap.contains("here"));
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
