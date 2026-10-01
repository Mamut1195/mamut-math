//! Genetic operators: SBX crossover, polynomial mutation, `varOr`, adaptive eta.
//!
//! Semantically equivalent to DEAP `tools.cxSimulatedBinaryBounded`,
//! `tools.mutPolynomialBounded` and `algorithms.varOr`, including the
//! adaptive-eta linear schedule.
//!
//! Extracted from the frozen `operators.rs` with the signature transplant
//! `&[GeneSpec]` → `&[Variable]` and `&mut ChaCha20Rng` → `&mut dyn RngPort`.
//! Integer genes are stored as raw floats in the chromosome; SBX and polynomial
//! mutation operate on raw float values (matches the frozen semantics; the
//! caller decodes/rounds).
//!
//! ## Documented DEAP divergences (preserved verbatim, H4)
//! - `var_or` keeps **both** SBX children when there is room (DEAP retains only
//!   child 1 per crossover step).
//! - `polynomial_mutation` invalidates cached fitness **only when at least one
//!   gene actually changed** (DEAP always clears fitness after `mutate()`).

use mamut_math_core::{Candidate, Feasibility, MathError, ObjectiveValues, RngPort, Variable};

use crate::internal;

/// A single GA individual: a core [`Candidate`] plus optional cached fitness
/// and feasibility.
///
/// `fitness == None` means "not evaluated yet" or "invalidated by
/// crossover/mutation". `feasibility == None` has the same lifecycle.
#[derive(Clone, Debug, PartialEq)]
pub struct Individual {
    /// Raw decision vector (validated finite values; raw floats for integers).
    pub candidate: Candidate,
    /// Cached objective values, `None` when unevaluated/invalidated.
    pub fitness: Option<ObjectiveValues>,
    /// Cached feasibility classification, `None` when unevaluated/invalidated.
    pub feasibility: Option<Feasibility>,
}

impl Individual {
    /// Wrap a candidate as an unevaluated individual.
    #[must_use]
    pub fn new(candidate: Candidate) -> Self {
        Self {
            candidate,
            fitness: None,
            feasibility: None,
        }
    }

    /// Wrap an evaluated candidate.
    #[must_use]
    pub fn evaluated(
        candidate: Candidate,
        fitness: ObjectiveValues,
        feasibility: Feasibility,
    ) -> Self {
        Self {
            candidate,
            fitness: Some(fitness),
            feasibility: Some(feasibility),
        }
    }

    /// The raw `f64` genes of this individual.
    #[must_use]
    pub fn genes(&self) -> Vec<f64> {
        internal::genes_of(&self.candidate)
    }

    /// Whether this individual carries a cached fitness.
    #[must_use]
    pub fn is_evaluated(&self) -> bool {
        self.fitness.is_some()
    }

    /// Invalidate the cached fitness and feasibility (crossover/mutation).
    fn invalidate(&mut self) {
        self.fitness = None;
        self.feasibility = None;
    }
}

/// Linear interpolation of eta from `eta_min` to `eta_max` over `max_gen`
/// generations.
///
/// Mirrors the frozen `_adaptive_eta_value` / `adaptive_eta_value`:
/// `progress = generation / max(max_gen, 1)`,
/// `eta = eta_min + (eta_max - eta_min) * progress`.
#[must_use]
pub fn adaptive_eta_value(generation: u32, max_gen: u32, eta_min: f64, eta_max: f64) -> f64 {
    let progress = generation as f64 / max_gen.max(1) as f64;
    eta_min + (eta_max - eta_min) * progress
}

