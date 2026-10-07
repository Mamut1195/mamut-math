//! MATH-E2E-001 T2a — index-sampling behavior of the public NSGA-III operators.
//!
//! This integration test covers the **MAMUT-owned** index-sampling seam used by
//! the genetic operators:
//! - the native default (`RngPort::next_index` default body over the pre-port
//!   multiply-high-with-rejection sampler) reached through the public
//!   `var_or` / `init_population` entry points with a port that does **not**
//!   override `next_index` (existing-implementer compatibility), and
//! - an explicit override, proving that the public operators dispatch to
//!   `RngPort::next_index` instead of keeping a private sampling body.
//!
//! Synthetic unit coverage only: marker populations, no engineering case, no
//! new dependency. The pinned native values were captured through the public
//! API *before* the port method existed; the override port returns a raw word
//! that a private sampling body would turn into parent `0` every time, so the
//! scripted selections prove which body actually ran.

use core::num::NonZeroUsize;

use mamut_math_core::{
    Candidate, DiscreteSet, MamutRng, RngMetadata, RngPort, Seed, ValidFloat, Variable, VariableId,
    VariableKind, RNG_ALGORITHM_ID, RNG_ALGORITHM_VERSION,
};
use mamut_nsga3::{init_population, var_or, Individual};

/// One continuous `[0, 1]` variable — enough to carry a distinct marker gene.
fn continuous_vars() -> Vec<Variable> {
    vec![Variable::new(
        VariableId(0),
        VariableKind::continuous(
            ValidFloat::new(0.0).expect("0 is finite"),
            ValidFloat::new(1.0).expect("1 is finite"),
        )
        .expect("valid bounds"),
    )
    .expect("valid variable")]
}

/// Build an unevaluated population whose single gene is the marker value.
fn marker_population(markers: &[f64]) -> Vec<Individual> {
    markers
        .iter()
        .map(|&marker| {
            let candidate = Candidate::new(vec![ValidFloat::new(marker).expect("finite marker")])
                .expect("valid candidate");
            Individual::new(candidate)
        })
        .collect()
}

/// Recover the chosen parent marker indices from pure-reproduction offspring.
fn chosen_indices(markers: &[f64], offspring: &[Individual]) -> Vec<usize> {
    offspring
        .iter()
        .map(|child| {
            let gene = child.genes()[0];
            markers
                .iter()
                .position(|&marker| marker == gene)
                .unwrap_or_else(|| panic!("offspring gene {gene} is not a parent marker"))
        })
        .collect()
}

/// A discrete variable with the given strictly ascending values.
fn discrete_var(values: [f64; 3]) -> Variable {
    Variable::new(
        VariableId(0),
        VariableKind::discrete(
            DiscreteSet::new(
                values
                    .iter()
                    .map(|&v| ValidFloat::new(v).expect("finite"))
                    .collect(),
            )
            .expect("strictly ascending set"),
        )
        .expect("valid kind"),
    )
    .expect("valid variable")
}

/// A categorical variable over `0..cardinality`.
fn categorical_var(cardinality: u64) -> Variable {
    Variable::new(
        VariableId(0),
        VariableKind::categorical(cardinality).expect("non-empty domain"),
    )
    .expect("valid variable")
}

/// Forwarding port that counts raw `u64` and unit draws.
///
/// It does **not** override `next_index`: this is the existing-implementer
/// compatibility case (a port written against the pre-port trait compiles
/// unchanged and receives the default sampler).
struct CountingPort<P: RngPort> {
    inner: P,
    word_draws: usize,
    unit_draws: usize,
}

impl<P: RngPort> CountingPort<P> {
    fn new(inner: P) -> Self {
        Self {
            inner,
            word_draws: 0,
            unit_draws: 0,
        }
    }

    fn word_draws(&self) -> usize {
        self.word_draws
    }

    fn unit_draws(&self) -> usize {
        self.unit_draws
    }
}

impl<P: RngPort> RngPort for CountingPort<P> {
    fn next_u32(&mut self) -> u32 {
        self.inner.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.word_draws += 1;
        self.inner.next_u64()
    }

    fn next_unit_f64(&mut self) -> f64 {
        self.unit_draws += 1;
        self.inner.next_unit_f64()
    }

    fn fill_bytes(&mut self, dst: &mut [u8]) {
        self.inner.fill_bytes(dst);
    }

    fn metadata(&self) -> RngMetadata {
        self.inner.metadata()
    }

    fn derive_child_port(&self, stream_index: u64) -> Box<dyn RngPort> {
        self.inner.derive_child_port(stream_index)
    }
}

/// A port with a fully scripted `next_index` and a raw word that a private
/// sampling body would map to index `0` for every bound: `next_u64` always
/// returns `1`, so a fallback body would never touch the script.
struct OverridePort {
    units: Vec<f64>,
    unit_pos: usize,
    indices: Vec<usize>,
    index_pos: usize,
    index_calls: usize,
    word_calls: usize,
}

