//! Variable identity, kind descriptors and candidate validation.

use crate::error::{ErrorKind, MathError};
use crate::value::ValidFloat;

/// Stable identity of a decision variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableId(pub u64);

/// A finite, non-empty, strictly ascending and unique set of admissible values.
///
/// The inner vector is private so that only [`DiscreteSet::new`] can construct a
/// value, enforcing non-emptiness and strict ordering at all times.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiscreteSet(Vec<ValidFloat>);

impl DiscreteSet {
    /// Build a validated discrete set.
    ///
    /// # Errors
    /// - [`ErrorKind::EmptyCollection`] if `values` is empty.
    /// - [`ErrorKind::InvertedBounds`] if `values` is not strictly ascending
    ///   (which also rejects duplicates).
    pub fn new(values: Vec<ValidFloat>) -> Result<Self, MathError> {
        let set = Self(values);
        set.validate()?;
        Ok(set)
    }

    /// Re-check the non-empty / strictly-ascending invariants.
    ///
    /// # Errors
    /// See [`DiscreteSet::new`].
    pub fn validate(&self) -> Result<(), MathError> {
        if self.0.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "discrete set must not be empty",
            ));
        }
        let mut previous: Option<ValidFloat> = None;
        for value in &self.0 {
            if let Some(prev) = previous {
                if prev >= *value {
                    return Err(MathError::new(
                        ErrorKind::InvertedBounds,
                        "discrete set must be strictly ascending",
                    ));
                }
            }
            previous = Some(*value);
        }
        Ok(())
    }

    /// Number of admissible values (always `>= 1`).
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the set is empty; always `false` for a valid [`DiscreteSet`].
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The admissible values in ascending order.
    #[must_use]
    pub fn values(&self) -> &[ValidFloat] {
        &self.0
    }

    /// The admissible value at `index`, if any.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&ValidFloat> {
        self.0.get(index)
    }

    /// Whether `value` is a member of the set (binary search).
    #[must_use]
    pub fn contains(&self, value: &ValidFloat) -> bool {
        self.0.binary_search(value).is_ok()
    }
}

/// The domain of a decision variable, with no domain-specific assumptions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum VariableKind {
    /// A continuous variable over the closed interval `[lower, upper]`.
    Continuous {
        /// Inclusive lower bound.
        lower: ValidFloat,
        /// Inclusive upper bound.
        upper: ValidFloat,
    },
    /// An integer variable over the closed interval `[lower, upper]`.
    Integer {
        /// Inclusive lower bound.
        lower: i64,
        /// Inclusive upper bound.
        upper: i64,
    },
    /// A variable restricted to a validated [`DiscreteSet`].
    Discrete {
        /// The admissible values.
        values: DiscreteSet,
    },
    /// A categorical variable addressed by an index in `0..cardinality`.
    Categorical {
        /// Number of categories (must be `>= 1`).
        cardinality: u64,
    },
}

impl VariableKind {
    /// Build a continuous kind, rejecting inverted bounds.
    ///
    /// # Errors
    /// [`ErrorKind::InvertedBounds`] if `lower > upper`.
    pub fn continuous(lower: ValidFloat, upper: ValidFloat) -> Result<Self, MathError> {
        if lower <= upper {
            Ok(Self::Continuous { lower, upper })
        } else {
            Err(MathError::new(
                ErrorKind::InvertedBounds,
                "continuous lower bound exceeds upper bound",
            ))
        }
    }

    /// Build an integer kind, rejecting inverted bounds.
    ///
    /// # Errors
    /// [`ErrorKind::InvertedBounds`] if `lower > upper`.
    pub fn integer(lower: i64, upper: i64) -> Result<Self, MathError> {
        if lower <= upper {
            Ok(Self::Integer { lower, upper })
        } else {
            Err(MathError::new(
                ErrorKind::InvertedBounds,
                "integer lower bound exceeds upper bound",
            ))
        }
    }

    /// Build a discrete kind from an already-validated [`DiscreteSet`].
    ///
    /// # Errors
    /// Never fails today (the [`DiscreteSet`] is validated), but returns a
    /// `Result` for forward compatibility.
    pub fn discrete(values: DiscreteSet) -> Result<Self, MathError> {
        Ok(Self::Discrete { values })
    }

    /// Build a categorical kind, rejecting an empty domain.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `cardinality == 0`.
    pub fn categorical(cardinality: u64) -> Result<Self, MathError> {
        if cardinality >= 1 {
            Ok(Self::Categorical { cardinality })
        } else {
            Err(MathError::new(
                ErrorKind::EmptyCollection,
                "categorical cardinality must be at least 1",
            ))
        }
    }

