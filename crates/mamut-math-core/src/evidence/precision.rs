//! Numerical tolerance envelope recorded for a run.

use crate::codec::{write_u64, Canonical, Decoder};
use crate::error::{ErrorKind, MathError};
use crate::value::ValidFloat;

/// The absolute and relative tolerances a run was configured with.
///
/// Both tolerances must be non-negative; zero is allowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Precision {
    absolute_tolerance: ValidFloat,
    relative_tolerance: ValidFloat,
}

impl Precision {
    /// Build a precision envelope.
    ///
    /// # Errors
    /// [`ErrorKind::InvertedBounds`] if either tolerance is negative.
    pub fn new(
        absolute_tolerance: ValidFloat,
        relative_tolerance: ValidFloat,
    ) -> Result<Self, MathError> {
        if absolute_tolerance.get() < 0.0 || relative_tolerance.get() < 0.0 {
            return Err(MathError::new(
                ErrorKind::InvertedBounds,
                "precision tolerances must be non-negative",
            ));
        }
        Ok(Self {
            absolute_tolerance,
            relative_tolerance,
        })
    }

    /// The absolute tolerance.
    #[must_use]
    pub fn absolute_tolerance(&self) -> ValidFloat {
        self.absolute_tolerance
    }

    /// The relative tolerance.
    #[must_use]
    pub fn relative_tolerance(&self) -> ValidFloat {
        self.relative_tolerance
    }
}

impl Canonical for Precision {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_u64(self.absolute_tolerance.get().to_bits(), out);
        write_u64(self.relative_tolerance.get().to_bits(), out);
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let absolute_tolerance = dec.read_valid_float()?;
        let relative_tolerance = dec.read_valid_float()?;
        Precision::new(absolute_tolerance, relative_tolerance)
    }
}
