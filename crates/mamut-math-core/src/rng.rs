//! Deterministic, dependency-free seeded random-number generation (frozen v1).
//!
//! This module provides the MAMUT-owned seeding and stream contract used by
//! every stochastic optimizer. It is intentionally built on a pure-Rust,
//! std-only ChaCha20 keystream generator ([RFC 7539] block function, 20 rounds)
//! so that a run is exactly replayable for the same release without depending on
//! the [`rand`](https://crates.io/crates/rand) crate (forbidden in core by ADRs
//! `MATH-ARC-002`/`MATH-DEP-001`).
//!
//! ## Frozen contract (version 1)
//!
//! Everything in this module is *frozen* under
//! [`RNG_ALGORITHM_ID`] = `"mamut.chacha20-std"` and
//! [`RNG_ALGORITHM_VERSION`] = `1`. Changing any constant, expansion rule,
//! derivation rule or byte layout is a **breaking change** and requires a new
//! algorithm version.
//!
//! ### Root construction
//! A root stream is built from a 32-byte [`Seed`]:
//! - `key` = the 32 seed bytes,
//! - `nonce` = ASCII `b"MAMUT-RNG-v1"` (exactly 12 bytes),
//! - `counter` = 0,
//! - `derivation_path` = empty.
//!
//! The seed fingerprint (below) identifies the root seed; it is preserved
//! unchanged through the whole tree, so metadata always reports the **root**
//! seed identity plus the full path.
//!
//! ### Stream consumption
//! A stream produces the keystream `chacha20_block(key, counter, nonce)` for
//! `counter = 0, 1, 2, …`. On buffer exhaustion the next block is generated at
//! the current counter and the counter is then incremented. A single stream
//! therefore yields at most `2^32 − 1` blocks (counters `0..=u32::MAX − 1`,
//! about 256 GiB); attempting to generate a
//! block at `counter == u32::MAX` is a documented exhaustion error. Because the
//! `fill_bytes`/`next_*` signatures cannot return an error, library code panics
//! on that practically unreachable case; [`MamutRng::try_next_block`] is the
//! checked alternative for embedders that want to avoid the panic.
//!
//! ### Child derivation (frozen v1, keyed and state-independent)
//! Children depend **only** on `(parent.key, stream_index)`, never on how much
//! of the parent stream was already consumed. For a stream index `s`:
//! - `derivation_nonce` = `s.to_le_bytes()[0..8]` ++ ASCII `b"CHLD"` (12 bytes),
//! - `counter` = `(s >> 32) as u32`,
//! - `d` = `chacha20_block(parent.key, counter, derivation_nonce)`,
//! - `child.key` = `d[0..32]`, `child.nonce` = `d[32..44]`,
//!   `child.counter` = 0,
//!   `child.derivation_path` = `parent.derivation_path` ++ `[s]`,
//!   `child.seed_fingerprint` = `parent.seed_fingerprint`.
//!
//! **Conflict rules.** Deterministic replay requires the same release, the same
//! root seed and the same derivation path. Sibling streams created with the same
//! index under the same parent key are identical *by design* (they are the same
//! logical stream); different indices are independent. Streams at different
//! paths never claim cross-version bit identity — that is an explicit
//! non-goal: a future algorithm version may change the bytes.
//!
//! Distinctness between distinct sibling/path streams is established
//! **empirically** on the frozen corpora (16 384-way sibling sets, 100-way path
//! sets); it is *not* a formal injectivity guarantee and downstream code must
//! not cite these tests as such.
//!
//! ### Seed expansion (frozen v1)
//! [`Seed::from_u64`] expands a `u64` into a 32-byte seed by setting
//! `key = value.to_le_bytes()` ++ 24 zero bytes, `counter = 0` and
//! `nonce = b"MAMUT-SEED1\0"`, then keeping the first 32 bytes of
//! `chacha20_block(key, 0, nonce)` as the child key. The literal
//! `b"MAMUT-SEED1"` is 11 bytes; it is right-padded with a single `0x00` byte
//! to satisfy the fixed 12-byte nonce width.
//!
//! ### Seed fingerprint (frozen v1)
//! `fingerprint(seed) = chacha20_block(seed, 0, b"MAM-FINGERPR")[0..8]`.
//!
//! [RFC 7539]: https://www.rfc-editor.org/rfc/rfc7539

