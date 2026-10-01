#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! # MAMUT Math — Generic NSGA-III (v0.1)
//!
//! Domain-agnostic, dependency-free extraction of the reference-point NSGA-III
//! selection machinery (Deb & Jain 2014) and the associated genetic operators
//! (SBX crossover, polynomial mutation, `varOr`, adaptive-eta schedule) into a
//! reusable MAMUT-owned library.
//!
//! ## Scope and invariants
//! - **Dynamic objective count** `M`: every kernel validates `M >= 1`, uniform
//!   arity and finiteness at call time, returning a typed
//!   [`mamut_math_core::MathError`]. There is no fixed-5 and no const-generic
//!   `M`.
//! - **f64 canonical** for V0.x: the numeric kernels operate on plain `f64`
//!   (`Vec<f64>`/`&[f64]`); the session boundary converts from/to core
//!   `Candidate`/`ObjectiveValues`.
//! - **Deterministic RNG only**: all stochasticity flows through the frozen core
//!   [`mamut_math_core::rng::RngPort`] / [`mamut_math_core::rng::MamutRng`]
//!   contract. No `rand`/`rand_chacha` dependency and no hidden RNG state.
//! - **MAMUT-owned public API**: no third-party types cross the crate boundary;
//!   the crate depends only on `mamut-math-core` and `std`.
//!
//! ## Module map
//! - [`reference_points`] — Das-Dennis simplex lattice.
//! - [`nondom_sort`] — Deb 2002 fast nondominated sort, first-front extraction
//!   and nondominated-set validation.
//! - [`mod@normalize`] — extreme-point hyperplane intercept normalization.
//! - [`metrics`] — generic Pareto quality metrics (exact hypervolume, IGD+,
//!   unit-box normalization); all objectives minimized.
//! - [`niching`] — association, niche counts and partial-front selection.
//! - [`selection`] — environmental selection wrapper.
//! - [`operators`] — SBX, polynomial mutation, `varOr`, adaptive eta, init.
//! - [`config`] — [`config::Nsga3Config`].
//! - [`session`] — ask/tell [`session::Nsga3Session`].
//! - [`error`] — the core [`mamut_math_core::MathError`] error surface.

pub mod config;
pub mod error;
pub mod metrics;
pub mod niching;
pub mod nondom_sort;
pub mod normalize;
pub mod operators;
pub mod reference_points;
pub mod selection;
pub mod session;

mod internal;

pub use config::Nsga3Config;
pub use error::{ErrorKind, MathError};
pub use metrics::{denormalize_from_unit_box, hypervolume, igd_plus, normalize_to_unit_box};
pub use niching::{associate, build_niche_counts, perp_distance, select_from_partial_front};
pub use nondom_sort::{
    dominates, extract_pareto_front, fast_nondominated_sort, is_nondominated_set,
};
pub use normalize::{normalize, NormalizeResult};
pub use operators::{
    adaptive_eta_value, init_population, polynomial_mutation, sbx_crossover, var_or, Individual,
};
pub use reference_points::uniform_reference_points;
pub use selection::select_environmental;
pub use session::{Nsga3Session, SessionProgress};
