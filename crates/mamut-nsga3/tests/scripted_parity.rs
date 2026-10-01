//! Scripted-RNG differential parity tests (extraction contract §D2.3 / H1(b)).
//!
//! For every RNG-consuming path (`sbx_crossover`, `polynomial_mutation`,
//! `var_or`, `init_population`) this module drives the crate and an independent
//! reference transcription of the **frozen** algorithm with the *same*
//! deterministic scripted stream, then asserts the outputs coincide. The
//! reference functions are copied line-by-line from the frozen
//! `operators.rs` (`git show 6db4862d…`) and are the documented derivation of
//! the expected behavior; the scripted stream is an in-test [`RngPort`] double,
//! so no `rand`/`rand_chacha` dependency is introduced.
//!
//! Differential expectation: same scripted stream in ⇒ same offspring/choice
//! decisions out (same betaq/delta formulas, same bounds/rounding, same
//! `var_or` swap/branch decisions). Values are compared at `1e-12` absolute;
//! the frozen formulas are reproduced verbatim so the results are expected to
//! be bit-identical.

use mamut_math_core::rng::MamutRng;
use mamut_math_core::{
    Candidate, Feasibility, ObjectiveValues, RngMetadata, RngPort, Seed, ValidFloat,
    RNG_ALGORITHM_ID, RNG_ALGORITHM_VERSION,
};

use mamut_nsga3::{init_population, polynomial_mutation, sbx_crossover, var_or, Individual};

// ───────────────────────── scripted RNG double ─────────────────────────

/// Deterministic in-test [`RngPort`] double backed by explicit scripted queues.
#[derive(Clone)]
struct ScriptedRng {
    units: Vec<f64>,
    words: Vec<u64>,
    unit_pos: usize,
    word_pos: usize,
}

impl ScriptedRng {
    fn new(units: Vec<f64>, words: Vec<u64>) -> Self {
        Self {
            units,
            words,
            unit_pos: 0,
            word_pos: 0,
        }
    }
}

impl RngPort for ScriptedRng {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        let value = self.words[self.word_pos];
        self.word_pos += 1;
        value
    }

    fn next_unit_f64(&mut self) -> f64 {
        let value = self.units[self.unit_pos];
        self.unit_pos += 1;
        value
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        for chunk in dst.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            for (slot, byte) in chunk.iter_mut().zip(bytes) {
                *slot = byte;
            }
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
        Box::new(self.clone())
    }
}

fn scripted_units(count: usize) -> Vec<f64> {
    // Low-discrepancy golden-ratio sequence in [0, 1).
    (0..count)
        .map(|k| ((k as f64) * 0.618_033_988_749_894_9).fract())
        .collect()
}

fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn scripted_words(count: usize) -> Vec<u64> {
    (0..count as u64).map(splitmix64).collect()
}

// ───────────────────────── frozen reference transcription ─────────────────────────

// Copied verbatim from frozen `operators.rs` (`sbx_betaq`).
fn frozen_sbx_betaq(u: f64, alpha: f64, eta: f64) -> f64 {
    if u <= 1.0 / alpha {
        (u * alpha).powf(1.0 / (eta + 1.0))
    } else {
        (1.0 / (2.0 - u * alpha)).powf(1.0 / (eta + 1.0))
    }
}

// Copied verbatim from frozen `operators.rs` (`poly_mutation_delta`).
fn frozen_poly_delta(u: f64, x: f64, lb: f64, ub: f64, eta: f64, dx: f64) -> f64 {
    if u < 0.5 {
        let delta_l = (x - lb) / dx;
        let base = 2.0 * u + (1.0 - 2.0 * u) * (1.0 - delta_l).powf(eta + 1.0);
        base.powf(1.0 / (eta + 1.0)) - 1.0
    } else {
        let delta_r = (ub - x) / dx;
        let base = 2.0 * (1.0 - u) + 2.0 * (u - 0.5) * (1.0 - delta_r).powf(eta + 1.0);
        1.0 - base.powf(1.0 / (eta + 1.0))
    }
}

