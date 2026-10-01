//! Golden vectors for the frozen v1 deterministic RNG.
//!
//! Two classes of evidence live here:
//! - **External oracles**: the ChaCha20 block/encryption vectors published in
//!   [RFC 7539] §2.3.2 and §2.4.2, transcribed verbatim from the RFC.
//! - **Regression guards**: seeded stream/derivation/metadata values pinned at
//!   `MATH-RNG-001` (release V0.1). These are *not* external oracles; they pin
//!   the exact implementation so any accidental drift fails loudly.
//!
//! [RFC 7539]: https://www.rfc-editor.org/rfc/rfc7539

use mamut_math_core::{
    chacha20_block, ErrorKind, MamutRng, RngMetadata, RngPort, Seed, RNG_ALGORITHM_ID,
    RNG_ALGORITHM_VERSION,
};

/// RFC 7539 §2.3.2 / §2.4.2 key: `00 01 02 … 1f`.
const RFC_KEY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

/// Seed A: bytes `00 01 02 … 1f`.
const SEED_A: [u8; 32] = RFC_KEY;

/// Expected keystream from RFC 7539 §2.3.2 (Block Count = 1).
///
/// RFC nonce `00:00:00:09:00:00:00:4a:00:00:00:00`, serialized block:
/// ```text
/// 10 f1 e7 e4 d1 3b 59 15 50 0f dd 1f a3 20 71 c4
/// c7 d1 f4 c7 33 c0 68 03 04 22 aa 9a c3 d4 6c 4e
/// d2 82 64 46 07 9f aa 09 14 c2 d7 05 d9 8b 02 a2
/// b5 12 9c d1 de 16 4e b9 cb d0 83 e8 a2 50 3c 4e
/// ```
#[rustfmt::skip]
const RFC_7539_2_3_2_KEYSTREAM: [u8; 64] = [
    0x10, 0xf1, 0xe7, 0xe4, 0xd1, 0x3b, 0x59, 0x15,
    0x50, 0x0f, 0xdd, 0x1f, 0xa3, 0x20, 0x71, 0xc4,
    0xc7, 0xd1, 0xf4, 0xc7, 0x33, 0xc0, 0x68, 0x03,
    0x04, 0x22, 0xaa, 0x9a, 0xc3, 0xd4, 0x6c, 0x4e,
    0xd2, 0x82, 0x64, 0x46, 0x07, 0x9f, 0xaa, 0x09,
    0x14, 0xc2, 0xd7, 0x05, 0xd9, 0x8b, 0x02, 0xa2,
    0xb5, 0x12, 0x9c, 0xd1, 0xde, 0x16, 0x4e, 0xb9,
    0xcb, 0xd0, 0x83, 0xe8, 0xa2, 0x50, 0x3c, 0x4e,
];

/// RFC 7539 §2.4.2 plaintext ("Sunscreen"), 114 bytes.
const RFC_7539_2_4_2_PLAINTEXT: &[u8] = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

/// RFC 7539 §2.4.2 ciphertext, 114 bytes (Ciphertext Sunscreen).
#[rustfmt::skip]
const RFC_7539_2_4_2_CIPHERTEXT: [u8; 114] = [
    0x6e, 0x2e, 0x35, 0x9a, 0x25, 0x68, 0xf9, 0x80, 0x41, 0xba, 0x07, 0x28, 0xdd, 0x0d, 0x69, 0x81,
    0xe9, 0x7e, 0x7a, 0xec, 0x1d, 0x43, 0x60, 0xc2, 0x0a, 0x27, 0xaf, 0xcc, 0xfd, 0x9f, 0xae, 0x0b,
    0xf9, 0x1b, 0x65, 0xc5, 0x52, 0x47, 0x33, 0xab, 0x8f, 0x59, 0x3d, 0xab, 0xcd, 0x62, 0xb3, 0x57,
    0x16, 0x39, 0xd6, 0x24, 0xe6, 0x51, 0x52, 0xab, 0x8f, 0x53, 0x0c, 0x35, 0x9f, 0x08, 0x61, 0xd8,
    0x07, 0xca, 0x0d, 0xbf, 0x50, 0x0d, 0x6a, 0x61, 0x56, 0xa3, 0x8e, 0x08, 0x8a, 0x22, 0xb6, 0x5e,
    0x52, 0xbc, 0x51, 0x4d, 0x16, 0xcc, 0xf8, 0x06, 0x81, 0x8c, 0xe9, 0x1a, 0xb7, 0x79, 0x37, 0x36,
    0x5a, 0xf9, 0x0b, 0xbf, 0x74, 0xa3, 0x5b, 0xe6, 0xb4, 0x0b, 0x8e, 0xed, 0xf2, 0x78, 0x5e, 0x42,
    0x87, 0x4d,
];

