//! Candidates, feasibility, diagnostics and typed evaluation outcomes.

use crate::error::MathError;
use crate::value::ValidFloat;
use crate::variable::Variable;

/// A non-empty vector of decision-variable values (one per variable).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Candidate(Vec<ValidFloat>);

impl Candidate {
    /// Build a validated candidate.
    ///
    /// # Errors
    /// [`crate::ErrorKind::EmptyCollection`] if `values` is empty.
    pub fn new(values: Vec<ValidFloat>) -> Result<Self, MathError> {
        if values.is_empty() {
            return Err(MathError::new(
                crate::error::ErrorKind::EmptyCollection,
                "candidate must not be empty",
            ));
        }
        Ok(Self(values))
    }

    /// Number of decision variables in this candidate (always `>= 1`).
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the candidate is empty; always `false` for a valid value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The candidate values as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[ValidFloat] {
        &self.0
    }

    /// Validate this candidate against a problem's variable descriptors.
    ///
    /// # Errors
    /// - [`crate::ErrorKind::DimensionMismatch`] if the arity differs.
    /// - Otherwise the per-variable error from [`Variable::validate_value`].
    pub fn validate_against(&self, variables: &[Variable]) -> Result<(), MathError> {
        if self.0.len() != variables.len() {
            return Err(MathError::new(
                crate::error::ErrorKind::DimensionMismatch,
                "candidate arity does not match variable count",
            ));
        }
        for (value, variable) in self.0.iter().zip(variables) {
            variable.validate_value(*value)?;
        }
        Ok(())
    }
}

/// Outcome of constraint evaluation for a single candidate.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Feasibility {
    /// Every hard constraint is satisfied and every soft violation is within tolerance.
    Feasible,
    /// At least one hard constraint is violated; the candidate is not admissible.
    InfeasibleHard {
        /// Indices of the violated hard constraints.
        violated: Vec<usize>,
    },
    /// No hard constraint is violated, but at least one soft constraint exceeds its tolerance.
    SoftViolation {
        /// Indices of the soft constraints whose tolerance is exceeded.
        violated: Vec<usize>,
    },
}

impl Feasibility {
    /// Whether a candidate with this feasibility may be admitted into a population.
    ///
    /// [`Feasibility::Feasible`] and [`Feasibility::SoftViolation`] are
    /// admissible; [`Feasibility::InfeasibleHard`] is not.
    #[must_use]
    pub fn is_admissible(&self) -> bool {
        matches!(
            self,
            Feasibility::Feasible | Feasibility::SoftViolation { .. }
        )
    }
}

/// Typed machine-readable diagnostic code.
///
/// This is intentionally **not** an evidence manifest: there are no seeds,
/// fingerprints or run identifiers here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// The Pareto front collapsed (for example a single point dominates the rest).
    DegenerateFront,
    /// A non-fatal warning was raised during evaluation.
    EvaluationWarning,
    /// A diagnostic without a more specific code.
    Reserved,
}

/// A single typed diagnostic with a human-readable note.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostic {
    /// The machine-readable code.
    pub code: DiagnosticCode,
    /// A short human-readable explanation.
    pub note: String,
}

/// An ordered collection of [`Diagnostic`] values (may be empty).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    /// Build a diagnostics collection.
    #[must_use]
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self(diagnostics)
    }

    /// Number of diagnostics.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no diagnostics.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The diagnostics as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.0
    }

    /// Iterate over the diagnostics.
    pub fn iter(&self) -> core::slice::Iter<'_, Diagnostic> {
        self.0.iter()
    }
}

/// Typed result of evaluating one candidate.
///
/// Failure is explicit and typed ([`MathError`]); the contract never substitutes
/// a silent sentinel "huge fitness" value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluationOutcome {
    /// Evaluation produced objective values, constraint violations and diagnostics.
    Success {
        /// Objective values, one per objective.
        objectives: crate::objective::ObjectiveValues,
        /// Constraint violation magnitudes, one per constraint descriptor.
        constraints: Vec<crate::constraint::ConstraintViolation>,
        /// The feasibility classification of the candidate.
        feasibility: Feasibility,
        /// Typed diagnostics (not an evidence manifest).
        diagnostics: Diagnostics,
    },
    /// Evaluation failed with a typed error.
    Failure {
        /// The typed failure.
        error: MathError,
    },
}
