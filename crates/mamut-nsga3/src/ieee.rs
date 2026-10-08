//! Explicit IEEE-754 `f64` (raw) variation route.
//!
//! This is the **compatibility low-level** entry point of `mamut-nsga3`. It
//! exposes SBX crossover, polynomial mutation and `varOr` over raw `f64` gene
//! vectors, preserving unsanitised `NaN`/`±Inf` intermediate genes instead of
//! rejecting them through the finite [`mamut_math_core::Candidate`] contract.
//!
//! ## Contract boundary (not an engineering-validity promise)
//! - [`crate::operators`] keeps the checked path: every gene is validated
//!   through [`mamut_math_core::ValidFloat`] and a non-finite result is reported
//!   as [`mamut_math_core::ErrorKind::InvalidNotFinite`].
//! - This module is a *compatibility* route: it shares the **same** arithmetic
//!   body, structural/probability guards, RNG consumption order and cache
//!   invalidations as the checked path, but it does **not** finite-validate the
//!   genes or cached fitness it stores.
//! - `NaN` ordering, `NaN`-in/`NaN`-out propagation and any downstream
//!   engineering or objective validity are **not** promised here. A consumer
//!   that needs finite genes must validate them itself or keep using the typed
//!   path. Structural shape and probability guards are preserved exactly,
//!   including typed [`mamut_math_core::MathError`] results.

use mamut_math_core::{Feasibility, MathError, RngPort, Variable};

use crate::operators;

/// A raw `f64` individual for the IEEE compatibility route.
///
/// Unlike [`crate::operators::Individual`], genes are **not** finite-checked: a
/// gene may hold `NaN`/`±Inf`. The cached fitness/feasibility mirror the typed
/// lifecycle (`None` means unevaluated or invalidated by variation); the
/// feasibility cache is a MAMUT type, never a third-party type.
#[derive(Clone, Debug, PartialEq)]
pub struct Individual {
    /// Raw decision genes (no finite validation; raw floats for integers).
    pub genes: Vec<f64>,
    /// Cached raw objective values, `None` when unevaluated/invalidated.
    pub fitness: Option<Vec<f64>>,
    /// Cached MAMUT feasibility, `None` when unevaluated/invalidated.
    pub feasibility: Option<Feasibility>,
}

impl Individual {
    /// Wrap raw genes as an unevaluated individual.
    #[must_use]
    pub fn new(genes: Vec<f64>) -> Self {
        Self {
            genes,
            fitness: None,
            feasibility: None,
        }
    }
}

impl operators::KernelIndividual for Individual {
    fn arity(&self) -> usize {
        self.genes.len()
    }

    fn raw_genes(&self) -> Vec<f64> {
        self.genes.clone()
    }

    fn assign_genes(&mut self, genes: Vec<f64>) -> Result<(), MathError> {
        self.genes = genes;
        Ok(())
    }

    fn invalidate_cache(&mut self) {
        self.fitness = None;
        self.feasibility = None;
    }
}

/// Simulated Binary Crossover (SBX) over raw `f64` genes.
///
/// Shares the single arithmetic body, gene-skip rule, draw order and cache
/// invalidation of [`crate::operators::sbx_crossover`]; the only difference is
/// that offspring genes are stored **without** finite validation, so a
/// `NaN`/`±Inf` result is preserved rather than rejected.
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] when a parent's arity
///   differs from `variables.len()`.
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when `variables` is empty
///   and both parents are empty.
pub fn sbx_crossover(
    parent1: &mut Individual,
    parent2: &mut Individual,
    variables: &[Variable],
    eta: f64,
    rng: &mut dyn RngPort,
) -> Result<(), MathError> {
    operators::sbx_crossover_kernel(parent1, parent2, variables, eta, rng)
}

/// Polynomial mutation over raw `f64` genes.
///
/// Shares the single arithmetic body, probability gate, draw order and
/// invalidation rule of [`crate::operators::polynomial_mutation`]; the only
/// difference is that genes are stored **without** finite validation.
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] when the chromosome's
///   arity differs from `variables.len()`.
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when `variables` is empty
///   and the chromosome is empty.
pub fn polynomial_mutation(
    individual: &mut Individual,
    variables: &[Variable],
    eta: f64,
    rng: &mut dyn RngPort,
) -> Result<(), MathError> {
    operators::polynomial_mutation_kernel(individual, variables, eta, rng)
}

/// `μ+λ` offspring production over raw `f64` individuals.
///
/// Shares the single arithmetic body, `cxpb + mutpb` guard, branch order, RNG
/// consumption and clone semantics of [`crate::operators::var_or`], including
/// keeping **both** SBX children when there is room. The only difference is that
/// children are stored **without** finite validation.
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::InvertedBounds`] when `cxpb + mutpb > 1`.
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when `population` or
///   `variables` is empty.
/// - The SBX/mutation structural error of the selected branch.
pub fn var_or(
    population: &[Individual],
    variables: &[Variable],
    lambda: usize,
    cxpb: f64,
    mutpb: f64,
    eta: f64,
    rng: &mut dyn RngPort,
) -> Result<Vec<Individual>, MathError> {
    operators::var_or_kernel(population, variables, lambda, cxpb, mutpb, eta, rng)
}
