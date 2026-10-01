//! Finite floating-point scalar contract (`f64` canonical for V0.x).

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use crate::error::{ErrorKind, MathError};

/// A floating-point value guaranteed to be finite (neither `NaN` nor infinite).
///
/// The only way to obtain a [`ValidFloat`] is through [`ValidFloat::new`],
/// which rejects `NaN` and both infinities with [`ErrorKind::InvalidNotFinite`].
///
/// Equality and hashing are bit-based on the underlying `f64` (so `0.0` and
/// `-0.0` are distinct); ordering is a total order over finite values. There is
/// intentionally **no `From<f64>`** conversion, so validation cannot be bypassed.
#[derive(Clone, Copy, Debug)]
pub struct ValidFloat(f64);

impl ValidFloat {
    /// Validate and wrap a finite `f64`.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidNotFinite`] when `value` is `NaN` or infinite.
    pub fn new(value: f64) -> Result<Self, MathError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(MathError::new(
                ErrorKind::InvalidNotFinite,
                "value is not finite",
            ))
        }
    }

    /// Return the underlying `f64`.
    #[must_use]
    pub fn get(&self) -> f64 {
        self.0
    }

    /// Return the underlying `f64` (explicit alias of [`ValidFloat::get`]).
    #[must_use]
    pub fn raw(&self) -> f64 {
        self.0
    }
}

impl PartialEq for ValidFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for ValidFloat {}

impl Hash for ValidFloat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl PartialOrd for ValidFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ValidFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}
