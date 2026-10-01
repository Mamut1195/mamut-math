//! Content hashes for artifacts produced by a run.

use crate::codec::{write_string, Canonical, Decoder};
use crate::error::{ErrorKind, MathError};

use super::fingerprint::read_digest;

/// A named, hashed artifact or output.
///
/// Artifacts are referenced by name and digest only: the manifest never embeds
/// the artifact bytes themselves.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ArtifactHash {
    name: String,
    algorithm: String,
    digest: [u8; 32],
}

impl ArtifactHash {
    /// Build an artifact hash from a name, algorithm label and 32-byte digest.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `name` or `algorithm` is empty.
    pub fn new(name: String, algorithm: String, digest: [u8; 32]) -> Result<Self, MathError> {
        if name.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "artifact name must not be empty",
            ));
        }
        if algorithm.is_empty() {
            return Err(MathError::new(
                ErrorKind::EmptyCollection,
                "artifact algorithm must not be empty",
            ));
        }
        Ok(Self {
            name,
            algorithm,
            digest,
        })
    }

    /// Hash `bytes` with SHA-256 and label the result `"sha256-v1"`.
    ///
    /// # Errors
    /// [`ErrorKind::EmptyCollection`] if `name` is empty.
    pub fn sha256(name: String, bytes: &[u8]) -> Result<Self, MathError> {
        let digest = super::digest::sha256(bytes);
        ArtifactHash::new(name, "sha256-v1".to_string(), digest)
    }

    /// The artifact name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The hash algorithm label.
    #[must_use]
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// The 32-byte digest.
    #[must_use]
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

impl Canonical for ArtifactHash {
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), MathError> {
        write_string(&self.name, out)?;
        write_string(&self.algorithm, out)?;
        out.extend_from_slice(&self.digest);
        Ok(())
    }

    fn decode(dec: &mut Decoder<'_>) -> Result<Self, MathError> {
        let name = dec.read_string()?;
        let algorithm = dec.read_string()?;
        let digest = read_digest(dec)?;
        ArtifactHash::new(name, algorithm, digest)
    }
}
