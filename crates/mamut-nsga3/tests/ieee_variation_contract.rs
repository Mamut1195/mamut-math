//! MATH-E2E-001 M1 — IEEE raw-variation contract (`mamut_nsga3::ieee`).
//!
//! Proves the raw-`f64` route shares **one** arithmetic body with the checked
//! operators: identical finite output bits, cache lifecycle, structural /
//! probability guards and RNG consumption, with the single intended difference
//! that unsanitised `NaN`/`±Inf` genes are stored instead of rejected.
//!
//! Synthetic marker corpus only: no engineering network and no duplicated
//! legacy algorithm oracle. Expectations are captured behaviour of the frozen
//! public typed operators, not a second implementation.

use core::num::NonZeroUsize;

use mamut_math_core::{
    Candidate, ErrorKind, Feasibility, MamutRng, ObjectiveValues, RngMetadata, RngPort, Seed,
    ValidFloat, Variable, VariableId, VariableKind, RNG_ALGORITHM_ID, RNG_ALGORITHM_VERSION,
};
use mamut_nsga3::{ieee, operators};

/// Scripted `RngPort`. `unit_pos`/`index_pos` double as draw counters; metadata
/// reports the real frozen MAMUT algorithm id (no faked id or seed key).
struct Port {
    units: Vec<f64>,
    unit_pos: usize,
    indices: Vec<usize>,
    index_pos: usize,
}

impl Port {
    fn new(units: Vec<f64>, indices: Vec<usize>) -> Self {
        Self {
            units,
            unit_pos: 0,
            indices,
            index_pos: 0,
        }
    }

    fn unit_draws(&self) -> usize {
        self.unit_pos
    }

    fn index_draws(&self) -> usize {
        self.index_pos
    }
}

impl RngPort for Port {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        0
    }

    fn next_unit_f64(&mut self) -> f64 {
        let value = self.units[self.unit_pos];
        self.unit_pos += 1;
        value
    }

    fn next_index(&mut self, upper_exclusive: NonZeroUsize) -> usize {
        let value = self.indices[self.index_pos];
        self.index_pos += 1;
        assert!(
            value < upper_exclusive.get(),
            "scripted index {value} out of range {upper_exclusive}"
        );
        value
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        for byte in dst.iter_mut() {
            *byte = 0;
        }
    }

    fn metadata(&self) -> RngMetadata {
        RngMetadata::new(
            RNG_ALGORITHM_ID,
            RNG_ALGORITHM_VERSION,
            [0u8; 8],
            Vec::new(),
            0,
        )
        .expect("frozen algorithm id is valid")
    }

    fn derive_child_port(&self, _stream_index: u64) -> Box<dyn RngPort> {
        Box::new(Port::new(self.units.clone(), self.indices.clone()))
    }
}

fn variable(id: u64, kind: VariableKind) -> Variable {
    Variable::new(VariableId(id), kind).expect("valid variable")
}

fn cont_var(id: u64, lower: f64, upper: f64) -> Variable {
    variable(
        id,
        VariableKind::continuous(
            ValidFloat::new(lower).expect("finite lower"),
            ValidFloat::new(upper).expect("finite upper"),
        )
        .expect("valid bounds"),
    )
}

fn typed(genes: &[f64]) -> operators::Individual {
    operators::Individual::new(typed_candidate(genes))
}

fn evaluated_typed(genes: &[f64]) -> operators::Individual {
    operators::Individual::evaluated(
        typed_candidate(genes),
        ObjectiveValues::new(vec![ValidFloat::new(1.0).expect("finite objective")])
            .expect("non-empty objectives"),
        Feasibility::Feasible,
    )
}

fn typed_candidate(genes: &[f64]) -> Candidate {
    Candidate::new(
        genes
            .iter()
            .map(|&gene| ValidFloat::new(gene).expect("finite gene"))
            .collect(),
    )
    .expect("non-empty candidate")
}

