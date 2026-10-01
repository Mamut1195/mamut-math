//! Generic GA configuration for the NSGA-III session.
//!
//! Contains only generic evolutionary controls. It deliberately has **no**
//! domain defaults: no spatial zones, no named objective weights, no norm
//! strings, no wall-clock budgets and no hydraulic policy.

use mamut_math_core::{ErrorKind, MathError, Seed, Variable};

/// Generic NSGA-III session configuration.
#[derive(Clone, Debug)]
pub struct Nsga3Config {
    /// Population size (`>= 1`).
    pub population_size: usize,
    /// Logical generation budget; `0` means an evaluate-only run.
    pub generations: u32,
    /// Crossover probability in `[0, 1]`.
    pub crossover_prob: f64,
    /// Mutation probability in `[0, 1]` (per-offspring, for `var_or`).
    pub mutation_prob: f64,
    /// Lower bound of the adaptive-eta schedule (`> 0`).
    pub eta_min: f64,
    /// Upper bound of the adaptive-eta schedule (`>= eta_min`).
    pub eta_max: f64,
    /// Das-Dennis divisions `p >= 1` (caller policy; e.g. 4 for 5 objectives).
    pub divisions: usize,
    /// Root seed for the frozen core RNG contract.
    pub seed: Seed,
    /// Caller-supplied variable boxes/dtypes (replaces any gene-spec table).
    pub variables: Vec<Variable>,
}

impl Nsga3Config {
    /// Build and validate a configuration.
    ///
    /// # Errors
    /// See [`Nsga3Config::validate`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        population_size: usize,
        generations: u32,
        crossover_prob: f64,
        mutation_prob: f64,
        eta_min: f64,
        eta_max: f64,
        divisions: usize,
        seed: Seed,
        variables: Vec<Variable>,
    ) -> Result<Self, MathError> {
        let config = Self {
            population_size,
            generations,
            crossover_prob,
            mutation_prob,
            eta_min,
            eta_max,
            divisions,
            seed,
            variables,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validate every invariant of this configuration.
    ///
    /// # Errors
    /// - [`ErrorKind::EmptyCollection`] for `population_size == 0`,
    ///   `divisions == 0` or empty `variables`.
    /// - [`ErrorKind::DomainMismatch`] for a probability outside `[0, 1]` or a
    ///   non-positive eta bound.
    /// - [`ErrorKind::InvertedBounds`] for `crossover_prob + mutation_prob > 1`
    ///   or `eta_min > eta_max`.
    pub fn validate(&self) -> Result<(), MathError> {
        if self.population_size == 0 {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "population_size must be >= 1",
            ));
        }
        validate_probability(self.crossover_prob, "crossover_prob")?;
        validate_probability(self.mutation_prob, "mutation_prob")?;
        if self.crossover_prob + self.mutation_prob > 1.0 + 1e-9 {
            return Err(MathError::new(
                ErrorKind::InvertedBounds,
                "crossover_prob + mutation_prob must be <= 1.0",
            ));
        }
        if self.eta_min.is_nan() || self.eta_min <= 0.0 {
            return Err(MathError::new(
                ErrorKind::DomainMismatch,
                "eta_min must be > 0",
            ));
        }
        if self.eta_max.is_nan() || self.eta_max <= 0.0 {
            return Err(MathError::new(
                ErrorKind::DomainMismatch,
                "eta_max must be > 0",
            ));
        }
        if self.eta_min > self.eta_max {
            return Err(MathError::new(
                ErrorKind::InvertedBounds,
                "eta_min must be <= eta_max",
            ));
        }
        if self.divisions == 0 {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "divisions must be >= 1",
            ));
        }
        if self.variables.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "variables must not be empty",
            ));
        }
        for variable in &self.variables {
            variable.kind().validate()?;
        }
        Ok(())
    }
}

/// Validate that a probability is finite and within `[0, 1]`.
fn validate_probability(value: f64, name: &str) -> Result<(), MathError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(MathError::new(
            ErrorKind::DomainMismatch,
            format!("{name} must be within [0, 1]"),
        ))
    }
}
