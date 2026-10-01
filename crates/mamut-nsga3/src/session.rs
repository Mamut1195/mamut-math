//! Ask/tell NSGA-III session.
//!
//! A `μ+λ` generational loop over the generic NSGA-III kernels. The session is
//! deterministic: all stochasticity flows through the root [`MamutRng`] built
//! from [`Nsga3Config::seed`], and per-generation offspring streams are derived
//! with [`MamutRng::derive_child`] (keyed, state-independent).
//!
//! ## Loop
//! - [`Nsga3Session::new`] builds the root stream and the initial population.
//! - [`Nsga3Session::ask`] returns the unevaluated candidates.
//! - [`Nsga3Session::tell`] assigns evaluation outcomes; once the pool is fully
//!   evaluated it runs environmental selection (`n = population_size`) and
//!   produces the next offspring generation (until the budget is reached).
//! - [`Nsga3Session::step`] is the convenience `ask → evaluate → tell`.
//!
//! Objective count `M` is discovered from the first successful evaluation and
//! enforced uniform thereafter (dynamic-`M`, no fixed arity).
//!
//! Stop reasons map onto the core taxonomy: generation budget →
//! [`StopReason::IterationBudgetExhausted`], a typed internal failure or an
//! evaluation [`EvaluationOutcome::Failure`] → [`StopReason::Failed`], a caller
//! [`Nsga3Session::abort`] → [`StopReason::Aborted`]. [`StopReason::Converged`]
//! is never emitted (no convergence criterion this ticket) and wall-clock is
//! intentionally not used.

use mamut_math_core::rng::MamutRng;
use mamut_math_core::{Candidate, ErrorKind, EvaluationOutcome, MathError, StopReason};

use crate::config::Nsga3Config;
use crate::internal;
use crate::operators::{adaptive_eta_value, init_population, var_or, Individual};
use crate::reference_points::uniform_reference_points;
use crate::selection::select_environmental;

/// Snapshot of session progress after an `ask`/`tell`/`step` transition.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionProgress {
    /// Current generation counter.
    pub generation: u32,
    /// Total number of successful evaluations recorded.
    pub evaluations: u64,
    /// Terminal reason, if the session has stopped.
    pub stop_reason: Option<StopReason>,
    /// Indices (into the pre-selection pool) chosen by the last environmental
    /// selection.
    pub selected_indices: Vec<usize>,
}

/// MAMUT-owned ask/tell NSGA-III session.
pub struct Nsga3Session {
    config: Nsga3Config,
    rng: MamutRng,
    population: Vec<Individual>,
    generation: u32,
    evaluations: u64,
    selected_indices: Vec<usize>,
    stop_reason: Option<StopReason>,
    objectives: Option<usize>,
    ref_points: Vec<Vec<f64>>,
    failure: Option<MathError>,
}

impl Nsga3Session {
    /// Build a session from a validated configuration.
    ///
    /// # Errors
    /// - Propagates [`Nsga3Config::validate`] failures.
    /// - Propagates [`init_population`] failures.
    pub fn new(config: Nsga3Config) -> Result<Self, MathError> {
        config.validate()?;
        let mut rng = MamutRng::root(&config.seed);
        let population = init_population(&config.variables, config.population_size, &mut rng)?;
        let selected_indices = (0..population.len()).collect();
        Ok(Self {
            config,
            rng,
            population,
            generation: 0,
            evaluations: 0,
            selected_indices,
            stop_reason: None,
            objectives: None,
            ref_points: Vec::new(),
            failure: None,
        })
    }

    /// The unevaluated candidates that still need objectives, in pool order.
    #[must_use]
    pub fn ask(&self) -> Vec<Candidate> {
        self.population
            .iter()
            .filter(|individual| individual.fitness.is_none())
            .map(|individual| individual.candidate.clone())
            .collect()
    }