use core::fmt;

use crate::codec::checked_u32;
use crate::error::{ErrorKind, MathError};

/// Stable identifier of the MAMUT RNG algorithm family/implementation.
///
/// This value is part of the evidence metadata contract; it must be recorded
/// verbatim so a run can be attributed to the exact algorithm.
pub const RNG_ALGORITHM_ID: &str = "mamut.chacha20-std";

/// Frozen algorithm version for [`RNG_ALGORITHM_ID`].
///
/// Any change to the keystream, seed expansion, derivation rules or metadata
/// encoding requires bumping this value.
pub const RNG_ALGORITHM_VERSION: u32 = 1;

/// ChaCha20 constant words, "expand 32-byte k" serialized little-endian.
const CHACHA_CONSTANTS: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

/// Length in bytes of a single ChaCha20 keystream block.
const BLOCK_LEN: usize = 64;

/// Root nonce, ASCII `b"MAMUT-RNG-v1"` (exactly 12 bytes).
const ROOT_NONCE: [u8; 12] = *b"MAMUT-RNG-v1";

/// Nonce used by [`Seed::from_u64`]. The frozen literal `b"MAMUT-SEED1"` is 11
/// bytes and is padded with one trailing `0x00` to the fixed 12-byte width.
const SEED_EXPANSION_NONCE: [u8; 12] = *b"MAMUT-SEED1\0";

/// Nonce used to compute the seed fingerprint, ASCII `b"MAM-FINGERPR"`.
const FINGERPRINT_NONCE: [u8; 12] = *b"MAM-FINGERPR";

/// The ChaCha20 block function as specified by [RFC 7539] §2.3.
///
/// The 16-word state is built from the constants `"expand 32-byte k"`, the key
/// (little-endian words 4..12), the 32-bit block counter (word 12) and the
/// 96-bit nonce (words 13..16). Twenty rounds (10 column rounds interleaved with
/// 10 diagonal rounds) are applied, the initial state is added back modulo
/// `2^32`, and the result is serialized little-endian.
///
/// This is a low-level primitive exposing the raw keystream; it is used both
/// internally and by the golden-vector tests that check it against the RFC.
///
/// It is exposed deliberately as an RFC 7539-anchored primitive so external
/// evidence/regression tests (and future oracles) can verify the block function
/// against the published vector without reimplementing it. It is stable under
/// algorithm id `mamut.chacha20-std` version 1.
///
/// [RFC 7539]: https://www.rfc-editor.org/rfc/rfc7539
#[must_use]
pub fn chacha20_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let mut state = [0u32; 16];
    state[0..4].copy_from_slice(&CHACHA_CONSTANTS);
    for (i, chunk) in key.chunks_exact(4).enumerate() {
        state[4 + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    state[12] = counter;
    for (i, chunk) in nonce.chunks_exact(4).enumerate() {
        state[13 + i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }

    let initial = state;
    for _ in 0..10 {
        // Column rounds.
        quarter_round(&mut state, 0, 4, 8, 12);
        quarter_round(&mut state, 1, 5, 9, 13);
        quarter_round(&mut state, 2, 6, 10, 14);
        quarter_round(&mut state, 3, 7, 11, 15);
        // Diagonal rounds.
        quarter_round(&mut state, 0, 5, 10, 15);
        quarter_round(&mut state, 1, 6, 11, 12);
        quarter_round(&mut state, 2, 7, 8, 13);
        quarter_round(&mut state, 3, 4, 9, 14);
    }

    let mut out = [0u8; 64];
    for (i, word) in state.iter().enumerate() {
        let sum = word.wrapping_add(initial[i]);
        out[i * 4..i * 4 + 4].copy_from_slice(&sum.to_le_bytes());
    }
    out
}

/// The ChaCha20 quarter round (RFC 7539 §2.1) applied in place to four words.
fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] = (state[d] ^ state[a]).rotate_left(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_left(12);
    state[a] = state[a].wrapping_add(state[b]);
    state[d] = (state[d] ^ state[a]).rotate_left(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_left(7);
}

/// Compute the frozen v1 seed fingerprint of a 32-byte seed.
fn seed_fingerprint_bytes(seed: &[u8; 32]) -> [u8; 8] {
    let block = chacha20_block(seed, 0, &FINGERPRINT_NONCE);
    let mut fingerprint = [0u8; 8];
    fingerprint.copy_from_slice(&block[..8]);
    fingerprint
}

/// A 32-byte seed identifying a deterministic root stream.
///
/// The bytes are never printed by [`fmt::Debug`]; the debug representation shows
/// the 8-byte fingerprint instead so logs cannot leak full seed material.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Seed([u8; 32]);

impl Seed {
    /// Wrap a raw 32-byte seed.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Seed {
        Seed(bytes)
    }

    /// Borrow the raw 32 seed bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Expand a `u64` into a seed using the frozen v1 rule.
    ///
    /// `key = value.to_le_bytes()` ++ 24 zero bytes, then the first 32 bytes of
    /// `chacha20_block(key, 0, b"MAMUT-SEED1\0")`. Documented in the module
    /// header; changing it breaks replay and requires a new version.
    #[must_use]
    pub fn from_u64(value: u64) -> Seed {
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(&value.to_le_bytes());
        let block = chacha20_block(&key, 0, &SEED_EXPANSION_NONCE);
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&block[..32]);
        Seed(seed)
    }
}

impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Seed")
            .field(&to_hex(&seed_fingerprint_bytes(&self.0)))
            .finish()
    }
}

/// MAMUT-owned, object-safe random-number generation port.
///
/// Implementors expose a deterministic byte stream plus evidence `metadata` and
/// deterministic child derivation. The trait intentionally lives in core so
/// domain crates never depend on a third-party RNG type.
pub trait RngPort {
    /// Return the next `u32` drawn little-endian from the stream.
    fn next_u32(&mut self) -> u32;

    /// Return the next `u64` drawn little-endian from the stream.
    fn next_u64(&mut self) -> u64;

    /// Return a `f64` uniformly distributed in `[0, 1)`.
    ///
    /// Canonical formula: `(next_u64() >> 11) as f64 * 2^-53`, which draws 53
    /// significand bits and can never return `1.0`.
    fn next_unit_f64(&mut self) -> f64;

    /// Fill `dst` with the next bytes of the stream.
    fn fill_bytes(&mut self, dst: &mut [u8]);

    /// Snapshot the evidence metadata for the current stream state.
    fn metadata(&self) -> RngMetadata;

    /// Derive an independent child stream at `stream_index`.
    ///
    /// The result is boxed so the trait stays object-safe. The derivation is
    /// keyed and state-independent; see the module header.
    fn derive_child_port(&self, stream_index: u64) -> Box<dyn RngPort>;
}

/// The ChaCha20 based [`RngPort`] implementation (frozen v1).
///
/// Construct a root with [`MamutRng::root`], then draw bytes through the
/// [`RngPort`] methods or derive children with [`MamutRng::derive_child`].
/// See the module header for the full frozen derivation, replay and conflict
/// rules.
pub struct MamutRng {
    key: [u8; 32],
    nonce: [u8; 12],
    counter: u32,
    block_buffer: [u8; BLOCK_LEN],
    buffer_pos: usize,
    seed_fingerprint: [u8; 8],
    derivation_path: Vec<u64>,
}

impl MamutRng {
    /// Build a root stream from a seed (frozen v1 root construction).
    ///
    /// `key = seed bytes`, `nonce = b"MAMUT-RNG-v1"`, `counter = 0`, empty path.
    #[must_use]
    pub fn root(seed: &Seed) -> MamutRng {
        MamutRng {
            key: *seed.as_bytes(),
            nonce: ROOT_NONCE,
            counter: 0,
            block_buffer: [0u8; BLOCK_LEN],
            buffer_pos: BLOCK_LEN,
            seed_fingerprint: seed_fingerprint_bytes(seed.as_bytes()),
            derivation_path: Vec::new(),
        }
    }