impl OverridePort {
    fn new(units: Vec<f64>, indices: Vec<usize>) -> Self {
        Self {
            units,
            unit_pos: 0,
            indices,
            index_pos: 0,
            index_calls: 0,
            word_calls: 0,
        }
    }

    fn index_calls(&self) -> usize {
        self.index_calls
    }

    fn word_calls(&self) -> usize {
        self.word_calls
    }
}

impl RngPort for OverridePort {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        self.word_calls += 1;
        // For any bound n >= 1, x = 1 gives m = n and high word 0, so a private
        // multiply-high body would deterministically select index 0.
        1
    }

    fn next_unit_f64(&mut self) -> f64 {
        let value = self.units[self.unit_pos];
        self.unit_pos += 1;
        value
    }

    fn next_index(&mut self, upper_exclusive: NonZeroUsize) -> usize {
        let value = self.indices[self.index_pos];
        self.index_pos += 1;
        self.index_calls += 1;
        assert!(
            value < upper_exclusive.get(),
            "override must stay in range: {value} >= {}",
            upper_exclusive
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
        Box::new(OverridePort::new(self.units.clone(), self.indices.clone()))
    }
}

/// A port with a constant unit draw and a constant in-range `next_index`
/// override, used to drive every `var_or` branch while proving dispatch.
struct ConstantUnitPort {
    unit: f64,
    index: usize,
    index_calls: usize,
    word_calls: usize,
}

impl ConstantUnitPort {
    fn new(unit: f64, index: usize) -> Self {
        Self {
            unit,
            index,
            index_calls: 0,
            word_calls: 0,
        }
    }
}

impl RngPort for ConstantUnitPort {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        self.word_calls += 1;
        1
    }

    fn next_unit_f64(&mut self) -> f64 {
        self.unit
    }

    fn next_index(&mut self, upper_exclusive: NonZeroUsize) -> usize {
        self.index_calls += 1;
        assert!(
            self.index < upper_exclusive.get(),
            "override must stay in range: {} >= {}",
            self.index,
            upper_exclusive
        );
        self.index
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
        Box::new(ConstantUnitPort::new(self.unit, self.index))
    }
}

/// `var_or` must route parent selection through the overridden port method and
/// must not fall back to any private `next_u64` sampler.
#[test]
fn var_or_dispatches_to_overridden_next_index() {
    let vars = continuous_vars();
    let markers = [0.1, 0.2, 0.3, 0.4];
    let population = marker_population(&markers);
    let scripted = vec![2usize, 1, 3, 0, 2, 2, 1, 3];
    // u = 0.9 keeps every offspring in the reproduction branch
    // (cxpb = mutpb = 0), so exactly one index draw happens per offspring.
    let mut rng = OverridePort::new(vec![0.9; 8], scripted.clone());

    let offspring = var_or(&population, &vars, 8, 0.0, 0.0, 15.0, &mut rng).expect("valid var_or");

    assert_eq!(chosen_indices(&markers, &offspring), scripted);
    assert_eq!(rng.index_calls(), 8, "one index draw per offspring");
    assert_eq!(
        rng.word_calls(),
        0,
        "var_or must dispatch to RngPort::next_index, not a private raw-word sampler"
    );
}

/// `init_population` must route discrete and categorical index draws through
/// the overridden port method as well.
#[test]
fn init_population_dispatches_to_overridden_next_index() {
    let categorical = vec![categorical_var(5)];
    let categorical_script = vec![4usize, 0, 3, 1, 2, 4, 2, 0];
    let mut rng = OverridePort::new(Vec::new(), categorical_script.clone());
    let population = init_population(&categorical, 8, &mut rng).expect("valid init_population");
    let genes: Vec<f64> = population.iter().map(|ind| ind.genes()[0]).collect();
    assert_eq!(
        genes,
        categorical_script
            .iter()
            .map(|&i| i as f64)
            .collect::<Vec<f64>>()
    );
    assert_eq!(rng.index_calls(), 8);
    assert_eq!(rng.word_calls(), 0);

    let discrete = vec![discrete_var([0.25, 0.5, 0.75])];
    let discrete_script = vec![2usize, 0, 1, 1, 2, 0, 1, 2];
    let mut rng = OverridePort::new(Vec::new(), discrete_script.clone());
    let population = init_population(&discrete, 8, &mut rng).expect("valid init_population");
    let genes: Vec<f64> = population.iter().map(|ind| ind.genes()[0]).collect();
    assert_eq!(genes, vec![0.75, 0.25, 0.5, 0.5, 0.75, 0.25, 0.5, 0.75]);
    assert_eq!(rng.index_calls(), 8);
    assert_eq!(rng.word_calls(), 0);
}