    /// Assign evaluation outcomes to the pending candidates.
    ///
    /// `outcomes` must be in the same order as the most recent [`Self::ask`].
    /// A [`EvaluationOutcome::Failure`] is recorded as a typed failure (never
    /// mapped to a sentinel fitness) and stops the session with
    /// [`StopReason::Failed`].
    ///
    /// # Errors
    /// - [`ErrorKind::DimensionMismatch`] when `outcomes.len()` does not match
    ///   the number of pending candidates, or when an objective arity differs
    ///   from the arity established by the first successful evaluation.
    pub fn tell(&mut self, outcomes: Vec<EvaluationOutcome>) -> Result<SessionProgress, MathError> {
        if self.stop_reason.is_some() {
            return Ok(self.progress());
        }

        let pending: Vec<usize> = self
            .population
            .iter()
            .enumerate()
            .filter(|(_, individual)| individual.fitness.is_none())
            .map(|(index, _)| index)
            .collect();

        if outcomes.len() != pending.len() {
            return Err(MathError::new(
                ErrorKind::DimensionMismatch,
                format!(
                    "tell: expected {} evaluation outcomes, received {}",
                    pending.len(),
                    outcomes.len()
                ),
            ));
        }

        for (index, outcome) in pending.into_iter().zip(outcomes) {
            match outcome {
                EvaluationOutcome::Success {
                    objectives,
                    feasibility,
                    ..
                } => {
                    let m = objectives.len();
                    match self.objectives {
                        None => {
                            self.ref_points = uniform_reference_points(m, self.config.divisions)?;
                            self.objectives = Some(m);
                        }
                        Some(existing) if existing != m => {
                            return Err(MathError::new(
                                ErrorKind::DimensionMismatch,
                                "tell: objective arity changed between evaluations",
                            ));
                        }
                        Some(_) => {}
                    }
                    self.population[index].fitness = Some(objectives);
                    self.population[index].feasibility = Some(feasibility);
                    self.evaluations += 1;
                }
                EvaluationOutcome::Failure { error } => {
                    self.stop_reason = Some(StopReason::Failed);
                    self.failure = Some(error);
                }
            }
        }

        if self.stop_reason.is_some() {
            return Ok(self.progress());
        }

        // Still waiting for more evaluations.
        if self
            .population
            .iter()
            .any(|individual| individual.fitness.is_none())
        {
            return Ok(self.progress());
        }

        self.advance()?;
        Ok(self.progress())
    }

    /// Convenience generation step: `ask → evaluate → tell`.
    ///
    /// # Errors
    /// Propagates [`Self::tell`].
    pub fn step(
        &mut self,
        evaluate: &mut dyn FnMut(&[Candidate]) -> Vec<EvaluationOutcome>,
    ) -> Result<SessionProgress, MathError> {
        if self.stop_reason.is_some() {
            return Ok(self.progress());
        }
        let pending = self.ask();
        if pending.is_empty() {
            return Ok(self.progress());
        }
        let outcomes = evaluate(&pending);
        self.tell(outcomes)
    }

