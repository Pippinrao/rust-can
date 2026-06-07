//! Crate-root macro for declaring a profiling scope.
//!
//! Lives at the crate root (rather than inside the `prof` module) so it
//! resolves to `crate::prof_scope!` from any module without `use`
//! gymnastics. The body is feature-gated: when the `profile` feature
//! is off, the macro expands to a single no-op binding, so call sites
//! pay no runtime cost.

/// Declare a profiling scope. The argument must be a string literal
/// that names the current function or code block. See the
/// `rust-can-io` README for the data flow.
#[cfg(feature = "profile")]
#[macro_export]
macro_rules! prof_scope {
    ($name:expr) => {
        let _prof = $crate::prof::Scope::new($name);
    };
}

/// Declare a profiling scope (no-op when the `profile` feature is off).
#[cfg(not(feature = "profile"))]
#[macro_export]
macro_rules! prof_scope {
    ($name:expr) => {
        // Profiling disabled: emit nothing. The `_name` binding is
        // not used; the binding avoids an unused-variable warning
        // when the call site binds `_prof`.
        let _ = $name;
    };
}