// Same documented `gen_range(0..n)`-equivalent used by the crate (reimplemented
// independently here; Lemire multiply-high with rejection).
fn frozen_index(rng: &mut dyn RngPort, n: usize) -> usize {
    let n = n as u64;
    let mut x = rng.next_u64();
    let mut m = (x as u128) * (n as u128);
    let mut low = m as u64;
    if low < n {
        let threshold = n.wrapping_neg() % n;
        while low < threshold {
            x = rng.next_u64();
            m = (x as u128) * (n as u128);
            low = m as u64;
        }
    }
    (m >> 64) as usize
}

// Copied from frozen `operators.rs::sbx_crossover` (bounds from GeneSpec replaced
// by an explicit `bounds` table; arithmetic and RNG order preserved).
fn frozen_sbx(
    g1: &mut [f64],
    g2: &mut [f64],
    bounds: &[(f64, f64)],
    eta: f64,
    rng: &mut dyn RngPort,
) {
    for (i, &(lb, ub)) in bounds.iter().enumerate() {
        let x1 = g1[i];
        let x2 = g2[i];
        if (x1 - x2).abs() < 1e-14 {
            continue;
        }
        let (x_lo, x_hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let dx = x_hi - x_lo;
        let u = rng.next_unit_f64();
        let beta1 = 1.0 + 2.0 * (x_lo - lb) / dx;
        let alpha1 = 2.0 - beta1.powf(-(eta + 1.0));
        let betaq1 = frozen_sbx_betaq(u, alpha1, eta);
        let beta2 = 1.0 + 2.0 * (ub - x_hi) / dx;
        let alpha2 = 2.0 - beta2.powf(-(eta + 1.0));
        let betaq2 = frozen_sbx_betaq(u, alpha2, eta);
        let c1 = (0.5 * ((x_lo + x_hi) - betaq1 * dx)).clamp(lb, ub);
        let c2 = (0.5 * ((x_lo + x_hi) + betaq2 * dx)).clamp(lb, ub);
        if x1 <= x2 {
            g1[i] = c1;
            g2[i] = c2;
        } else {
            g1[i] = c2;
            g2[i] = c1;
        }
    }
}

// Copied from frozen `operators.rs::polynomial_mutation` (returns whether any
// gene changed, i.e. whether fitness would be invalidated).
fn frozen_poly_mutation(
    genes: &mut [f64],
    bounds: &[(f64, f64)],
    eta: f64,
    rng: &mut dyn RngPort,
) -> bool {
    let indpb = 1.0 / bounds.len().max(1) as f64;
    let mut mutated = false;
    for (i, &(lb, ub)) in bounds.iter().enumerate() {
        if rng.next_unit_f64() > indpb {
            continue;
        }
        mutated = true;
        let x = genes[i];
        let dx = ub - lb;
        if dx < 1e-14 {
            continue;
        }
        let u = rng.next_unit_f64();
        let delta = frozen_poly_delta(u, x, lb, ub, eta, dx);
        genes[i] = (x + delta * dx).clamp(lb, ub);
    }
    mutated
}

// Copied from frozen `operators.rs::var_or` (exact branch/RNG order; both SBX
// children kept when there is room).
fn frozen_var_or(
    pop: &[Vec<f64>],
    bounds: &[(f64, f64)],
    lambda: usize,
    cxpb: f64,
    mutpb: f64,
    eta: f64,
    rng: &mut dyn RngPort,
) -> Vec<Vec<f64>> {
    let mut offspring: Vec<Vec<f64>> = Vec::with_capacity(lambda);
    let n = pop.len();
    while offspring.len() < lambda {
        let u = rng.next_unit_f64();
        if u < cxpb {
            let i1 = frozen_index(rng, n);
            let i2 = frozen_index(rng, n);
            let mut child1 = pop[i1].clone();
            let mut child2 = pop[i2].clone();
            frozen_sbx(&mut child1, &mut child2, bounds, eta, rng);
            offspring.push(child1);
            if offspring.len() < lambda {
                offspring.push(child2);
            }
        } else if u < cxpb + mutpb {
            let i = frozen_index(rng, n);
            let mut child = pop[i].clone();
            frozen_poly_mutation(&mut child, bounds, eta, rng);
            offspring.push(child);
        } else {
            let i = frozen_index(rng, n);
            offspring.push(pop[i].clone());
        }
    }
    offspring
}

// ───────────────────────── helpers ─────────────────────────

fn candidate_from(genes: &[f64]) -> Candidate {
    Candidate::new(genes.iter().map(|&g| ValidFloat::new(g).unwrap()).collect()).unwrap()
}

fn evaluated(genes: &[f64]) -> Individual {
    let fitness = ObjectiveValues::new(vec![ValidFloat::new(1.0).unwrap(); 5]).unwrap();
    Individual::evaluated(candidate_from(genes), fitness, Feasibility::Feasible)
}

fn variables(bounds: &[(f64, f64)]) -> Vec<mamut_math_core::Variable> {
    bounds
        .iter()
        .enumerate()
        .map(|(i, &(lb, ub))| {
            mamut_math_core::Variable::new(
                mamut_math_core::VariableId(i as u64),
                mamut_math_core::VariableKind::continuous(
                    ValidFloat::new(lb).unwrap(),
                    ValidFloat::new(ub).unwrap(),
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect()
}

fn assert_close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() < 1e-12, "{what}: crate={a} reference={b}");
}

// ───────────────────────── tests ─────────────────────────

/// Hand-computed single-gene SBX off a scripted `u = 0.3`, `eta = 2`.
///
/// Documented derivation: x1=2, x2=6, [lb,ub]=[0,10] ⇒ dx=4,
/// beta1=2, alpha1=1.875 ⇒ betaq1=(0.3·1.875)^(1/3)≈0.825482,
/// beta2=3, alpha2=1.962963 ⇒ betaq2=(0.3·1.962963)^(1/3)≈0.838217,
/// c1=0.5·(8 − 4·betaq1)≈2.349035, c2=0.5·(8 + 4·betaq2)≈5.676434.
#[test]
fn sbx_single_gene_hand_computed() {
    let vars = variables(&[(0.0, 10.0)]);
    let mut c1 = Individual::new(candidate_from(&[2.0]));
    let mut c2 = Individual::new(candidate_from(&[6.0]));
    let mut rng = ScriptedRng::new(vec![0.3], vec![]);
    sbx_crossover(&mut c1, &mut c2, &vars, 2.0, &mut rng).unwrap();
    let g1 = c1.genes();
    let g2 = c2.genes();
    assert!((g1[0] - 2.349_036_376).abs() < 1e-9, "c1={}", g1[0]);
    assert!((g2[0] - 5.676_387_636).abs() < 1e-9, "c2={}", g2[0]);
}

/// SBX outputs coincide with the frozen formula for a scripted stream.
#[test]
fn sbx_scripted_matches_frozen_formula() {
    let bounds = [
        (-2.0, 3.0),
        (0.0, 1.0),
        (1.0, 1.5),
        (-5.0, 5.0),
        (60.0, 120.0),
    ];
    let vars = variables(&bounds);
    let p1 = [0.3, 0.6, 1.1, 1.0, 70.0];
    let p2 = [1.5, 0.2, 1.4, -2.0, 110.0];

    for trial in 0..25 {
        let units = scripted_units(100 + trial);
        let offset = trial; // vary the alignment of the stream across trials.
        let probe: Vec<f64> = units[offset..].to_vec();

        let mut crate1 = Individual::new(candidate_from(&p1));
        let mut crate2 = Individual::new(candidate_from(&p2));
        let mut crate_rng = ScriptedRng::new(probe.clone(), vec![]);
        sbx_crossover(&mut crate1, &mut crate2, &vars, 15.0, &mut crate_rng).unwrap();

        let mut ref1 = p1.to_vec();
        let mut ref2 = p2.to_vec();
        let mut ref_rng = ScriptedRng::new(probe, vec![]);
        frozen_sbx(&mut ref1, &mut ref2, &bounds, 15.0, &mut ref_rng);

        for i in 0..bounds.len() {
            assert_close(crate1.genes()[i], ref1[i], "sbx child1 gene");
            assert_close(crate2.genes()[i], ref2[i], "sbx child2 gene");
        }
        // Cached fitness is invalidated on both offspring (DEAP semantics).
        assert!(crate1.fitness.is_none() && crate2.fitness.is_none());
    }
}

/// Polynomial-mutation single-gene hand-computed value (`u = 0.3`, `eta = 2`).
///
/// Documented derivation: one gene ⇒ indpb=1 (always mutate); x=2, [0,10],
/// ∆_l=0.2, base=2·0.3 + 0.4·0.8³ = 0.8048 ⇒ delta=0.8048^(1/3)−1≈−0.069829,
/// x' = 2 − 0.698293 ≈ 1.301707.
#[test]
fn poly_mutation_single_gene_hand_computed() {
    let vars = variables(&[(0.0, 10.0)]);
    let mut ind = Individual::new(candidate_from(&[2.0]));
    let mut rng = ScriptedRng::new(vec![0.3, 0.3], vec![]);
    polynomial_mutation(&mut ind, &vars, 2.0, &mut rng).unwrap();
    assert!(
        (ind.genes()[0] - 1.301_707_013).abs() < 1e-9,
        "x={}",
        ind.genes()[0]
    );
}

/// Polynomial-mutation outputs coincide with the frozen formula.
#[test]
fn poly_mutation_scripted_matches_frozen_formula() {
    let bounds = [
        (-2.0, 3.0),
        (0.0, 1.0),
        (1.0, 1.5),
        (-5.0, 5.0),
        (60.0, 120.0),
    ];
    let vars = variables(&bounds);
    let base = [0.3, 0.6, 1.1, 1.0, 70.0];

    for trial in 0..25 {
        let units: Vec<f64> = scripted_units(200 + trial)[trial..].to_vec();

        let mut crate_ind = evaluated(&base);
        let mut crate_rng = ScriptedRng::new(units.clone(), vec![]);
        polynomial_mutation(&mut crate_ind, &vars, 20.0, &mut crate_rng).unwrap();

        let mut ref_genes = base.to_vec();
        let mut ref_rng = ScriptedRng::new(units, vec![]);
        let ref_mutated = frozen_poly_mutation(&mut ref_genes, &bounds, 20.0, &mut ref_rng);

        let crate_genes = crate_ind.genes();
        for (i, &expected) in ref_genes.iter().enumerate() {
            assert_close(crate_genes[i], expected, "poly mutation gene");
        }
        // Divergence vs DEAP: fitness invalidated only when a gene changed.
        assert_eq!(crate_ind.fitness.is_none(), ref_mutated);
    }
}

/// `var_or` reproduces the frozen branch/RNG decisions and offspring.
#[test]
fn var_or_scripted_matches_frozen_decisions() {
    let bounds = [
        (-2.0, 3.0),
        (0.0, 1.0),
        (1.0, 1.5),
        (-5.0, 5.0),
        (60.0, 120.0),
    ];
    let vars = variables(&bounds);

    let pool_genes: Vec<Vec<f64>> = (0..6)
        .map(|j| {
            bounds
                .iter()
                .enumerate()
                .map(|(i, &(lb, ub))| {
                    let frac = ((j as f64) * 0.37 + (i as f64) * 0.11).fract();
                    lb + (ub - lb) * frac
                })
                .collect()
        })
        .collect();
    let pool: Vec<Individual> = pool_genes.iter().map(|g| evaluated(g)).collect();

    let (cxpb, mutpb, eta, lambda) = (0.7, 0.2, 15.0, 24);
    let units = scripted_units(2000);
    let words = scripted_words(2000);

    let mut crate_rng = ScriptedRng::new(units.clone(), words.clone());
    let offspring = var_or(&pool, &vars, lambda, cxpb, mutpb, eta, &mut crate_rng).unwrap();
    assert_eq!(offspring.len(), lambda);

    let mut ref_rng = ScriptedRng::new(units, words);
    let reference = frozen_var_or(&pool_genes, &bounds, lambda, cxpb, mutpb, eta, &mut ref_rng);
    assert_eq!(reference.len(), lambda);

    for (j, (child, ref_genes)) in offspring.iter().zip(reference.iter()).enumerate() {
        let genes = child.genes();
        for (i, &expected) in ref_genes.iter().enumerate().take(bounds.len()) {
            assert_close(
                genes[i],
                expected,
                &format!("var_or offspring[{j}] gene[{i}]"),
            );
        }
        // Reproduction clones stay evaluated; SBX/mutation clear fitness.
        if child.fitness.is_none() {
            // Fine: a crossover/mutation child.
        } else {
            // A reproduction clone must equal some parent exactly.
            assert!(
                pool_genes.contains(&genes),
                "evaluated child must be an exact clone"
            );
        }
    }
}

/// `init_population` matches frozen uniform-box sampling (Continuous/Integer).
#[test]
fn init_population_scripted_matches_frozen_box_sampling() {
    let bounds = [
        (-2.0, 3.0),
        (0.0, 1.0),
        (1.0, 1.5),
        (-5.0, 5.0),
        (60.0, 120.0),
    ];
    let vars = variables(&bounds);
    let units = scripted_units(500);

    let mut crate_rng = ScriptedRng::new(units.clone(), vec![]);
    let population = init_population(&vars, 20, &mut crate_rng).unwrap();
    assert_eq!(population.len(), 20);

    let mut ref_rng = ScriptedRng::new(units, vec![]);
    for ind in &population {
        let genes = ind.genes();
        for i in 0..bounds.len() {
            let (lb, ub) = bounds[i];
            let expected = lb + (ub - lb) * ref_rng.next_unit_f64();
            assert_close(genes[i], expected, "init_population gene");
        }
        assert!(ind.fitness.is_none() && ind.feasibility.is_none());
    }
}

/// Determinism: the same core `Seed` produces identical offspring; a different
/// seed produces different offspring.
#[test]
fn var_or_same_seed_is_deterministic() {
    let bounds = [(-2.0, 3.0), (0.0, 1.0), (1.0, 1.5)];
    let vars = variables(&bounds);
    let pool_genes: Vec<Vec<f64>> = (0..5).map(|j| vec![j as f64 * 0.1, 0.5, 1.2]).collect();
    let pool: Vec<Individual> = pool_genes.iter().map(|g| evaluated(g)).collect();

    let run = |seed: u64| {
        let mut rng = MamutRng::root(&Seed::from_u64(seed));
        var_or(&pool, &vars, 12, 0.7, 0.2, 15.0, &mut rng)
            .unwrap()
            .iter()
            .map(Individual::genes)
            .collect::<Vec<_>>()
    };
    assert_eq!(run(42), run(42));
    assert_ne!(run(42), run(43));
}

/// The public `init_population` handles all four `VariableKind`s: Discrete
/// draws exact set members, Categorical draws indices in range.
#[test]
fn init_population_covers_discrete_and_categorical() {
    use mamut_math_core::{DiscreteSet, VariableId, VariableKind};

    let discrete = DiscreteSet::new(vec![
        ValidFloat::new(0.0).unwrap(),
        ValidFloat::new(2.5).unwrap(),
        ValidFloat::new(7.0).unwrap(),
    ])
    .unwrap();
    let vars = vec![
        mamut_math_core::Variable::new(
            VariableId(0),
            VariableKind::discrete(discrete.clone()).unwrap(),
        )
        .unwrap(),
        mamut_math_core::Variable::new(VariableId(1), VariableKind::categorical(4).unwrap())
            .unwrap(),
    ];

    let mut rng = MamutRng::root(&Seed::from_u64(7));
    let population = init_population(&vars, 200, &mut rng).unwrap();
    let members: Vec<f64> = discrete.values().iter().map(ValidFloat::get).collect();
    for ind in &population {
        let g = ind.genes();
        assert!(
            members.contains(&g[0]),
            "discrete gene {} must be a set member",
            g[0]
        );
        assert!(
            (0.0..4.0).contains(&g[1]),
            "categorical index {} out of range",
            g[1]
        );
        assert!((g[1] - g[1].round()).abs() < 1e-12);
    }
}