    /// Derive a child stream at `stream_index` (frozen v1 keyed derivation).
    ///
    /// The child depends only on `(self.key, stream_index)`, never on the number
    /// of bytes already consumed from `self`. The root seed fingerprint and the
    /// full derivation path are propagated; see the module header for the exact
    /// byte layout.
    #[must_use]
    pub fn derive_child(&self, stream_index: u64) -> MamutRng {
        let mut derivation_nonce = [0u8; 12];
        derivation_nonce[..8].copy_from_slice(&stream_index.to_le_bytes());
        derivation_nonce[8..12].copy_from_slice(b"CHLD");
        let counter = (stream_index >> 32) as u32;
        let digest = chacha20_block(&self.key, counter, &derivation_nonce);

        let mut key = [0u8; 32];
        key.copy_from_slice(&digest[..32]);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&digest[32..44]);

        let mut derivation_path = self.derivation_path.clone();
        derivation_path.push(stream_index);

        MamutRng {
            key,
            nonce,
            counter: 0,
            block_buffer: [0u8; BLOCK_LEN],
            buffer_pos: BLOCK_LEN,
            seed_fingerprint: self.seed_fingerprint,
            derivation_path,
        }
    }

    /// Generate and buffer the next block, or report stream exhaustion.
    ///
    /// This is the checked alternative to the internal panic path: it returns
    /// [`ErrorKind::IndexOutOfRange`] when the 32-bit block counter would have
    /// to generate at `counter == u32::MAX`, i.e. after 256 GiB on one stream.
    ///
    /// # Errors
    /// Returns [`ErrorKind::IndexOutOfRange`] when the stream is exhausted.
    pub fn try_next_block(&mut self) -> Result<(), MathError> {
        if self.counter == u32::MAX {
            return Err(MathError::new(
                ErrorKind::IndexOutOfRange,
                "MAMUT RNG stream exhausted: block counter reached u32::MAX",
            ));
        }
        self.block_buffer = chacha20_block(&self.key, self.counter, &self.nonce);
        self.counter += 1;
        self.buffer_pos = 0;
        Ok(())
    }

    /// Generate the next block, panicking if the stream is exhausted.
    ///
    /// # Panics
    /// Panics only on the documented, practically unreachable exhaustion of a
    /// single stream after 256 GiB. Use [`MamutRng::try_next_block`] to handle
    /// that case without panicking.
    fn ensure_buffer(&mut self) {
        if let Err(error) = self.try_next_block() {
            panic!("{error}");
        }
    }
}

impl RngPort for MamutRng {
    fn next_u32(&mut self) -> u32 {
        let mut bytes = [0u8; 4];
        self.fill_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }

    fn next_u64(&mut self) -> u64 {
        let mut bytes = [0u8; 8];
        self.fill_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    fn next_unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / ((1u64 << 53) as f64))
    }

    /// # Panics
    /// Panics only on the documented stream exhaustion after 256 GiB; see
    /// [`MamutRng::try_next_block`].
    fn fill_bytes(&mut self, dst: &mut [u8]) {
        let mut written = 0usize;
        while written < dst.len() {
            if self.buffer_pos == BLOCK_LEN {
                self.ensure_buffer();
            }
            let take = (BLOCK_LEN - self.buffer_pos).min(dst.len() - written);
            dst[written..written + take]
                .copy_from_slice(&self.block_buffer[self.buffer_pos..self.buffer_pos + take]);
            self.buffer_pos += take;
            written += take;
        }
    }

    fn metadata(&self) -> RngMetadata {
        RngMetadata::new(
            RNG_ALGORITHM_ID,
            RNG_ALGORITHM_VERSION,
            self.seed_fingerprint,
            self.derivation_path.clone(),
            self.counter as u64,
        )
        .expect("frozen algorithm id is valid: mamut.chacha20-std contains no NUL")
    }

    fn derive_child_port(&self, stream_index: u64) -> Box<dyn RngPort> {
        Box::new(self.derive_child(stream_index))
    }
}

