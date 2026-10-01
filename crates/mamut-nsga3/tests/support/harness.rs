//! A test-local NSGA-III generation loop driven by the crate's public kernels.
//!
//! The loop mirrors the literature NSGA-III sequence (Deb & Jain 2014):
//! initialize -> produce offspring (SBX + polynomial mutation) -> combine the
//! parent and offspring pools -> nondominated sort -> fill whole fronts ->
//! normalize + associate + niche-select the splitting front. It composes the
//! public kernels only; it never calls [`mamut_nsga3::Nsga3Session`] or the
//! `select_environmental` wrapper, because loop composition is itself under
//! test here (session-level parity has its own suite in `session.rs`).
//!
//! The three selection sub-steps are injected as [`Pipeline`] function
//! pointers so the corruption suite can mutate one step at a time.

use mamut_math_core::rng::MamutRng;
use mamut_math_core::{Candidate, RngPort, Seed, ValidFloat, Variable, VariableId, VariableKind};
use mamut_nsga3::{
    associate, build_niche_counts, extract_pareto_front, fast_nondominated_sort, hypervolume,
    igd_plus, init_population, normalize, polynomial_mutation, sbx_crossover,
    select_from_partial_front, uniform_reference_points, Individual,
};

use super::dtlz::Problem;

/// Population size (pymoo matched: 92).
pub const POP: usize = 92;
/// Generations so that `POP + GENS * POP == 9200` evaluations.
pub const GENS: u32 = 99;
/// Seeds 0..7 (matched to the pymoo oracle).
pub const SEEDS: [u64; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
/// SBX distribution index (pymoo NSGA3 default eta=30).
pub const SBX_ETA: f64 = 30.0;
/// Polynomial-mutation distribution index (pymoo NSGA3 default eta=20).
pub const PM_ETA: f64 = 20.0;
/// Objective count of the DTLZ instances.
pub const N_OBJ: usize = 3;
/// Das-Dennis divisions (p=12 -> 91 reference directions).
pub const DIVISIONS: usize = 12;

/// Number of decision variables in a final population evaluation budget.
pub fn total_evaluations() -> u64 {
    (POP + GENS as usize * POP) as u64
}

/// Continuous `[0, 1]` variables for a DTLZ instance.
pub fn variables(n_var: usize) -> Vec<Variable> {
    (0..n_var)
        .map(|i| {
            Variable::new(
                VariableId(i as u64),
                VariableKind::continuous(
                    ValidFloat::new(0.0).unwrap(),
                    ValidFloat::new(1.0).unwrap(),
                )
                .unwrap(),
            )
            .unwrap()
        })
        .collect()
}

/// The das-dennis reference directions used by every quality run.
pub fn reference_directions() -> Vec<Vec<f64>> {
    uniform_reference_points(N_OBJ, DIVISIONS).expect("p=12 lattice is valid")
}

/// Uniform parent index over `0..n` from the public `RngPort` stream.
fn random_index(rng: &mut dyn RngPort, n: usize) -> usize {
    let u = rng.next_unit_f64();
    ((u * n as f64) as usize).min(n - 1)
}

/// Canonical NSGA-III mating: for each offspring pair, SBX crossover of two
/// parents followed by polynomial mutation of BOTH children (indpb = 1/n).
///
/// This follows the Deb & Jain 2014 sequence and pymoo NSGA3's default mating
/// (`SBX(eta=30, prob=1.0)` then `PM(eta=20, indpb=1/n)`), rather than DEAP's
/// `varOr` crossover-XOR-mutation operator. Parent selection is a
/// tournament-equivalent uniform random draw (pymoo's default tournament
/// comparator reduces to a random winner for unconstrained problems).
pub fn canonical_mating(
    population: &[Individual],
    variables: &[Variable],
    rng: &mut dyn RngPort,
) -> Vec<Individual> {
    let n = population.len();
    let mut offspring: Vec<Individual> = Vec::with_capacity(POP);
    while offspring.len() < POP {
        let i1 = random_index(rng, n);
        let i2 = random_index(rng, n);
        let mut child1 = population[i1].clone();
        let mut child2 = population[i2].clone();
        sbx_crossover(&mut child1, &mut child2, variables, SBX_ETA, rng).expect("valid sbx");
        polynomial_mutation(&mut child1, variables, PM_ETA, rng).expect("valid pm");
        polynomial_mutation(&mut child2, variables, PM_ETA, rng).expect("valid pm");
        offspring.push(child1);
        if offspring.len() < POP {
            offspring.push(child2);
        }
    }
    offspring
}

// ───────────────────────── injectable selection pipeline ─────────────────────

pub type SortFn = fn(&[Vec<f64>]) -> Vec<Vec<usize>>;
pub type NormalizeFn = fn(&[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<f64>);
pub type PartialSelectFn = fn(&[usize], &[(usize, f64)], &mut [u32], usize) -> Vec<usize>;

/// The three injectable selection sub-steps.
pub struct Pipeline {
    pub sort: SortFn,
    pub normalize: NormalizeFn,
    pub partial_select: PartialSelectFn,
}

/// The honest baseline pipeline: every step is the crate kernel directly.
pub fn baseline_pipeline() -> Pipeline {
    Pipeline {
        sort: |fitnesses| fast_nondominated_sort(fitnesses).expect("validated fitness matrix"),
        normalize: |fitnesses| {
            let result = normalize(fitnesses).expect("validated fitness matrix");
            (result.normalized, result.intercepts)
        },
        partial_select: select_from_partial_front,
    }
}

/// Environmental selection composed from the injected pipeline steps.
///
/// Returns survivor indices into `fitnesses` (length `min(n, len)`).
pub fn select_with_pipeline(
    fitnesses: &[Vec<f64>],
    n: usize,
    ref_points: &[Vec<f64>],
    pipeline: &Pipeline,
) -> Vec<usize> {
    let pop_size = fitnesses.len();
    if pop_size == 0 || n == 0 {
        return Vec::new();
    }
    if n >= pop_size {
        return (0..pop_size).collect();
    }

    let fronts = (pipeline.sort)(fitnesses);

    let mut selected: Vec<usize> = Vec::with_capacity(n);
    let mut partial_front: Vec<usize> = Vec::new();
    for front in &fronts {
        if selected.len() + front.len() <= n {
            selected.extend_from_slice(front);
        } else {
            partial_front = front.clone();
            break;
        }
        if selected.len() == n {
            return selected;
        }
    }
    if selected.len() == n {
        return selected;
    }

    let needed = n - selected.len();
    let all_candidates: Vec<usize> = selected
        .iter()
        .copied()
        .chain(partial_front.iter().copied())
        .collect();
    let candidate_fitnesses: Vec<Vec<f64>> = all_candidates
        .iter()
        .map(|&i| fitnesses[i].clone())
        .collect();

    let (normalized, _intercepts) = (pipeline.normalize)(&candidate_fitnesses);

    // Association is a shared kernel; only normalization and the partial-front
    // selection are injectable.
    let candidate_assoc = associate(&normalized, ref_points).expect("validated arities");
    let mut assoc: Vec<(usize, f64)> = vec![(0, f64::INFINITY); pop_size];
    for (pos, &orig_idx) in all_candidates.iter().enumerate() {
        assoc[orig_idx] = candidate_assoc[pos];
    }

    let mut niche_count = build_niche_counts(&selected, &assoc, ref_points.len());
    let extra = (pipeline.partial_select)(&partial_front, &assoc, &mut niche_count, needed);
    selected.extend(extra);
    selected
}

// ───────────────────────── generation loop ─────────────────────────

/// pymoo-faithful bounded SBX (per-variable `prob_var=0.5`, `prob_bin=0.5`).
///
/// This reproduces pymoo's `cross_sbx` semantics exactly: each differing gene
/// is recombined with probability 0.5, using a single `rand` per crossed gene,
/// and the two children are exchanged with probability 0.5. It is a TEST-HARNESS
/// emulation used to align the quality loop with pymoo's operator defaults; the
/// crate's own `sbx_crossover` (DEAP lineage: every differing gene crossed,
/// deterministic child assignment) is left untouched and is independently
/// validated by `conformance_components.rs`. Variation composition is a caller
/// choice (documented DoD item 6 operator/default difference).
fn pymoo_sbx(
    child1: &mut Individual,
    child2: &mut Individual,
    variables: &[Variable],
    eta: f64,
    rng: &mut dyn RngPort,
) {
    let mut g1 = child1.genes();
    let mut g2 = child2.genes();
    for (i, variable) in variables.iter().enumerate() {
        let (lb, ub) = match variable.kind() {
            VariableKind::Continuous { lower, upper } => (lower.get(), upper.get()),
            _ => panic!("pymoo_sbx: continuous variables expected"),
        };
        let x1 = g1[i];
        let x2 = g2[i];
        if (x1 - x2).abs() <= 1e-14 {
            continue;
        }
        if rng.next_unit_f64() > 0.5 {
            continue; // prob_var = 0.5
        }
        let (x_lo, x_hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let dx = x_hi - x_lo;
        let u = rng.next_unit_f64();
        let betaq = |beta: f64| -> f64 {
            let alpha = 2.0 - beta.powf(-(eta + 1.0));
            if u <= 1.0 / alpha {
                (u * alpha).powf(1.0 / (eta + 1.0))
            } else {
                (1.0 / (2.0 - u * alpha)).powf(1.0 / (eta + 1.0))
            }
        };
        let beta1 = 1.0 + 2.0 * (x_lo - lb) / dx;
        let low_child = 0.5 * ((x_lo + x_hi) - betaq(beta1) * dx);
        let beta2 = 1.0 + 2.0 * (ub - x_hi) / dx;
        let high_child = 0.5 * ((x_lo + x_hi) + betaq(beta2) * dx);
        let (mut c1, mut c2) = if x1 <= x2 {
            (low_child, high_child)
        } else {
            (high_child, low_child)
        };
        if rng.next_unit_f64() < 0.5 {
            std::mem::swap(&mut c1, &mut c2); // prob_bin = 0.5
        }
        g1[i] = c1.clamp(lb, ub);
        g2[i] = c2.clamp(lb, ub);
    }
    child1.candidate =
        Candidate::new(g1.iter().map(|&g| ValidFloat::new(g).unwrap()).collect()).unwrap();
    child2.candidate =
        Candidate::new(g2.iter().map(|&g| ValidFloat::new(g).unwrap()).collect()).unwrap();
}

/// Canonical NSGA-III mating with the pymoo-faithful SBX and the crate's
/// `polynomial_mutation` (eta=20, indpb=1/n), matching pymoo NSGA3 defaults.
fn pymoo_mating(
    population: &[Individual],
    variables: &[Variable],
    rng: &mut dyn RngPort,
) -> Vec<Individual> {
    let n = population.len();
    let mut offspring: Vec<Individual> = Vec::with_capacity(POP);
    while offspring.len() < POP {
        let i1 = random_index(rng, n);
        let i2 = random_index(rng, n);
        let mut child1 = population[i1].clone();
        let mut child2 = population[i2].clone();
        pymoo_sbx(&mut child1, &mut child2, variables, SBX_ETA, rng);
        polynomial_mutation(&mut child1, variables, PM_ETA, rng).expect("valid pm");
        polynomial_mutation(&mut child2, variables, PM_ETA, rng).expect("valid pm");
        offspring.push(child1);
        if offspring.len() < POP {
            offspring.push(child2);
        }
    }
    offspring
}

/// Run the NSGA-III loop and return the final population's objective rows.
///
/// The final population is the survivor set after the last environmental
/// selection (size `POP`), after exactly `POP + GENS * POP` evaluations.
pub fn run_front(problem: &Problem, seed: u64, pipeline: &Pipeline) -> Vec<Vec<f64>> {
    run_front_with(problem, seed, pipeline, GENS)
}

/// As [`run_front`] but with an explicit generation count (probe/diagnostics).
pub fn run_front_with(
    problem: &Problem,
    seed: u64,
    pipeline: &Pipeline,
    generations: u32,
) -> Vec<Vec<f64>> {
    let variables = variables(problem.n_var);
    let ref_points = reference_directions();
    let mut root = MamutRng::root(&Seed::from_u64(seed));

    let mut population = init_population(&variables, POP, &mut root).expect("valid variables");
    let mut fitnesses: Vec<Vec<f64>> = population
        .iter()
        .map(|individual| (problem.evaluate)(&individual.genes()))
        .collect();

    for generation in 0..generations {
        let mut offspring_rng = root.derive_child(generation as u64);
        let offspring = pymoo_mating(&population, &variables, &mut offspring_rng);
        let offspring_fitnesses: Vec<Vec<f64>> = offspring
            .iter()
            .map(|individual| (problem.evaluate)(&individual.genes()))
            .collect();

        let mut combined = population.clone();
        combined.extend(offspring);
        let mut combined_fitnesses = fitnesses.clone();
        combined_fitnesses.extend(offspring_fitnesses);

        let selected = select_with_pipeline(&combined_fitnesses, POP, &ref_points, pipeline);
        population = selected.iter().map(|&i| combined[i].clone()).collect();
        fitnesses = selected
            .iter()
            .map(|&i| combined_fitnesses[i].clone())
            .collect();
    }

    fitnesses
}

/// The nondominated (rank-1) subset of a fitness matrix, preserving order.
pub fn nondominated_rows(fitnesses: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let indices = extract_pareto_front(fitnesses).expect("validated fitness matrix");
    indices.iter().map(|&i| fitnesses[i].clone()).collect()
}

// ───────────────────────── quality metrics and band ─────────────────────────

/// Per-seed quality metrics for one problem.
#[derive(Clone, Copy, Debug)]
pub struct SeedMetrics {
    pub hv: f64,
    pub igd_plus: f64,
}

/// Compute our HV / IGD+ for one seed using the crate metrics on the final
/// nondominated front.
pub fn seed_metrics(
    problem: &Problem,
    seed: u64,
    hv_reference: &[f64],
    reference_front: &[Vec<f64>],
    pipeline: &Pipeline,
) -> SeedMetrics {
    let final_population = run_front(problem, seed, pipeline);
    let front = nondominated_rows(&final_population);
    SeedMetrics {
        hv: hypervolume(&front, hv_reference).expect("finite front"),
        igd_plus: igd_plus(&front, reference_front).expect("finite fronts"),
    }
}

/// Aggregate comparison of our per-seed metrics against the pymoo oracle.
pub struct BandReport {
    pub problem: &'static str,
    pub ours: Vec<SeedMetrics>,
    pub oracle_hv: Vec<f64>,
    pub oracle_igd: Vec<f64>,
}

impl BandReport {
    pub fn median_hv_ours(&self) -> f64 {
        median(&self.ours.iter().map(|m| m.hv).collect::<Vec<_>>())
    }

    pub fn median_hv_oracle(&self) -> f64 {
        median(&self.oracle_hv)
    }

    pub fn max_igd_ours(&self) -> f64 {
        self.ours
            .iter()
            .map(|m| m.igd_plus)
            .fold(f64::NEG_INFINITY, f64::max)
    }

    pub fn median_igd_oracle(&self) -> f64 {
        median(&self.oracle_igd)
    }
}

/// Median of a non-empty slice (average of the two central values when even).
pub fn median(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "median of empty slice");
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
    }
}

/// Build a band report for one problem over the given seeds.
pub fn band_report(
    problem: &Problem,
    hv_reference: &[f64],
    reference_front: &[Vec<f64>],
    oracle_hv: &[f64],
    oracle_igd: &[f64],
    seeds: &[u64],
    pipeline: &Pipeline,
) -> BandReport {
    let ours = seeds
        .iter()
        .map(|&seed| seed_metrics(problem, seed, hv_reference, reference_front, pipeline))
        .collect();
    BandReport {
        problem: problem.name,
        ours,
        oracle_hv: oracle_hv.to_vec(),
        oracle_igd: oracle_igd.to_vec(),
    }
}

/// Verdict of the quality gate for one problem.
pub struct GateVerdict {
    pub ok: bool,
    pub detail: String,
}

/// Per-problem quality-band specification.
///
/// The bands are pre-registered and justified per problem (see
/// `provenance.md`); the DTLZ1 and DTLZ7 bands are wider than the DTLZ2 5%
/// band because of the matched-budget degeneracy and the disconnected front
/// respectively. The raw medians are always reported so the gap is visible.
#[derive(Clone, Copy, Debug)]
pub struct BandSpec {
    /// Minimum `median_hv_ours / median_hv_oracle` when the oracle median is
    /// non-degenerate.
    pub hv_min_ratio: f64,
    /// Oracle median HV below which a relative band is not informative.
    pub hv_degenerate_floor: f64,
    /// Absolute HV slack in the degenerate case (`ours >= oracle - slack`).
    pub hv_abs_floor: f64,
    /// Absolute IGD+ floor added to `2 x median(oracle)`.
    pub igd_floor: f64,
}

/// Evaluate the quality gate for one problem report.
///
/// - Hypervolume: a two-sided band around the oracle median when that median is
///   non-degenerate (`>= hv_degenerate_floor`): `ours >= hv_min_ratio * oracle`
///   and `ours <= 1.05 * oracle`. Otherwise (degenerate oracle median) the gate
///   requires `ours >= oracle - hv_abs_floor`, because a relative band around a
///   near-zero median is not informative.
/// - IGD+: `max(ours) <= 2 * median(oracle) + igd_floor`.
pub fn evaluate_gate(report: &BandReport, spec: BandSpec) -> GateVerdict {
    let hv_ours = report.median_hv_ours();
    let hv_oracle = report.median_hv_oracle();
    let igd_ours = report.max_igd_ours();
    let igd_oracle = report.median_igd_oracle();

    let (hv_ok, hv_mode) = if hv_oracle >= spec.hv_degenerate_floor {
        (
            hv_ours >= spec.hv_min_ratio * hv_oracle && hv_ours <= 1.05 * hv_oracle,
            "relative-band",
        )
    } else {
        (
            hv_ours >= hv_oracle - spec.hv_abs_floor,
            "degenerate-absolute",
        )
    };
    let igd_ok = igd_ours <= 2.0 * igd_oracle + spec.igd_floor;

    GateVerdict {
        ok: hv_ok && igd_ok,
        detail: format!(
            "problem={} hv_mode={} median_hv_ours={hv_ours:.6e} median_hv_oracle={hv_oracle:.6e} \
             hv_ratio={:.4} hv_ok={hv_ok} max_igd_ours={igd_ours:.6e} \
             median_igd_oracle={igd_oracle:.6e} igd_band={:.6e} igd_ok={igd_ok}",
            report.problem,
            hv_mode,
            hv_ours / hv_oracle.max(f64::MIN_POSITIVE),
            2.0 * igd_oracle + spec.igd_floor,
        ),
    }
}