    /// Select survivors and (budget permitting) generate the next offspring.
    ///
    /// Survivors are drawn **only** from admissible individuals
    /// ([`mamut_math_core::Feasibility::Feasible`] / `SoftViolation`):
    /// hard-infeasible individuals are never backfilled into the next
    /// generation.
    ///
    /// Disposition when the admissible count is below the configured
    /// population size `n`:
    /// - `admissible >= n`: environmental selection runs over the admissible
    ///   subset and returns exactly `n` survivors (unchanged normal path).
    /// - `2 <= admissible < n`: all admissible individuals are kept as-is (no
    ///   backfill); the next generation is reproduced from them.
    /// - `admissible < 2` (zero, or a single individual that cannot reproduce a
    ///   next generation): the session terminates with a typed
    ///   [`ErrorKind::EmptyCollection`] failure and records
    ///   [`StopReason::Failed`]. This is a failure, is never a normal
    ///   completion, and [`StopReason::Converged`] is never emitted.
    ///
    /// # Errors
    /// - [`ErrorKind::EmptyCollection`] when fewer than two admissible
    ///   individuals remain.
    /// - Propagates [`crate::selection::select_environmental`] and
    ///   [`crate::operators::var_or`] failures.
    fn advance(&mut self) -> Result<(), MathError> {
        let n = self.config.population_size;

        // Fully-evaluated pool; all entries have a fitness at this point.
        let fits: Vec<Vec<f64>> = self
            .population
            .iter()
            .filter_map(|individual| individual.fitness.as_ref())
            .map(internal::objective_vector)
            .collect();

        // Admissible individuals (Feasible / SoftViolation) are preferred; the
        // generic form of the frozen "fitness[0] < threshold" gate, without its
        // domain constant.
        let admissible: Vec<usize> = self
            .population
            .iter()
            .enumerate()
            .filter(|(_, individual)| {
                individual
                    .feasibility
                    .as_ref()
                    .is_none_or(mamut_math_core::Feasibility::is_admissible)
            })
            .map(|(index, _)| index)
            .collect();

        let survivors: Vec<usize> = if admissible.len() >= n {
            let admissible_fits: Vec<Vec<f64>> =
                admissible.iter().map(|&i| fits[i].clone()).collect();
            let mut selection_rng = self.rng.derive_child(self.generation as u64);
            let local =
                select_environmental(&admissible_fits, n, &self.ref_points, &mut selection_rng)?;
            local
                .into_iter()
                .map(|position| admissible[position])
                .collect()
        } else if admissible.len() >= 2 {
            // Fewer admissible than the target size, but enough to reproduce:
            // keep exactly the admissible individuals (no hard-infeasible
            // backfill).
            admissible.clone()
        } else {
            // Zero admissible individuals, or a single one that cannot form a
            // next generation. Terminate with a typed failure; never backfill
            // hard-infeasible individuals and never report a normal completion.
            let message = if admissible.is_empty() {
                "no admissible individuals remain; cannot continue NSGA-III selection"
            } else {
                "fewer than two admissible individuals remain; cannot reproduce the next generation"
            };
            let error = MathError::new(ErrorKind::EmptyCollection, message);
            self.stop_reason = Some(StopReason::Failed);
            self.failure = Some(error.clone());
            return Err(error);
        };

        self.selected_indices = survivors.clone();

        let survivor_pool: Vec<Individual> = survivors
            .iter()
            .map(|&index| self.population[index].clone())
            .collect();

        if self.generation >= self.config.generations {
            self.stop_reason = Some(StopReason::IterationBudgetExhausted);
            self.population = survivor_pool;
            return Ok(());
        }

        let eta = adaptive_eta_value(
            self.generation,
            self.config.generations,
            self.config.eta_min,
            self.config.eta_max,
        );
        let mut offspring_rng = self.rng.derive_child(self.generation as u64);
        let offspring = var_or(
            &survivor_pool,
            &self.config.variables,
            n,
            self.config.crossover_prob,
            self.config.mutation_prob,
            eta,
            &mut offspring_rng,
        )?;

        let mut next_population = survivor_pool;
        next_population.extend(offspring);
        self.population = next_population;
        self.generation += 1;
        Ok(())
    }

    /// Request an early halt; the next [`Self::tell`]/[`Self::step`] becomes a
    /// no-op and [`Self::stop_reason`] reports [`StopReason::Aborted`].
    pub fn abort(&mut self) {
        if self.stop_reason.is_none() {
            self.stop_reason = Some(StopReason::Aborted);
        }
    }

    /// Indices chosen by the last environmental selection.
    #[must_use]
    pub fn selected_indices(&self) -> &[usize] {
        &self.selected_indices
    }

    /// The terminal reason, if the session has stopped.
    #[must_use]
    pub fn stop_reason(&self) -> Option<StopReason> {
        self.stop_reason
    }

    /// The typed failure recorded from an evaluation `Failure`, if any.
    #[must_use]
    pub fn failure(&self) -> Option<&MathError> {
        self.failure.as_ref()
    }

    /// Total successful evaluations recorded.
    #[must_use]
    pub fn evaluations(&self) -> u64 {
        self.evaluations
    }

    /// The current generation counter.
    #[must_use]
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// The number of objectives discovered from the first evaluation.
    #[must_use]
    pub fn objectives(&self) -> Option<usize> {
        self.objectives
    }

    fn progress(&self) -> SessionProgress {
        SessionProgress {
            generation: self.generation,
            evaluations: self.evaluations,
            stop_reason: self.stop_reason,
            selected_indices: self.selected_indices.clone(),
        }
    }
}