impl fmt::Debug for MamutRng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never print the key or nonce; show only non-secret state.
        f.debug_struct("MamutRng")
            .field("algorithm_id", &RNG_ALGORITHM_ID)
            .field("algorithm_version", &RNG_ALGORITHM_VERSION)
            .field("seed_fingerprint", &to_hex(&self.seed_fingerprint))
            .field("derivation_path", &self.derivation_path)
            .field("counter", &self.counter)
            .finish_non_exhaustive()
    }
}

/// Evidence metadata describing one RNG stream (owned by the RNG side).
///
/// The module `core::evidence` (a separate ticket) embeds these fields; this
/// type only *defines* the contract and a deterministic
/// [`RngMetadata::canonical_bytes`] encoding. No serialization dependency is
/// used here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RngMetadata {
    algorithm_id: &'static str,
    algorithm_version: u32,
    seed_fingerprint: [u8; 8],
    derivation_path: Vec<u64>,
    blocks_consumed: u64,
}

impl RngMetadata {
    /// Construct metadata from its fields (frozen v1 encoding-domain validated).
    ///
    /// Intended for evidence decoders reconstructing a manifest; live streams
    /// obtain metadata from [`RngPort::metadata`].
    ///
    /// [`RngMetadata::canonical_bytes`] delimits `algorithm_id` with a trailing
    /// `0x00`, so an id containing a NUL byte would have no unambiguous v1
    /// encoding. This is the frozen v1 encoding-domain constraint: the id must
    /// be non-empty and must not contain a `0x00` byte.
    ///
    /// # Errors
    /// Returns [`ErrorKind::Reserved`] when `algorithm_id` is empty or contains
    /// a NUL byte.
    pub fn new(
        algorithm_id: &'static str,
        algorithm_version: u32,
        seed_fingerprint: [u8; 8],
        derivation_path: Vec<u64>,
        blocks_consumed: u64,
    ) -> Result<RngMetadata, MathError> {
        if algorithm_id.is_empty() || algorithm_id.as_bytes().contains(&0) {
            return Err(MathError::new(
                ErrorKind::Reserved,
                "algorithm id must be non-empty and contain no NUL byte",
            ));
        }
        Ok(RngMetadata {
            algorithm_id,
            algorithm_version,
            seed_fingerprint,
            derivation_path,
            blocks_consumed,
        })
    }

    /// The algorithm identifier, e.g. [`RNG_ALGORITHM_ID`].
    #[must_use]
    pub fn algorithm_id(&self) -> &'static str {
        self.algorithm_id
    }

    /// The algorithm version, e.g. [`RNG_ALGORITHM_VERSION`].
    #[must_use]
    pub fn algorithm_version(&self) -> u32 {
        self.algorithm_version
    }

    /// The 8-byte fingerprint of the **root** seed.
    #[must_use]
    pub fn seed_fingerprint(&self) -> &[u8; 8] {
        &self.seed_fingerprint
    }

    /// The full derivation path from the root, outermost first.
    #[must_use]
    pub fn derivation_path(&self) -> &[u64] {
        &self.derivation_path
    }

    /// The number of keystream blocks generated by this stream so far.
    #[must_use]
    pub fn blocks_consumed(&self) -> u64 {
        self.blocks_consumed
    }

    /// Deterministic, versioned byte encoding (frozen v1).
    ///
    /// Layout:
    /// `b"MAMUT-RNG-META-1"` ++ `0x00`
    /// ++ `algorithm_id` bytes ++ `0x00`
    /// ++ `algorithm_version` (`u32` big-endian)
    /// ++ `seed_fingerprint` (8 bytes)
    /// ++ `path_len` (`u32` big-endian)
    /// ++ `path_len` × `stream_index` (`u64` big-endian)
    /// ++ `blocks_consumed` (`u64` big-endian).
    ///
    /// The leading magic (16 bytes plus the `0x00` terminator, 17 bytes total)
    /// makes the encoding self-identifying for the evidence/manifest ticket.
    ///
    /// The wire format stores `path_len` as a `u32`, so a derivation path longer
    /// than `u32::MAX` elements has no v1 representation. Such a path is
    /// **rejected explicitly** rather than silently truncated, using the shared
    /// crate chokepoint `checked_u32` — the same reject-don't-truncate rule
    /// applied throughout [`crate::codec`]. For every representable value the
    /// bytes are identical to the frozen v1 layout.
    ///
    /// # Errors
    /// Returns [`ErrorKind::IndexOutOfRange`] when the derivation path has more
    /// than `u32::MAX` elements (unreachable in practice, but never silent).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, MathError> {
        let mut out = Vec::new();
        out.extend_from_slice(b"MAMUT-RNG-META-1");
        out.push(0);
        out.extend_from_slice(self.algorithm_id.as_bytes());
        out.push(0);
        out.extend_from_slice(&self.algorithm_version.to_be_bytes());
        out.extend_from_slice(&self.seed_fingerprint);
        let path_len = checked_u32(self.derivation_path.len(), "derivation path length")?;
        out.extend_from_slice(&path_len.to_be_bytes());
        for index in &self.derivation_path {
            out.extend_from_slice(&index.to_be_bytes());
        }
        out.extend_from_slice(&self.blocks_consumed.to_be_bytes());
        Ok(out)
    }

    /// Lowercase hex of [`RngMetadata::canonical_bytes`], for human evidence.
    ///
    /// # Errors
    /// Propagates the [`RngMetadata::canonical_bytes`] failure when the
    /// derivation path exceeds the `u32` wire-format range.
    pub fn canonical_hex(&self) -> Result<String, MathError> {
        Ok(to_hex(&self.canonical_bytes()?))
    }
}