/// Simulated Binary Crossover (SBX) — bounded variant.
///
/// Ports `deap.tools.cxSimulatedBinaryBounded` (Deb & Agrawal, 1995). Operates
/// in place, invalidating cached fitness/feasibility on both offspring. When
/// the two parents are identical at a gene (`|x1 - x2| < 1e-14`) the gene is
/// skipped and no RNG draw is consumed (matches the frozen behavior).
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when `variables` is empty.
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] when a parent's arity
///   differs from `variables.len()`.
pub fn sbx_crossover(
    parent1: &mut Individual,
    parent2: &mut Individual,
    variables: &[Variable],
    eta: f64,
    rng: &mut dyn RngPort,
) -> Result<(), MathError> {
    let len = variables.len();
    if parent1.candidate.len() != len || parent2.candidate.len() != len {
        return Err(MathError::new(
            mamut_math_core::ErrorKind::DimensionMismatch,
            "sbx_crossover: chromosome length does not match variable count",
        ));
    }
    if len == 0 {
        return Err(MathError::new(
            mamut_math_core::ErrorKind::EmptyCollection,
            "sbx_crossover: at least one variable is required",
        ));
    }

    let mut g1 = internal::genes_of(&parent1.candidate);
    let mut g2 = internal::genes_of(&parent2.candidate);

    for (i, variable) in variables.iter().enumerate() {
        let (lb, ub) = internal::variable_bounds(variable);
        let x1 = g1[i];
        let x2 = g2[i];

        // When parents are identical at this gene, skip (DEAP behavior).
        if (x1 - x2).abs() < 1e-14 {
            continue;
        }

        // Ensure x_lo <= x_hi for the formula.
        let (x_lo, x_hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let dx = x_hi - x_lo;

        let u = rng.next_unit_f64();

        let beta1 = 1.0 + 2.0 * (x_lo - lb) / dx;
        let alpha1 = 2.0 - beta1.powf(-(eta + 1.0));
        let betaq1 = sbx_betaq(u, alpha1, eta);

        let beta2 = 1.0 + 2.0 * (ub - x_hi) / dx;
        let alpha2 = 2.0 - beta2.powf(-(eta + 1.0));
        let betaq2 = sbx_betaq(u, alpha2, eta);

        let c1 = (0.5 * ((x_lo + x_hi) - betaq1 * dx)).clamp(lb, ub);
        let c2 = (0.5 * ((x_lo + x_hi) + betaq2 * dx)).clamp(lb, ub);

        // Assign back preserving original ordering.
        if x1 <= x2 {
            g1[i] = c1;
            g2[i] = c2;
        } else {
            g1[i] = c2;
            g2[i] = c1;
        }
    }

    parent1.candidate = internal::candidate_from_f64(g1)?;
    parent2.candidate = internal::candidate_from_f64(g2)?;
    parent1.invalidate();
    parent2.invalidate();
    Ok(())
}

/// Compute the SBX spread factor `betaq` from a uniform sample `u` and `alpha`.
fn sbx_betaq(u: f64, alpha: f64, eta: f64) -> f64 {
    if u <= 1.0 / alpha {
        (u * alpha).powf(1.0 / (eta + 1.0))
    } else {
        (1.0 / (2.0 - u * alpha)).powf(1.0 / (eta + 1.0))
    }
}

/// Polynomial mutation — bounded variant.
///
/// Ports `deap.tools.mutPolynomialBounded` (Deb, 2001). Each gene is mutated
/// independently with probability `indpb = 1 / num_genes`. Operates in place,
/// invalidating cached fitness/feasibility **only when at least one gene was
/// actually changed** (frozen divergence from DEAP, preserved).
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when `variables` is empty.
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] on arity mismatch.
pub fn polynomial_mutation(
    individual: &mut Individual,
    variables: &[Variable],
    eta: f64,
    rng: &mut dyn RngPort,
) -> Result<(), MathError> {
    let len = variables.len();
    if individual.candidate.len() != len {
        return Err(MathError::new(
            mamut_math_core::ErrorKind::DimensionMismatch,
            "polynomial_mutation: chromosome length does not match variable count",
        ));
    }
    if len == 0 {
        return Err(MathError::new(
            mamut_math_core::ErrorKind::EmptyCollection,
            "polynomial_mutation: at least one variable is required",
        ));
    }

    let indpb = 1.0 / len.max(1) as f64;
    let mut genes = internal::genes_of(&individual.candidate);
    let mut mutated = false;

    for (i, variable) in variables.iter().enumerate() {
        if rng.next_unit_f64() > indpb {
            continue;
        }
        mutated = true;

        let (lb, ub) = internal::variable_bounds(variable);
        let x = genes[i];
        let dx = ub - lb;

        if dx < 1e-14 {
            continue; // degenerate gene — leave unchanged
        }

        let u = rng.next_unit_f64();
        let delta = poly_mutation_delta(u, x, lb, ub, eta, dx);
        genes[i] = (x + delta * dx).clamp(lb, ub);
    }

    individual.candidate = internal::candidate_from_f64(genes)?;
    if mutated {
        individual.invalidate();
    }
    Ok(())
}

/// Compute normalised polynomial mutation delta (scaled by range `dx`).
///
/// Implements the DEAP `mutPolynomialBounded` inner formula; the caller
/// multiplies the result by `dx`.
fn poly_mutation_delta(u: f64, x: f64, lb: f64, ub: f64, eta: f64, dx: f64) -> f64 {
    if u < 0.5 {
        let delta_l = (x - lb) / dx;
        let base = 2.0 * u + (1.0 - 2.0 * u) * (1.0 - delta_l).powf(eta + 1.0);
        base.powf(1.0 / (eta + 1.0)) - 1.0
    } else {
        let delta_r = (ub - x) / dx;
        let base = 2.0 * (1.0 - u) + 2.0 * (u - 0.5) * (1.0 - delta_r).powf(eta + 1.0);
        1.0 - base.powf(1.0 / (eta + 1.0))
    }
}