fn raw(genes: &[f64]) -> ieee::Individual {
    ieee::Individual::new(genes.to_vec())
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

/// Checked operators validate shape then `cxpb + mutpb` before any draw, and
/// the fixture genuinely reports the frozen MAMUT RNG id.
#[test]
fn checked_guards_precede_any_draw() {
    let vars = vec![cont_var(0, 0.0, 1.0), cont_var(1, 0.0, 1.0)];

    let (mut a, mut b) = (typed(&[0.5]), typed(&[0.5]));
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        operators::sbx_crossover(&mut a, &mut b, &vars, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
    let mut c = typed(&[0.5]);
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        operators::polynomial_mutation(&mut c, &vars, 20.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
    assert_eq!(rng.unit_draws(), 0);

    let one = vec![cont_var(0, 0.0, 1.0)];
    let pop = vec![typed(&[0.5])];
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        operators::var_or(&pop, &one, 2, 1.0, 1.0, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::InvertedBounds
    );
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        operators::var_or(&[], &one, 2, 0.0, 0.0, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        operators::var_or(&pop, &[], 2, 0.0, 0.0, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(rng.unit_draws() + rng.index_draws(), 0);
    assert_eq!(rng.metadata().algorithm_id(), RNG_ALGORITHM_ID);
}

/// Raw operators mirror the same guard order and zero-draw timing.
#[test]
fn raw_guards_precede_any_draw() {
    let vars = vec![cont_var(0, 0.0, 1.0), cont_var(1, 0.0, 1.0)];

    let (mut a, mut b) = (raw(&[0.5]), raw(&[0.5]));
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        ieee::sbx_crossover(&mut a, &mut b, &vars, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::DimensionMismatch
    );
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        ieee::sbx_crossover(&mut raw(&[]), &mut raw(&[]), &[], 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(rng.unit_draws(), 0);

    let one = vec![cont_var(0, 0.0, 1.0)];
    let pop = vec![raw(&[0.5])];
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        ieee::var_or(&pop, &one, 2, 1.0, 1.0, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::InvertedBounds
    );
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        ieee::var_or(&[], &one, 2, 0.0, 0.0, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    let mut rng = Port::new(vec![0.0; 8], vec![0; 8]);
    assert_eq!(
        ieee::var_or(&pop, &[], 2, 0.0, 0.0, 15.0, &mut rng)
            .unwrap_err()
            .kind(),
        ErrorKind::EmptyCollection
    );
    assert_eq!(rng.unit_draws() + rng.index_draws(), 0);
}

/// Identical parents (including `-0.0`/`0.0`) skip SBX with no draw, keep the
/// exact signed-zero bits, and both children are still invalidated.
#[test]
fn sbx_skips_identical_genes_and_preserves_signed_zero() {
    let vars = vec![cont_var(0, 0.0, 1.0)];
    let mut a = evaluated_typed(&[-0.0]);
    let mut b = evaluated_typed(&[-0.0]);
    let mut rng = Port::new(vec![0.9; 8], vec![0; 8]);
    operators::sbx_crossover(&mut a, &mut b, &vars, 15.0, &mut rng).unwrap();
    assert_eq!(rng.unit_draws(), 0, "identical genes consume no draw");
    assert_eq!(bits(&a.genes()), bits(&[-0.0]));
    assert!(a.fitness.is_none() && b.fitness.is_none());
}

/// Finite genes: raw and checked SBX produce identical bits and draw counts,
/// across continuous and categorical (static `0..cardinality-1`) bounds.
#[test]
fn sbx_finite_parity_across_static_bounds() {
    let vars = vec![
        cont_var(0, -5.0, 5.0),
        variable(
            1,
            VariableKind::categorical(5).expect("non-empty categorical"),
        ),
    ];
    let units = vec![0.17, 0.83];
    let (mut ta, mut tb) = (typed(&[1.0, 2.0]), typed(&[-3.0, 7.0]));
    let (mut ra, mut rb) = (raw(&[1.0, 2.0]), raw(&[-3.0, 7.0]));
    let (mut tp, mut rp) = (Port::new(units.clone(), vec![]), Port::new(units, vec![]));

    operators::sbx_crossover(&mut ta, &mut tb, &vars, 15.0, &mut tp).unwrap();
    ieee::sbx_crossover(&mut ra, &mut rb, &vars, 15.0, &mut rp).unwrap();

    assert_eq!(bits(&ta.genes()), bits(&ra.genes));
    assert_eq!(bits(&tb.genes()), bits(&rb.genes));
    assert_eq!(tp.unit_draws(), rp.unit_draws());
    assert!(ta.fitness.is_none() && ra.fitness.is_none());
}

/// Finite genes: raw and checked mutation produce identical bits and draws.
#[test]
fn mutation_finite_parity() {
    let vars = vec![cont_var(0, 0.0, 1.0), cont_var(1, -2.0, 2.0)];
    let units = vec![0.0, 0.1, 0.0, 0.9, 0.0, 0.6, 0.0, 0.4];
    let mut t = typed(&[0.5, -1.0]);
    let mut r = raw(&[0.5, -1.0]);
    let (mut tp, mut rp) = (Port::new(units.clone(), vec![]), Port::new(units, vec![]));

    operators::polynomial_mutation(&mut t, &vars, 20.0, &mut tp).unwrap();
    ieee::polynomial_mutation(&mut r, &vars, 20.0, &mut rp).unwrap();

    assert_eq!(bits(&t.genes()), bits(&r.genes));
    assert_eq!(tp.unit_draws(), rp.unit_draws());
}

/// Overflowed SBX spread (`0 * Inf`, parents `±f64::MAX`): the raw route stores
/// the `NaN` while the checked route rejects it after the same draw.
#[test]
fn sbx_nan_preserved_where_checked_rejects() {
    let max = f64::MAX;
    let vars = vec![cont_var(0, -max, max)];
    let (mut ta, mut tb) = (typed(&[max]), typed(&[-max]));
    let (mut ra, mut rb) = (raw(&[max]), raw(&[-max]));
    let (mut tp, mut rp) = (Port::new(vec![0.0], vec![]), Port::new(vec![0.0], vec![]));

    assert_eq!(
        operators::sbx_crossover(&mut ta, &mut tb, &vars, 15.0, &mut tp)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
    ieee::sbx_crossover(&mut ra, &mut rb, &vars, 15.0, &mut rp).unwrap();

    assert_eq!(bits(&ta.genes()), bits(&[max]), "checked child 1 unchanged");
    assert_eq!(
        bits(&tb.genes()),
        bits(&[-max]),
        "checked child 2 unchanged"
    );

    assert_eq!(ra.genes[0].to_bits(), 0xfff8_0000_0000_0000);
    assert_eq!(rb.genes[0].to_bits(), 0xfff8_0000_0000_0000);
    assert_eq!(tp.unit_draws(), rp.unit_draws());

    // Finite `±f64::MAX`-bounded parents whose intermediate overflows to `Inf`
    // are clamped to the finite bound in BOTH routes (no NaN).
    let (mut ca, mut cb) = (typed(&[0.5 * max]), typed(&[-0.5 * max]));
    let (mut da, mut db) = (raw(&[0.5 * max]), raw(&[-0.5 * max]));
    let (mut cp, mut dp) = (Port::new(vec![0.9], vec![]), Port::new(vec![0.9], vec![]));
    operators::sbx_crossover(&mut ca, &mut cb, &vars, 15.0, &mut cp).unwrap();
    ieee::sbx_crossover(&mut da, &mut db, &vars, 15.0, &mut dp).unwrap();
    assert!(ca.genes().iter().all(|g| g.is_finite()));
    assert_eq!(bits(&ca.genes()), bits(&da.genes));
    assert_eq!(bits(&cb.genes()), bits(&db.genes));
}

/// Overflowed mutation range (`0 * Inf`): raw stores the `NaN`, checked
/// rejects after the same draws.
#[test]
fn mutation_nan_preserved_where_checked_rejects() {
    let max = f64::MAX;
    let vars = vec![cont_var(0, -max, max)];
    let mut t = typed(&[0.0]);
    let mut r = raw(&[0.0]);
    let (mut tp, mut rp) = (
        Port::new(vec![0.0, 0.6], vec![]),
        Port::new(vec![0.0, 0.6], vec![]),
    );

    assert_eq!(
        operators::polynomial_mutation(&mut t, &vars, 20.0, &mut tp)
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidNotFinite
    );
    ieee::polynomial_mutation(&mut r, &vars, 20.0, &mut rp).unwrap();

    assert_eq!(bits(&t.genes()), bits(&[0.0]), "checked gene unchanged");

    assert_eq!(r.genes[0].to_bits(), 0xfff8_0000_0000_0000);
    assert_eq!(tp.unit_draws(), rp.unit_draws());
}

/// A raw input `NaN` propagates through mutation without sanitisation and keeps
/// its payload bit-for-bit.
#[test]
fn mutation_preserves_input_nan_payload() {
    let vars = vec![cont_var(0, -1.0, 1.0)];
    let mut r = raw(&[f64::NAN]);
    let mut rp = Port::new(vec![0.0, 0.6], vec![]);

    ieee::polynomial_mutation(&mut r, &vars, 20.0, &mut rp).unwrap();

    assert_eq!(r.genes[0].to_bits(), f64::NAN.to_bits());
}

/// Reproduction clones keep the raw caches; a gate-passing mutation invalidates
/// both caches at the original site.
#[test]
fn cache_lifecycle_reproduction_and_mutation() {
    let vars = vec![cont_var(0, 0.0, 1.0)];
    let mut population = vec![raw(&[0.2]), raw(&[0.8])];
    population[0].fitness = Some(vec![1.0, 2.0]);
    population[0].feasibility = Some(Feasibility::InfeasibleHard { violated: vec![0] });

    let mut rng = Port::new(vec![0.9, 0.9], vec![0, 1]);
    let kids = ieee::var_or(&population, &vars, 2, 0.0, 0.0, 15.0, &mut rng).unwrap();
    assert_eq!(kids[0].fitness, Some(vec![1.0, 2.0]));
    assert_eq!(
        kids[0].feasibility,
        Some(Feasibility::InfeasibleHard { violated: vec![0] })
    );

    let mut mutated = raw(&[0.5]);
    mutated.fitness = Some(vec![3.0]);
    mutated.feasibility = Some(Feasibility::Feasible);
    let mut rng = Port::new(vec![0.0, 0.1], vec![]);
    ieee::polynomial_mutation(&mut mutated, &vars, 20.0, &mut rng).unwrap();
    assert!(mutated.fitness.is_none() && mutated.feasibility.is_none());
}

/// SBX writes and invalidates BOTH children inside the call, so an odd `lambda`
/// that never keeps the second child still paid for its conversion.
#[test]
fn sbx_writes_both_children_inside_the_call() {
    let vars = vec![cont_var(0, -5.0, 5.0)];
    let mut a = raw(&[1.0]);
    let mut b = raw(&[-2.0]);
    a.fitness = Some(vec![1.0]);
    b.fitness = Some(vec![2.0]);

    let mut rng = Port::new(vec![0.3], vec![]);
    ieee::sbx_crossover(&mut a, &mut b, &vars, 15.0, &mut rng).unwrap();

    assert!(a.fitness.is_none() && b.fitness.is_none());
    assert_ne!(a.genes[0].to_bits(), 1.0_f64.to_bits());
    assert_ne!(b.genes[0].to_bits(), (-2.0_f64).to_bits());
}

/// `lambda == 0` draws nothing; odd `lambda` still computes both SBX children;
/// a single parent with `lambda >= 2` consumes one branch + one index per child.
#[test]
fn var_or_handles_lambda_zero_odd_and_single_parent() {
    let vars = vec![cont_var(0, 0.0, 1.0)];
    let pop = vec![raw(&[0.1]), raw(&[0.9])];

    let mut rng = Port::new(vec![0.9; 8], vec![0; 8]);
    assert!(ieee::var_or(&pop, &vars, 0, 0.0, 0.0, 15.0, &mut rng)
        .unwrap()
        .is_empty());
    assert_eq!(rng.unit_draws() + rng.index_draws(), 0);

    let mut rng = Port::new(vec![0.0, 0.3], vec![0, 1]);
    let kids = ieee::var_or(&pop, &vars, 1, 1.0, 0.0, 15.0, &mut rng).unwrap();
    assert_eq!(kids.len(), 1);
    assert_eq!(rng.unit_draws(), 2, "branch draw + SBX spread draw");
    assert_eq!(rng.index_draws(), 2, "both parents selected");

    let mut rng = Port::new(vec![0.9, 0.9, 0.9], vec![0, 0, 0]);
    let kids = ieee::var_or(&[raw(&[0.4])], &vars, 3, 0.0, 0.0, 15.0, &mut rng).unwrap();
    assert_eq!(kids.len(), 3);
    assert_eq!((rng.unit_draws(), rng.index_draws()), (3, 3));
}

/// Raw genes are neither rounded nor validated against the integer domain; raw
/// and checked mutation agree bit-for-bit on a fractional integer gene.
#[test]
fn integer_gene_is_not_rounded_or_rejected() {
    let vars = vec![variable(
        0,
        VariableKind::integer(0, 10).expect("valid integer bounds"),
    )];
    let mut t = typed(&[3.75]);
    let mut r = raw(&[3.75]);
    let (mut tp, mut rp) = (
        Port::new(vec![0.0, 0.1], vec![]),
        Port::new(vec![0.0, 0.1], vec![]),
    );

    operators::polynomial_mutation(&mut t, &vars, 20.0, &mut tp).unwrap();
    ieee::polynomial_mutation(&mut r, &vars, 20.0, &mut rp).unwrap();

    assert_eq!(bits(&t.genes()), bits(&r.genes));
    assert_ne!(r.genes[0].fract().to_bits(), 0, "left as a raw float");
}

/// Raw and checked routes agree on the native Math `MamutRng` port for a
/// published seed and variation mix (default parity; no legacy seed expansion).
#[test]
fn raw_and_checked_agree_on_native_mamut_port() {
    let vars = vec![cont_var(0, 0.0, 1.0)];
    let pop_t = vec![typed(&[0.2]), typed(&[0.8])];
    let pop_r = vec![raw(&[0.2]), raw(&[0.8])];
    let mut trng = MamutRng::root(&Seed::from_u64(0));
    let mut rrng = MamutRng::root(&Seed::from_u64(0));

    let t = operators::var_or(&pop_t, &vars, 8, 0.7, 0.2, 15.0, &mut trng).unwrap();
    let r = ieee::var_or(&pop_r, &vars, 8, 0.7, 0.2, 15.0, &mut rrng).unwrap();

    assert_eq!(t.len(), r.len());
    for (checked, raw_child) in t.iter().zip(&r) {
        assert_eq!(bits(&checked.genes()), bits(&raw_child.genes));
    }
}
