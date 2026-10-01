//! Constraint descriptors, violation magnitudes and feasibility evaluation.

use crate::error::{ErrorKind, MathError};
use crate::evaluation::Feasibility;
use crate::value::ValidFloat;

/// Hard/soft semantics of a constraint.
///
/// `Hard` constraints are inadmissible when violated at all; `Soft` constraints
/// tolerate a non-negative violation magnitude up to `tolerance` (inclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConstraintKind {
    /// Must hold exactly; any non-zero violation makes a candidate inadmissible.
    Hard,
    /// May be exceeded by up to `tolerance` (inclusive) before it counts as a violation.
    Soft {
        /// The non-negative tolerance.
        tolerance: ValidFloat,
    },
}

impl ConstraintKind {
    /// Build a soft constraint, rejecting a negative tolerance.
    ///
    /// # Errors
    /// [`ErrorKind::InvertedBounds`] if `tolerance` is negative.
    pub fn soft(tolerance: ValidFloat) -> Result<Self, MathError> {
        if tolerance.get() >= 0.0 {
            Ok(Self::Soft { tolerance })
        } else {
            Err(MathError::new(
                ErrorKind::InvertedBounds,
                "soft tolerance must be non-negative",
            ))
        }
    }

    /// Re-validate this kind.
    ///
    /// # Errors
    /// [`ErrorKind::InvertedBounds`] if a soft tolerance is negative.
    pub fn validate(&self) -> Result<(), MathError> {
        match self {
            Self::Hard => Ok(()),
            Self::Soft { tolerance } => {
                if tolerance.get() >= 0.0 {
                    Ok(())
                } else {
                    Err(MathError::new(
                        ErrorKind::InvertedBounds,
                        "soft tolerance must be non-negative",
                    ))
                }
            }
        }
    }

    /// The soft tolerance, if this is a soft constraint.
    #[must_use]
    pub fn tolerance(&self) -> Option<ValidFloat> {
        match self {
            Self::Hard => None,
            Self::Soft { tolerance } => Some(*tolerance),
        }
    }
}

/// Position and kind of a single constraint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConstraintDescriptor {
    index: usize,
    kind: ConstraintKind,
}

impl ConstraintDescriptor {
    /// Build a constraint descriptor, validating its kind.
    ///
    /// # Errors
    /// Propagates [`ConstraintKind::validate`].
    pub fn new(index: usize, kind: ConstraintKind) -> Result<Self, MathError> {
        kind.validate()?;
        Ok(Self { index, kind })
    }

    /// The constraint index.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// The constraint kind.
    #[must_use]
    pub fn kind(&self) -> ConstraintKind {
        self.kind
    }
}

/// A non-negative constraint violation magnitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConstraintViolation(ValidFloat);

impl ConstraintViolation {
    /// Build a violation magnitude, rejecting a negative value.
    ///
    /// # Errors
    /// [`ErrorKind::InvertedBounds`] if `magnitude` is negative.
    pub fn new(magnitude: ValidFloat) -> Result<Self, MathError> {
        if magnitude.get() >= 0.0 {
            Ok(Self(magnitude))
        } else {
            Err(MathError::new(
                ErrorKind::InvertedBounds,
                "constraint violation magnitude must be non-negative",
            ))
        }
    }

    /// The violation magnitude.
    #[must_use]
    pub fn magnitude(&self) -> ValidFloat {
        self.0
    }

    /// The violation magnitude as a raw `f64`.
    #[must_use]
    pub fn get(&self) -> f64 {
        self.0.get()
    }
}

/// Evaluate feasibility from descriptors and per-constraint violation magnitudes.
///
/// Semantics (DoD 3):
/// - any `Hard` constraint with a non-zero violation produces
///   [`Feasibility::InfeasibleHard`] and takes precedence;
/// - otherwise any `Soft` constraint with `violation > tolerance` produces
///   [`Feasibility::SoftViolation`] (still admissible);
/// - otherwise [`Feasibility::Feasible`].
///
/// # Errors
/// [`ErrorKind::DimensionMismatch`] if the two slices have different lengths.
pub fn evaluate_feasibility(
    constraint_descriptors: &[ConstraintDescriptor],
    violations: &[ConstraintViolation],
) -> Result<Feasibility, MathError> {
    if constraint_descriptors.len() != violations.len() {
        return Err(MathError::new(
            ErrorKind::DimensionMismatch,
            "descriptor count does not match violation count",
        ));
    }

    let mut hard_violated = Vec::new();
    let mut soft_violated = Vec::new();

    for (index, (descriptor, violation)) in
        constraint_descriptors.iter().zip(violations).enumerate()
    {
        match descriptor.kind() {
            ConstraintKind::Hard => {
                if violation.get().to_bits() & 0x7fff_ffff_ffff_ffff != 0 {
                    hard_violated.push(index);
                }
            }
            ConstraintKind::Soft { tolerance } => {
                if violation.get() > tolerance.get() {
                    soft_violated.push(index);
                }
            }
        }
    }

    if !hard_violated.is_empty() {
        return Ok(Feasibility::InfeasibleHard {
            violated: hard_violated,
        });
    }
    if !soft_violated.is_empty() {
        return Ok(Feasibility::SoftViolation {
            violated: soft_violated,
        });
    }
    Ok(Feasibility::Feasible)
}
