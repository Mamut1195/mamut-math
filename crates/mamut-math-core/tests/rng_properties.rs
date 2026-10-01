//! Deterministic property and collision tests for the frozen v1 RNG.
//!
//! No third-party test framework is used (core forbids `[dev-dependencies]`);
//! every corpus is hand-written and deterministic so the suite is reproducible.

use std::collections::HashSet;

use mamut_math_core::{MamutRng, RngPort, Seed};

/// First 64 keystream bytes of a stream (one full block).
fn first_block_bytes(rng: &mut dyn RngPort) -> [u8; 64] {
    let mut block = [0u8; 64];
    rng.fill_bytes(&mut block);
    block
}

/// Follow a derivation path from a root and return the target stream.
fn stream_at_path(root: &MamutRng, path: &[u64]) -> MamutRng {
    let mut current = root.derive_child(path[0]);
    for &index in &path[1..] {
        current = current.derive_child(index);
    }
    current
}

#[test]
fn sibling_child_keys_are_pairwise_distinct() {
    // 64 roots × 256 sibling indices = 16 384 child streams.
    let mut blocks: HashSet<[u8; 64]> = HashSet::new();
    for parent in 0u64..64 {
        let root = MamutRng::root(&Seed::from_u64(parent));
        for index in 0u64..256 {
            let mut child = root.derive_child(index);
            blocks.insert(first_block_bytes(&mut child));
        }
    }
    assert_eq!(blocks.len(), 64 * 256, "sibling collision detected");
}

#[test]
fn derivation_paths_are_injective() {
    let paths: Vec<Vec<u64>> = vec![
        vec![0],
        vec![1],
        vec![2],
        vec![0, 0],
        vec![0, 1],
        vec![1, 0],
        vec![1, 1],
        vec![0, 0, 0],
        vec![0, 0, 1],
        vec![3],
    ];

    let mut blocks: HashSet<[u8; 64]> = HashSet::new();
    let mut pairs = 0usize;
    for root_index in 0u64..10 {
        let root = MamutRng::root(&Seed::from_u64(root_index));
        for path in &paths {
            let mut stream = stream_at_path(&root, path);
            blocks.insert(first_block_bytes(&mut stream));
            pairs += 1;
        }
    }
    assert!(pairs >= 100);
    assert_eq!(blocks.len(), pairs, "distinct (root, path) pairs collided");
}

#[test]
fn unit_f64_distribution_sanity() {
    // Loose structural sanity check, NOT statistical certification: the
    // half-open split of 200 000 draws must fall within a ±5% band.
    let mut rng = MamutRng::root(&Seed::from_u64(123_456_789));
    let mut below_half = 0u32;
    for _ in 0..200_000 {
        let value = rng.next_unit_f64();
        assert!((0.0..1.0).contains(&value), "value out of [0,1): {value}");
        if value < 0.5 {
            below_half += 1;
        }
    }
    assert!(
        (95_000..=105_000).contains(&below_half),
        "half-split outside ±5% band: {below_half}"
    );
}

#[test]
fn u32_parity_split_sanity() {
    let mut rng = MamutRng::root(&Seed::from_u64(987_654_321));
    let mut even = 0u32;
    for _ in 0..100_000 {
        if rng.next_u32() % 2 == 0 {
            even += 1;
        }
    }
    assert!(
        (47_500..=52_500).contains(&even),
        "parity split outside ±5% band: {even}"
    );
}

#[test]
fn fill_bytes_chunking_is_invariant() {
    // Chunked fills of 1..=200 bytes must equal one 20 100-byte draw.
    let mut chunked = MamutRng::root(&Seed::from_bytes([7u8; 32]));
    let mut expected = Vec::new();
    for size in 1usize..=200 {
        let mut chunk = vec![0u8; size];
        chunked.fill_bytes(&mut chunk);
        expected.extend_from_slice(&chunk);
    }

    let mut single = MamutRng::root(&Seed::from_bytes([7u8; 32]));
    let mut one_shot = vec![0u8; expected.len()];
    single.fill_bytes(&mut one_shot);

    assert_eq!(expected.len(), 20_100);
    assert_eq!(chunked.metadata(), single.metadata());
    assert_eq!(one_shot, expected);
}

#[test]
fn metadata_is_a_stable_snapshot() {
    let root_a = MamutRng::root(&Seed::from_u64(2024));
    let root_b = MamutRng::root(&Seed::from_u64(2024));

    // Identical states produce byte-identical metadata.
    let meta_a = root_a.metadata();
    let meta_b = root_b.metadata();
    assert_eq!(meta_a, meta_b);
    assert_eq!(
        meta_a.canonical_bytes().unwrap(),
        meta_b.canonical_bytes().unwrap()
    );

    // metadata() is a pure snapshot: taking it does not advance the stream.
    let snapshot = root_a.metadata();
    assert_eq!(root_a.metadata(), snapshot);
    assert_eq!(root_a.metadata().blocks_consumed(), 0);

    // Different derivation paths yield different metadata encodings.
    let child = root_a.derive_child(9);
    assert_ne!(
        child.metadata().canonical_bytes().unwrap(),
        snapshot.canonical_bytes().unwrap()
    );
    assert_eq!(child.metadata().derivation_path(), &[9]);

    // Consuming bytes updates blocks_consumed without changing the fingerprint.
    let mut progress = MamutRng::root(&Seed::from_u64(2024));
    let mut buf = [0u8; 64];
    progress.fill_bytes(&mut buf);
    let progressed = progress.metadata();
    assert_eq!(progressed.blocks_consumed(), 1);
    assert_eq!(progressed.seed_fingerprint(), snapshot.seed_fingerprint());
    assert_ne!(
        progressed.canonical_bytes().unwrap(),
        snapshot.canonical_bytes().unwrap()
    );
}