/// Encode bytes as a lowercase hex string (no separators).
fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte offset of the `path_len` field in the frozen v1 metadata layout:
    /// magic `b"MAMUT-RNG-META-1"` (16) + `0x00` (1)
    /// + `"mamut.chacha20-std"` (18) + `0x00` (1)
    /// + version (4) + fingerprint (8) = 48.
    const PATH_LEN_OFFSET: usize = 48;

    /// The metadata path length is written only through the crate chokepoint
    /// [`checked_u32`], so its exact accept/reject boundary is exercised here
    /// directly (no multi-gigabyte allocation is possible or required).
    #[test]
    fn derivation_path_length_guard_boundaries() {
        // The largest representable length is accepted verbatim.
        assert_eq!(
            checked_u32(u32::MAX as usize, "derivation path length").unwrap(),
            u32::MAX
        );

        // One past u32::MAX is rejected explicitly instead of truncating.
        let error = checked_u32(u32::MAX as usize + 1, "derivation path length").unwrap_err();
        assert_eq!(error.kind(), ErrorKind::IndexOutOfRange);
    }

    /// `canonical_bytes` routes its `derivation_path.len()` through the guard
    /// above and still writes the frozen `u32` big-endian prefix, so the wire
    /// format is bit-identical for every representable path.
    #[test]
    fn canonical_bytes_path_length_prefix_is_u32_be() {
        let metadata = RngMetadata::new(
            RNG_ALGORITHM_ID,
            RNG_ALGORITHM_VERSION,
            [0u8; 8],
            vec![7, 9, 4_294_967_295],
            0,
        )
        .expect("valid metadata");
        let bytes = metadata
            .canonical_bytes()
            .expect("representable path length");
        assert_eq!(
            &bytes[PATH_LEN_OFFSET..PATH_LEN_OFFSET + 4],
            &3u32.to_be_bytes()
        );
        // A zero-length path still encodes the prefix (no off-by-one).
        let empty = RngMetadata::new(
            RNG_ALGORITHM_ID,
            RNG_ALGORITHM_VERSION,
            [0u8; 8],
            Vec::new(),
            0,
        )
        .expect("valid metadata");
        let empty_bytes = empty.canonical_bytes().expect("empty path is valid");
        assert_eq!(
            &empty_bytes[PATH_LEN_OFFSET..PATH_LEN_OFFSET + 4],
            &0u32.to_be_bytes()
        );
    }
}
