//! Crate error surface.
//!
//! The crate deliberately has **no error type of its own**. Every fallible
//! entry point returns the MAMUT-owned core [`MathError`], so the failure
//! taxonomy stays single-sourced with `mamut-math-core` (documented deviation
//! from extraction-contract §B1 row 9, which sketched an optional
//! `Nsga3Error` wrapper; the binding orchestration decision and the §C API
//! sketches use `MathError` directly).
//!
//! Failure kinds used by this crate:
//! - [`ErrorKind::EmptyCollection`] — `M == 0`, no variables, no refs.
//! - [`ErrorKind::DimensionMismatch`] — ragged/mismatched arity.
//! - [`ErrorKind::InvalidNotFinite`] — `NaN`/`±Inf` where finite is required.
//! - [`ErrorKind::InvertedBounds`] / [`ErrorKind::DomainMismatch`] — config
//!   invariants (probability ranges, `cxpb + mutpb <= 1`, eta ordering).

pub use mamut_math_core::{ErrorKind, MathError};