    /// Re-validate the invariants of this kind.
    ///
    /// # Errors
    /// [`ErrorKind::InvertedBounds`] or [`ErrorKind::EmptyCollection`] when an
    /// invariant is violated (possible only for values built via a raw enum
    /// literal rather than the validating constructors).
    pub fn validate(&self) -> Result<(), MathError> {
        match self {
            Self::Continuous { lower, upper } => {
                if lower <= upper {
                    Ok(())
                } else {
                    Err(MathError::new(
                        ErrorKind::InvertedBounds,
                        "continuous lower bound exceeds upper bound",
                    ))
                }
            }
            Self::Integer { lower, upper } => {
                if lower <= upper {
                    Ok(())
                } else {
                    Err(MathError::new(
                        ErrorKind::InvertedBounds,
                        "integer lower bound exceeds upper bound",
                    ))
                }
            }
            Self::Discrete { values } => values.validate(),
            Self::Categorical { cardinality } => {
                if *cardinality >= 1 {
                    Ok(())
                } else {
                    Err(MathError::new(
                        ErrorKind::EmptyCollection,
                        "categorical cardinality must be at least 1",
                    ))
                }
            }
        }
    }
}

/// A decision variable: a stable identity plus a validated domain.
///
/// Fields are private so invalid domains cannot be stored; construct with
/// [`Variable::new`] and inspect with [`Variable::id`] / [`Variable::kind`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Variable {
    id: VariableId,
    kind: VariableKind,
}

impl Variable {
    /// Build a variable, validating its kind.
    ///
    /// # Errors
    /// Propagates the validation error of `kind` (see [`VariableKind::validate`]).
    pub fn new(id: VariableId, kind: VariableKind) -> Result<Self, MathError> {
        kind.validate()?;
        Ok(Self { id, kind })
    }

    /// The variable identity.
    #[must_use]
    pub fn id(&self) -> VariableId {
        self.id
    }

    /// The variable domain descriptor.
    #[must_use]
    pub fn kind(&self) -> &VariableKind {
        &self.kind
    }

    /// Check that `value` lies in this variable's domain.
    ///
    /// # Errors
    /// - [`ErrorKind::DomainMismatch`] for continuous/integer values outside the
    ///   interval (and non-integral integer values).
    /// - [`ErrorKind::ValueNotInDiscreteSet`] when a discrete value is absent.
    /// - [`ErrorKind::IndexOutOfRange`] when a categorical value is non-integral
    ///   or outside `0..cardinality`.
    pub fn validate_value(&self, value: ValidFloat) -> Result<(), MathError> {
        match &self.kind {
            VariableKind::Continuous { lower, upper } => {
                if *lower <= value && value <= *upper {
                    Ok(())
                } else {
                    Err(MathError::new(
                        ErrorKind::DomainMismatch,
                        "value outside continuous bounds",
                    ))
                }
            }
            VariableKind::Integer { lower, upper } => {
                let raw = value.get();
                // Integer bounds are i64 and are NOT exactly representable as
                // f64 once |bound| >= 2^53, so an f64 comparison would round and
                // misclassify values (e.g. `Integer { 0, i64::MAX }` would wrongly
                // accept 2^63). Compare in exact integer space instead: reject
                // non-integral values and anything outside the i64 range, then
                // cast — exact for an integral f64 with |raw| < 2^63 — and compare
                // `lower..=upper` in i64.
                if !is_integral(raw) || !(I64_MIN..I64_MAX_PLUS_ONE).contains(&raw) {
                    return Err(MathError::new(
                        ErrorKind::DomainMismatch,
                        "value is not an integer within bounds",
                    ));
                }
                let integer = raw as i64;
                if integer < *lower || integer > *upper {
                    return Err(MathError::new(
                        ErrorKind::DomainMismatch,
                        "value is not an integer within bounds",
                    ));
                }
                Ok(())
            }
            VariableKind::Discrete { values } => {
                if values.contains(&value) {
                    Ok(())
                } else {
                    Err(MathError::new(
                        ErrorKind::ValueNotInDiscreteSet,
                        "value is not a member of the discrete set",
                    ))
                }
            }
            VariableKind::Categorical { cardinality } => {
                let raw = value.get();
                // Cardinality is u64 and may exceed 2^53, so an f64 comparison
                // (`raw >= cardinality as f64`) would round (e.g. cardinality
                // 2^53 + 1 would wrongly reject index 2^53). Compare in exact
                // integer space: reject non-integral, negative, or >= 2^64
                // values, then cast (exact) and test `index < cardinality` in u64.
                if !is_integral(raw) || !(0.0..U64_MAX_PLUS_ONE).contains(&raw) {
                    return Err(MathError::new(
                        ErrorKind::IndexOutOfRange,
                        "categorical index out of range",
                    ));
                }
                let index = raw as u64;
                if index >= *cardinality {
                    return Err(MathError::new(
                        ErrorKind::IndexOutOfRange,
                        "categorical index out of range",
                    ));
                }
                Ok(())
            }
        }
    }
}

/// Whether a finite `f64` has a zero fractional part (`-0.0` counts as integral).
fn is_integral(value: f64) -> bool {
    value.fract().to_bits() & 0x7fff_ffff_ffff_ffff == 0
}

/// `2^63` as `f64`: exact, the first value that overflows `i64`.
const I64_MAX_PLUS_ONE: f64 = 9_223_372_036_854_775_808.0;

/// `-2^63` as `f64`: exact, equal to `i64::MIN`.
const I64_MIN: f64 = -9_223_372_036_854_775_808.0;

/// `2^64` as `f64`: exact, the first value that overflows `u64`.
const U64_MAX_PLUS_ONE: f64 = 18_446_744_073_709_551_616.0;