/// RFC 7539 §2.4.2 ChaCha20 encryption, exercising the multi-block path.
fn chacha20_encrypt(key: &[u8; 32], counter: u32, nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(plaintext.len());
    for (j, chunk) in plaintext.chunks(64).enumerate() {
        let keystream = chacha20_block(key, counter + j as u32, nonce);
        out.extend(chunk.iter().zip(keystream.iter()).map(|(p, k)| p ^ k));
    }
    out
}

fn take_u64s(rng: &mut dyn RngPort, n: usize) -> Vec<u64> {
    (0..n).map(|_| rng.next_u64()).collect()
}

#[test]
fn chacha20_block_matches_rfc7539_2_3_2() {
    // RFC 7539 §2.3.2: Nonce = 00:00:00:09:00:00:00:4a:00:00:00:00, Block Count = 1.
    let nonce = [
        0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00,
    ];
    let block = chacha20_block(&RFC_KEY, 1, &nonce);
    assert_eq!(block, RFC_7539_2_3_2_KEYSTREAM);
}

#[test]
fn chacha20_encrypt_matches_rfc7539_2_4_2() {
    // RFC 7539 §2.4.2: Nonce = 00:00:00:00:00:00:00:4a:00:00:00:00, Initial Counter = 1.
    let nonce = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(RFC_7539_2_4_2_PLAINTEXT.len(), 114);

    let ciphertext = chacha20_encrypt(&RFC_KEY, 1, &nonce, RFC_7539_2_4_2_PLAINTEXT);
    assert_eq!(ciphertext, RFC_7539_2_4_2_CIPHERTEXT);

    // Decryption is the same operation.
    let decrypted = chacha20_encrypt(&RFC_KEY, 1, &nonce, &ciphertext);
    assert_eq!(decrypted, RFC_7539_2_4_2_PLAINTEXT);
}

// --- Regression guards pinned at MATH-RNG-001, release V0.1. NOT external oracles. ---

#[test]
fn seed_a_first_eight_u64() {
    let mut rng = MamutRng::root(&Seed::from_bytes(SEED_A));
    assert_eq!(
        take_u64s(&mut rng, 8),
        vec![
            0x54c9_1453_f006_bf84,
            0x3d42_5f3a_aa2b_a293,
            0x4e46_ff32_a6c2_0699,
            0x2ec4_b714_fd56_a292,
            0xa250_feff_af9b_24d5,
            0x5000_318f_2aa4_7f3a,
            0x1212_0f54_965e_fb5d,
            0x71b1_3252_000c_8715,
        ]
    );
}

#[test]
fn seed_b_zero_first_eight_u64() {
    let mut rng = MamutRng::root(&Seed::from_bytes([0u8; 32]));
    assert_eq!(
        take_u64s(&mut rng, 8),
        vec![
            0xa17d_ee30_3a63_5eec,
            0x5dc5_1ae6_e22d_8219,
            0xeb2a_1a65_8874_51a0,
            0xded5_e660_9788_d7c9,
            0x8882_e617_7485_88c0,
            0x4630_219e_86c6_19ee,
            0x3ac0_64e4_79f3_3f93,
            0xec8f_09c8_ca72_8e50,
        ]
    );
}

#[test]
fn seed_c_from_u64_first_eight_u64() {
    let mut rng = MamutRng::root(&Seed::from_u64(42));
    assert_eq!(
        take_u64s(&mut rng, 8),
        vec![
            0x7a4c_f16f_01c4_1f01,
            0x6f7a_2fd3_6f00_dd5d,
            0xb8c3_96ab_750a_b1ba,
            0xb5f3_ea28_87a3_864e,
            0xa832_a5aa_9cdb_ce45,
            0xed7a_a398_d8cb_9999,
            0x5cec_28ed_da07_b5c2,
            0x0dbe_b63b_b719_e8e2,
        ]
    );
}

#[test]
fn seed_a_first_four_unit_f64_bits() {
    let mut rng = MamutRng::root(&Seed::from_bytes(SEED_A));
    let bits: Vec<u64> = (0..4).map(|_| rng.next_unit_f64().to_bits()).collect();
    assert_eq!(
        bits,
        vec![
            0x3fd5_3245_14fc_01ae,
            0x3fce_a12f_9d55_15d0,
            0x3fd3_91bf_cca9_b080,
            0x3fc7_625b_8a7e_ab50,
        ]
    );
    // All draws stay in [0, 1).
    let mut rng = MamutRng::root(&Seed::from_bytes(SEED_A));
    for _ in 0..1000 {
        let value = rng.next_unit_f64();
        assert!((0.0..1.0).contains(&value), "value out of [0,1): {value}");
    }
}