/// `μ+λ` offspring production operator.
///
/// Semantically equivalent to `deap.algorithms.varOr` (NOT byte-identical):
/// - With probability `cxpb`: pick two parents, clone, apply SBX, and push
///   **both** children when there is room (frozen divergence from DEAP, kept).
/// - With probability `mutpb`: clone one parent and apply polynomial mutation.
/// - Otherwise: clone one parent unchanged (reproduction).
///
/// Produces exactly `lambda` offspring. The RNG consumption sequence mirrors
/// the frozen `var_or` exactly (one `next_unit_f64` per iteration for the
/// branch decision; `gen_index` draws for parent selection).
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when `population` or
///   `variables` is empty.
/// - [`mamut_math_core::ErrorKind::InvertedBounds`] when `cxpb + mutpb > 1`.
/// - [`mamut_math_core::ErrorKind::DimensionMismatch`] on arity mismatch.
pub fn var_or(
    population: &[Individual],
    variables: &[Variable],
    lambda: usize,
    cxpb: f64,
    mutpb: f64,
    eta: f64,
    rng: &mut dyn RngPort,
) -> Result<Vec<Individual>, MathError> {
    if cxpb + mutpb > 1.0 + 1e-9 {
        return Err(MathError::new(
            mamut_math_core::ErrorKind::InvertedBounds,
            format!("var_or: cxpb + mutpb must be <= 1.0, got {}", cxpb + mutpb),
        ));
    }
    if population.is_empty() {
        return Err(MathError::new(
            mamut_math_core::ErrorKind::EmptyCollection,
            "var_or: population must not be empty",
        ));
    }
    if variables.is_empty() {
        return Err(MathError::new(
            mamut_math_core::ErrorKind::EmptyCollection,
            "var_or: at least one variable is required",
        ));
    }

    let mut offspring: Vec<Individual> = Vec::with_capacity(lambda);
    let n = population.len();

    while offspring.len() < lambda {
        let u = rng.next_unit_f64();

        if u < cxpb {
            // Crossover.
            let i1 = internal::gen_index(rng, n);
            let i2 = internal::gen_index(rng, n);
            let mut child1 = population[i1].clone();
            let mut child2 = population[i2].clone();
            sbx_crossover(&mut child1, &mut child2, variables, eta, rng)?;
            offspring.push(child1);
            if offspring.len() < lambda {
                offspring.push(child2);
            }
        } else if u < cxpb + mutpb {
            // Mutation.
            let i = internal::gen_index(rng, n);
            let mut child = population[i].clone();
            polynomial_mutation(&mut child, variables, eta, rng)?;
            offspring.push(child);
        } else {
            // Reproduction — exact clone.
            let i = internal::gen_index(rng, n);
            offspring.push(population[i].clone());
        }
    }

    Ok(offspring)
}

/// Generate an initial population of `pop_size` random individuals.
///
/// Uniform-box sampling per [`mamut_math_core::VariableKind`]:
/// - `Continuous`/`Integer`: uniform in the closed `[lower, upper]` interval,
///   as raw `f64` (frozen semantics).
/// - `Discrete`: a uniform index draw over the validated set (exact member).
/// - `Categorical`: a uniform index draw over `0..cardinality`, as `f64`.
///
/// All returned individuals have `fitness == None` / `feasibility == None`.
///
/// # Errors
/// - [`mamut_math_core::ErrorKind::EmptyCollection`] when `variables` is empty.
pub fn init_population(
    variables: &[Variable],
    pop_size: usize,
    rng: &mut dyn RngPort,
) -> Result<Vec<Individual>, MathError> {
    if variables.is_empty() {
        return Err(MathError::new(
            mamut_math_core::ErrorKind::EmptyCollection,
            "init_population: at least one variable is required",
        ));
    }

    let mut population = Vec::with_capacity(pop_size);
    for _ in 0..pop_size {
        let mut genes = Vec::with_capacity(variables.len());
        for variable in variables {
            genes.push(sample_gene(variable, rng));
        }
        population.push(Individual::new(internal::candidate_from_f64(genes)?));
    }
    Ok(population)
}

/// Sample a single raw gene value for a variable (see [`init_population`]).
fn sample_gene(variable: &Variable, rng: &mut dyn RngPort) -> f64 {
    match variable.kind() {
        mamut_math_core::VariableKind::Discrete { values } => {
            let idx = internal::gen_index(rng, values.len());
            values.values()[idx].get()
        }
        mamut_math_core::VariableKind::Categorical { cardinality } => {
            internal::gen_index(rng, *cardinality as usize) as f64
        }
        _ => {
            let (lb, ub) = internal::variable_bounds(variable);
            lb + (ub - lb) * rng.next_unit_f64()
        }
    }
}
