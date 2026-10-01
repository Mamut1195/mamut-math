//! Objective descriptors and objective-vector contract.

use crate::error::{ErrorKind, MathError};
use crate::value::ValidFloat;

/// Whether an objective is minimized or maximized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ObjectiveSense {
    /// The objective is minimized.
    Minimize,
    /// The objective is maximized.
    Maximize,
}

/// Position and sense of a single objective.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectiveDescriptor {
    index: usize,
    sense: ObjectiveSense,
}

impl ObjectiveDescriptor {
    /// Build an objective descriptor.
    #[must_use]
    pub fn new(index: usize, sense: ObjectiveSense) -> Self {
        Self { index, sense }
    }

    /// The objective index.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    /// The objective sense.
    #[must_use]
    pub fn sense(&self) -> ObjectiveSense {
        self.sense
    }
}

/// A non-empty vector of objective values, one per objective.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ObjectiveValues(Vec<ValidFloat>);

impl ObjectiveValues {
    /// Build a validated objective vector.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `values` is empty.
    pub fn new(values: Vec<ValidFloat>) -> Result<Self, MathError> {
        if values.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "objective vector must not be empty",
            ));
        }
        Ok(Self(values))
    }

    /// Number of objectives (always `>= 1`).
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the vector is empty; always `false` for a valid value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The objective value at `index`, if any.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&ValidFloat> {
        self.0.get(index)
    }

    /// Iterate over the objective values.
    pub fn iter(&self) -> core::slice::Iter<'_, ValidFloat> {
        self.0.iter()
    }

    /// The objective values as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[ValidFloat] {
        &self.0
    }
}