#[test]
fn replay_is_deterministic() {
    let mut a = MamutRng::root(&Seed::from_u64(7));
    let mut b = MamutRng::root(&Seed::from_u64(7));
    let mut buf_a = [0u8; 10_000];
    let mut buf_b = [0u8; 10_000];
    a.fill_bytes(&mut buf_a);
    b.fill_bytes(&mut buf_b);
    assert_eq!(buf_a, buf_b);
}

#[test]
fn child_derivation_is_state_independent() {
    let mut parent = MamutRng::root(&Seed::from_bytes(SEED_A));
    let before = parent.derive_child(3);

    // Consume an unaligned number of bytes, then derive again from the same parent.
    let mut scratch = [0u8; 777];
    parent.fill_bytes(&mut scratch);
    let after = parent.derive_child(3);

    let mut a = before;
    let mut b = after;
    assert_eq!(a.next_u64(), b.next_u64());
    assert_eq!(a.metadata(), b.metadata());
}

#[test]
fn child_path_golden() {
    let root = MamutRng::root(&Seed::from_bytes(SEED_A));

    let mut c7 = root.derive_child(7);
    assert_eq!(c7.next_u64(), 0x7c5f_ab41_eaa6_6579);

    // Path [7, 0].
    let mut c70 = root.derive_child(7).derive_child(0);
    assert_eq!(c70.next_u64(), 0x1169_66ba_d3bc_94a8);

    // Path [11, 3].
    let mut c113 = root.derive_child(11).derive_child(3);
    assert_eq!(c113.next_u64(), 0xf916_6e29_2700_f8ab);
}

#[test]
fn metadata_canonical_bytes_golden() {
    let rng = MamutRng::root(&Seed::from_u64(42));
    let metadata = rng.metadata();
    assert_eq!(metadata.algorithm_id(), RNG_ALGORITHM_ID);
    assert_eq!(metadata.algorithm_version(), RNG_ALGORITHM_VERSION);
    assert_eq!(
        metadata.canonical_hex().unwrap(),
        "4d414d55542d524e472d4d4554412d31006d616d75742e63686163686132302d73746400000000018dadfe0bbaacfc7e000000000000000000000000"
    );

    // A child records its path and preserves the root fingerprint.
    let child = MamutRng::root(&Seed::from_u64(42)).derive_child(5);
    let child_meta = child.metadata();
    assert_eq!(child_meta.derivation_path(), &[5]);
    assert_eq!(
        child_meta.seed_fingerprint(),
        metadata.seed_fingerprint(),
        "root fingerprint must propagate to children"
    );
}

#[test]
fn algorithm_id_encoding_domain_is_validated() {
    // An empty id has no unambiguous v1 canonical encoding.
    let empty = RngMetadata::new("", RNG_ALGORITHM_VERSION, [0u8; 8], Vec::new(), 0);
    assert_eq!(empty.unwrap_err().kind(), ErrorKind::Reserved);

    // An id with an embedded NUL is rejected (the encoding uses NUL as a delimiter).
    let with_nul = RngMetadata::new("a\0b", RNG_ALGORITHM_VERSION, [0u8; 8], Vec::new(), 0);
    assert_eq!(with_nul.unwrap_err().kind(), ErrorKind::Reserved);

    // The frozen id is accepted and the wire format is bit-identical to the
    // committed seed-C root golden.
    let fingerprint = *MamutRng::root(&Seed::from_u64(42))
        .metadata()
        .seed_fingerprint();
    let metadata = RngMetadata::new(
        RNG_ALGORITHM_ID,
        RNG_ALGORITHM_VERSION,
        fingerprint,
        Vec::new(),
        0,
    )
    .unwrap();
    assert_eq!(
        metadata.canonical_hex().unwrap(),
        "4d414d55542d524e472d4d4554412d31006d616d75742e63686163686132302d73746400000000018dadfe0bbaacfc7e000000000000000000000000"
    );
}

#[test]
fn seed_debug_does_not_leak_bytes() {
    let seed = Seed::from_bytes(SEED_A);
    let text = format!("{seed:?}");
    assert!(
        text.contains("dd774c5d3ee12b99"),
        "debug should show the fingerprint: {text}"
    );
    assert!(
        !text.contains("0001020304050607"),
        "debug must not leak raw seed bytes: {text}"
    );
}