/// Triangulation: all three `var_or` branches (crossover, mutation,
/// reproduction) draw their parent indices from the port override, with the
/// exact analytic draw counts and no raw-word fallback.
#[test]
fn var_or_all_branches_dispatch_to_next_index() {
    let vars = continuous_vars();
    let markers = [0.1, 0.2, 0.3, 0.4];
    let population = marker_population(&markers);

    // u = 0.1 < cxpb = 0.6: crossover, two index draws per iteration.
    let mut rng = ConstantUnitPort::new(0.1, 0);
    let offspring = var_or(&population, &vars, 8, 0.6, 0.2, 15.0, &mut rng).expect("valid var_or");
    assert_eq!(offspring.len(), 8);
    assert_eq!(rng.index_calls, 8);
    assert_eq!(rng.word_calls, 0);

    // cxpb <= u = 0.65 < cxpb + mutpb: mutation, one index draw per iteration.
    let mut rng = ConstantUnitPort::new(0.65, 0);
    let offspring = var_or(&population, &vars, 6, 0.6, 0.2, 15.0, &mut rng).expect("valid var_or");
    assert_eq!(offspring.len(), 6);
    assert_eq!(rng.index_calls, 6);
    assert_eq!(rng.word_calls, 0);

    // u = 0.9 >= cxpb + mutpb: reproduction, one index draw per offspring.
    let mut rng = ConstantUnitPort::new(0.9, 3);
    let offspring = var_or(&population, &vars, 5, 0.6, 0.2, 15.0, &mut rng).expect("valid var_or");
    assert_eq!(chosen_indices(&markers, &offspring), vec![3, 3, 3, 3, 3]);
    assert_eq!(rng.index_calls, 5);
    assert_eq!(rng.word_calls, 0);
}

/// Native default through `var_or`: the pre-port parent indices are reproduced
/// exactly, with one unit draw and one raw `u64` draw per offspring.
#[test]
fn var_or_default_port_matches_pre_port_indices() {
    let vars = continuous_vars();
    let markers = [0.1, 0.2, 0.3, 0.4];
    let population = marker_population(&markers);
    let mut rng = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));

    let offspring = var_or(&population, &vars, 8, 0.0, 0.0, 15.0, &mut rng).expect("valid var_or");

    assert_eq!(
        chosen_indices(&markers, &offspring),
        vec![1, 2, 3, 0, 2, 0, 2, 0]
    );
    assert_eq!(rng.unit_draws(), 8);
    assert_eq!(
        rng.word_draws(),
        8,
        "n = 4 is a power of two: exactly one raw u64 per index"
    );
}

/// Same pin for an odd parent count (`n = 3`, non-power bound).
#[test]
fn var_or_default_port_matches_pre_port_indices_for_n3() {
    let vars = continuous_vars();
    let markers = [0.25, 0.5, 0.75];
    let population = marker_population(&markers);
    let mut rng = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));

    let offspring = var_or(&population, &vars, 6, 0.0, 0.0, 15.0, &mut rng).expect("valid var_or");

    assert_eq!(chosen_indices(&markers, &offspring), vec![1, 2, 2, 0, 1, 0]);
    assert_eq!(rng.unit_draws(), 6);
    assert_eq!(rng.word_draws(), 6);
}

/// A single-parent population (`upper = 1`) still consumes one raw draw per
/// offspring through the real caller path: the trivial bound is not
/// short-circuited.
#[test]
fn var_or_single_parent_consumes_one_raw_draw_per_offspring() {
    let vars = continuous_vars();
    let markers = [0.5];
    let population = marker_population(&markers);
    let mut rng = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));

    let offspring = var_or(&population, &vars, 3, 0.0, 0.0, 15.0, &mut rng).expect("valid var_or");

    assert_eq!(chosen_indices(&markers, &offspring), vec![0, 0, 0]);
    assert_eq!(rng.unit_draws(), 3);
    assert_eq!(
        rng.word_draws(),
        3,
        "upper = 1 must still consume its historical raw draw"
    );
}

/// Native default through `init_population`: the pre-port discrete and
/// categorical index streams are reproduced exactly.
#[test]
fn init_population_default_port_matches_pre_port_indices() {
    let categorical = vec![categorical_var(5)];
    let mut rng = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));
    let population = init_population(&categorical, 8, &mut rng).expect("valid init_population");
    let genes: Vec<f64> = population.iter().map(|ind| ind.genes()[0]).collect();
    assert_eq!(genes, vec![2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 1.0, 0.0]);
    assert_eq!(rng.word_draws(), 8);
    assert_eq!(rng.unit_draws(), 0);

    let discrete = vec![discrete_var([0.25, 0.5, 0.75])];
    let mut rng = CountingPort::new(MamutRng::root(&Seed::from_u64(42)));
    let population = init_population(&discrete, 8, &mut rng).expect("valid init_population");
    let genes: Vec<f64> = population.iter().map(|ind| ind.genes()[0]).collect();
    assert_eq!(genes, vec![0.5, 0.5, 0.75, 0.75, 0.5, 0.75, 0.5, 0.25]);
    assert_eq!(rng.word_draws(), 8);
    assert_eq!(rng.unit_draws(), 0);
}
