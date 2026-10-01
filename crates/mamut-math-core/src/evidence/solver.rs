//! Solver/backend versions and the deterministic-vs-stochastic run
//! classification.
//!
//! This module intentionally defines **no** RNG port: a stochastic run records
//! the seed and the RNG algorithm/version as plain data only. Random-number
//! generation itself is owned by its own ticket (`MATH-RNG-001`).

use crate::codec::{write_string, Canonical, Decoder};
use crate::error::{ErrorKind, MathError};

/// Identity and version of a solver or backend that participated in a run.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SolverVersion {
    id: String,
    version: String,
}

impl SolverVersion {
    /// Build a solver version record.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `id` or `version` is empty.
    pub fn new(id: String, version: String) -> Result<Self, MathError> {
        if id.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "solver id must not be empty",
            ));
        }
        if version.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "solver version must not be empty",
            ));
        }
        Ok(Self { id, version })
    }

    /// The solver/backend identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The solver/backend version string.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl Canonical for SolverVersion {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_string(&self.id, out)?;
        write_string(&self.version, out)?;
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let id = dec.read_string()?;
        let version = dec.read_string()?;
        SolverVersion::new(id, version)
    }
}

/// Identity and version of the random-number generator used by a stochastic run.
///
/// This is *metadata only*; it does not reference or define any RNG port.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RngSpec {
    algorithm: String,
    version: String,
}

impl RngSpec {
    /// Build an RNG specification.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `algorithm` or `version` is empty.
    pub fn new(algorithm: String, version: String) -> Result<Self, MathError> {
        if algorithm.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "rng algorithm must not be empty",
            ));
        }
        if version.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "rng version must not be empty",
            ));
        }
        Ok(Self { algorithm, version })
    }

    /// The RNG algorithm label.
    #[must_use]
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// The RNG algorithm version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl Canonical for RngSpec {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_string(&self.algorithm, out)?;
        write_string(&self.version, out)?;
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let algorithm = dec.read_string()?;
        let version = dec.read_string()?;
        RngSpec::new(algorithm, version)
    }
}

/// Whether a run is deterministic or stochastic, and the seed/rng metadata.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RunKind {
    /// A run with no random state; replaying it must reproduce the result.
    Deterministic,
    /// A run driven by a seeded RNG; the seed and RNG identity are recorded.
    Stochastic {
        /// The seed used by the run.
        seed: u128,
        /// The RNG algorithm and version.
        rng: RngSpec,
    },
}

impl RunKind {
    /// Build a deterministic run kind.
    #[must_use]
    pub const fn deterministic() -> Self {
        RunKind::Deterministic
    }

    /// Build a stochastic run kind.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `rng_algorithm` or `rng_version` is empty.
    pub fn stochastic(
        seed: u128,
        rng_algorithm: String,
        rng_version: String,
    ) -> Result<Self, MathError> {
        let rng = RngSpec::new(rng_algorithm, rng_version)?;
        Ok(RunKind::Stochastic { seed, rng })
    }

    /// Whether this run kind carries random state.
    #[must_use]
    pub fn is_stochastic(&self) -> bool {
        matches!(self, RunKind::Stochastic { .. })
    }

    /// The seed, if this is a stochastic run.
    #[must_use]
    pub fn seed(&self) -> Option<u128> {
        match self {
            RunKind::Deterministic => None,
            RunKind::Stochastic { seed, .. } => Some(*seed),
        }
    }

    /// The RNG specification, if this is a stochastic run.
    #[must_use]
    pub fn rng(&self) -> Option<&RngSpec> {
        match self {
            RunKind::Deterministic => None,
            RunKind::Stochastic { rng, .. } => Some(rng),
        }
    }
}

impl Canonical for RunKind {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        match self {
            RunKind::Deterministic => out.push(0),
            RunKind::Stochastic { seed, rng } => {
                out.push(1);
                out.extend_from_slice(&seed.to_le_bytes());
                write_string(rng.algorithm(), out)?;
                write_string(rng.version(), out)?;
            }
        }
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        match dec.read_u8()? {
            0 => Ok(RunKind::Deterministic),
            1 => {
                let seed = read_u128(dec)?;
                let rng_algorithm = dec.read_string()?;
                let rng_version = dec.read_string()?;
                RunKind::stochastic(seed, rng_algorithm, rng_version)
            }
            _ => Err(MathError::new(ErrorKind::Reserved, "unknown RunKind tag")),
        }
    }
}

/// Read a little-endian `u128` as two `u64` words.
pub(crate) fn read_u128(dec: &mut Decoder<'_>) -> Result<u128, MathError> {
    let low = dec.read_u64()? as u128;
    let high = dec.read_u64()? as u128;
    Ok((high << 64) | low)
}
