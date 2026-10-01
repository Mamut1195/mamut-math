# EXTRACTION CONTRACT — MATH-NSGA-002 · Extract generic NSGA-III implementation

- **Ticket**: MATH-NSGA-002 · Extract generic NSGA-III implementation (Risk L3, MATH_VERIFY=ON)
- **Kind**: EXTRACTION CONTRACT ONLY — authorizes the implementation pass, contains no implementation code.
- **Status**: CONTRACT (no `.rs` file is created, moved, or edited by this document).
- **Frozen inputs**:
  - Characterization boundary (MATH-NSGA-001, accepted): `mamut-math/docs/characterization/00..07.md` at mamut-math HEAD `726d3302ff46c183ade754049630aad3f5a1c510`.
  - Source implementation, RE-FROZEN this session at `6db4862d7a4035de8d494cdc62607b12b4edb103` in `C:\Users\Jonnathan\Mamut system\mamut-harness\motores\optimizador-hidraulico-rust\hydro-optimizer\src\` — verified `git rev-parse HEAD == 6db4862d…`, `git status --short` shows ONLY `M mamut.engine.json` (pre-existing manifest dirt, untouched). All `.rs` evidence below was read read-only via `git show 6db4862d…:<path>`.
  - Ticket canon scope: reference points, sorting, normalization, association/niching, selection, SBX/polynomial mutation ONLY where generic, ask/tell-compatible session. Non-goals: hydraulic encoding, ObjectiveFunction, norms, solver calls. DoD: generic Candidate/Evaluation contracts accepted; NO dependency on hydro-* crates; seeded compact fixtures preserve expected selection/offspring behavior; consumable by a future Hydraulics compatibility layer. Validation: differential tests vs frozen old implementation + independent review of Deb/Jain algorithm steps.
  - MAMUT Math core contracts on main `726d330`: `crates/mamut-math-core` — `Candidate`, `EvaluationOutcome`, `ObjectiveValues`, `Feasibility`, `Diagnostics`, `Variable`/`VariableKind`/`ValidFloat`, `codec`, `rng` (`MamutRng`/`RngPort`/`Seed`/`RngMetadata`, `mamut.chacha20-std` v1), `evidence` (`RunManifest`, `RngSpec`, `StopReason`, …). NSGA MUST consume this frozen RNG contract; no new RNG.

---

## A. Crate layout

### A1. Exact target crate name per frozen DAG: `mamut-nsga3`

The frozen DAG name is **`mamut-nsga3`** — NOT `mamut-math-nsga3`. Evidence:

- Doc 02 classification matrix: *"Hydraulics-owned code that must be re-expressed against future `mamut-nsga3` traits"* (§ legend) and rollback doc 07 §2.3–2.4 (`[dependencies] mamut-nsga3 = { … }`, *"delete the mamut-nsga3 usage"*).
- Doc 03 seam notes §4 title: *"API-shape implications for a future `mamut-nsga3` crate"*; §4 governing architecture cites the **ARC-002 crate freeze (`core/nsga3/sensitivity/gradient/hybrid/backend/testkit/wasm`)**.
- **Disclosure**: no `docs/ADRs/` directory exists on disk at `726d330` (`git ls-files` returns only `docs/characterization/*`, `crates/mamut-math-core/*`, workspace roots). The DAG name is therefore sourced from the accepted characterization docs (01 §3, 02, 03 §4, 07 §2.3–2.4), which are unanimous: short name `mamut-nsga3`. If a future ADR renames it, this contract follows the ADR; until then `mamut-nsga3` is normative.

### A2. Paths and workspace registration

- New crate path: `crates/mamut-nsga3/` (sibling of `crates/mamut-math-core/`).
  - `crates/mamut-nsga3/Cargo.toml` — package `mamut-nsga3`, `version.workspace = true`, `edition.workspace = true`, `rust-version.workspace = true`, `license.workspace = true` (same four-line shape as `mamut-math-core/Cargo.toml`).
  - `crates/mamut-nsga3/src/*.rs` — modules per §B (mirror of frozen source basenames, §B table).
  - `crates/mamut-nsga3/tests/*.rs` — parity fixtures per §D (expected-value fixtures + differential tests, no live hydro dependency).
- Workspace `Cargo.toml`: extend `members = ["crates/mamut-math-core"]` → `members = ["crates/mamut-math-core", "crates/mamut-nsga3"]`. No other workspace field changes (resolver 2, edition 2021, version 0.1.0, rust-version 1.86.0, MIT OR Apache-2.0 all frozen).
- Dependency direction (frozen DAG): `mamut-nsga3 → mamut-math-core` ONLY. `mamut-math-core` gains no dependency. No dev-dependency may introduce a third-party crate either (see A4).

### A3. Dependencies: ONLY `mamut-math-core` + std

```toml
[dependencies]
mamut-math-core = { path = "../mamut-math-core", version = "0.1" }
```

- No `[dependencies]` entry other than `mamut-math-core`. In particular: NO `rand`, NO `rand_chacha`, NO `serde`/`serde_json`, NO `geo`, NO `rayon`, NO hydro-* path dependency.
- `std` only (plus `core`). `forbid(unsafe_code)` and `#![deny(missing_docs)]` at the crate root, mirroring core `lib.rs:1-2`.
- RNG comes exclusively through the frozen core contract: `mamut_math_core::rng::{MamutRng, RngPort, Seed}`. Stochastic functions take `&mut dyn RngPort` (object-safe, MAMUT-owned trait) — never a concrete third-party RNG type. Uniform draws use `RngPort::next_unit_f64` (`(next_u64() >> 11) as f64 * 2^-53`, frozen formula). Child streams use `MamutRng::derive_child` / `RngPort::derive_child_port`.
- Evidence (if the session emits a manifest) uses core `evidence` types (`RunManifest`, `RngSpec` as plain metadata via `RngPort::metadata()`); the evidence layer never references the RNG port (core `evidence/mod.rs:15` invariant, preserved).

### A4. MSRV / no-deps architecture test implications

- MSRV stays `1.86.0` (workspace `rust-version`). The port must not use post-1.86 language/library features. The frozen source uses nothing beyond this (iterators, `Vec`, `BTreeMap`-free NSGA core — the NSGA-III slices use only `Vec`/slices; confirm at implementation time with `cargo +1.86.0 check` if a pinned toolchain is available, else `cargo check` + manual feature audit).
- A `tests/architecture.rs` (mirroring `mamut-math-core/tests/architecture.rs`) is MANDATORY in the new crate with at least:
  1. `no_third_party_dependencies`: read `crates/mamut-nsga3/Cargo.toml`, assert the only `[dependencies]` entry is `mamut-math-core` (path dependency), and assert no `[dev-dependencies]` table introduces third-party crates (parity fixtures must be hand-written expected values or `include_str!` of committed JSON — no `serde_json`, no `approx`; compare with explicit tolerance expressions).
  2. `no_third_party_types_in_public_api`: a compile-time surface test — every public function signature in the crate must be expressible with `mamut_math_core` types + std + crate-owned types. Operational form (same technique as core's `public_api_composes_with_an_app_model`): a test that drives the whole public API (reference points → sort → normalize → associate → select; SBX/mutation/var_or over caller-supplied bounds; ask/tell session over a stub evaluator) using only core types, proving no solver is invoked and no third-party type crosses the boundary.
  3. `f64_canonical`: assert the crate exposes no generic scalar parameter (spot-check that `ValidFloat`/`ObjectiveValues` are the only float carriers in public signatures).
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` must stay green (frozen CI contract per doc 04 §B4).

---

## B. Module-by-module extraction map

Target root: `crates/mamut-nsga3/src/`. Module basenames deliberately mirror the frozen source to minimize review diff. All line citations are to `6db4862d…` and were re-verified this session via `git show` (see §F) — **zero drift**.

### B0. Drift verification summary (this session)

Every doc-02 EXTRACT row was re-checked against the frozen blob: `uniform_reference_points`/`gen_refs` present with `(nobj, p)` dynamic signature and `C(nobj+p-1,p)` lattice; `dominates`/`fast_nondominated_sort` present with `[f64; 5]` binding; `NormalizeResult` + `normalize`/`find_extreme_points`/`compute_asf`/`compute_intercepts`/`fallback_intercepts` present with `[f64; 5]` binding; `perp_distance`/`associate`/`build_niche_counts`/`select_from_partial_front` present with `&[f64; 5]` + reserved `&mut dyn RngCore` param; `select_environmental` + `sel_nsga3` (5-obj/p=4/70-point) present; operators expose exactly 5 `pub(crate)` fns (`adaptive_eta_value`, `sbx_crossover`, `polynomial_mutation`, `var_or`, `init_population`) with private helpers `sbx_betaq`/`poly_mutation_delta`; `rng.rs` exposes exactly `root_rng`/`child_rng` (splitmix64 `(gen,idx)` derivation); `lib.rs` exposes exactly the 6 REQ-014 items + `#[doc(hidden)] SolverType`; `nsga3/mod.rs` re-exports `sel_nsga3`. **All EXTRACT rows still match; zero drift.**

### B1. Extraction table

| # | Frozen source file (6db4862d) | Target module | Class (doc 02) | Required adaptations to generic core contracts | MUST NOT come (doc 03 §5 must-not-extract list) |
|---|---|---|---|---|---|
| 1 | `nsga3/reference_points.rs` (166 LOC): `uniform_reference_points(nobj,p)`, `gen_refs` | `src/reference_points.rs` | **EXTRACT** (verbatim logic) | None structural: source is already `(nobj: usize, p: usize) -> Vec<Vec<f64>>` dynamic. Re-expose as public with validated errors (`MathError`: `nobj ≥ 1`, `p ≥ 1` instead of `debug_assert`). Return type stays `Vec<Vec<f64>>` (plain `f64` lattice constants, not `ValidFloat` — lattice values are exact rationals `k/p`; validation at the consumer boundary). | Nothing domain-coupled exists in this file; no cut needed. |
| 2 | `nsga3/nondom_sort.rs` (211 LOC): `dominates`, `fast_nondominated_sort` | `src/nondom_sort.rs` | **EXTRACT** (generalize signature) | **[f64;5] → dynamic-N (see B2 decision).** New shape: `fast_nondominated_sort(fitnesses: &[impl AsRef<[ValidFloat]> or &[ObjectiveValues]]) -> Vec<Vec<usize>>`; internal `dominates(p, q)` over slices with `M = p.len()` (assert uniform arity, `DimensionMismatch` on ragged input). Preserve Deb-2002 O(M·N²) loop order and insertion-order tie-break exactly (parity-sensitive). | `dominates_5` + `ParetoFront` in `optimizer.rs:40-107` do NOT move (doc 02: split-seam → REWRITE; Hydraulics rewrites against generic archive). |
| 3 | `nsga3/normalize.rs` (341 LOC): `NormalizeResult`, `normalize`, `find_extreme_points`, `compute_asf`, `compute_intercepts`, `fallback_intercepts` | `src/normalize.rs` | **EXTRACT** (generalize 5×5 Gauss elimination to M×M) | **[f64;5] → dynamic-N (see B2).** `NormalizeResult { normalized: Vec<Vec<f64>>, ideal: Vec<f64>, intercepts: Vec<f64> }`. Hand-coded Gaussian elimination with partial pivoting generalized from fixed 5 to `M` (same pivot guard `|pivot| < 1e-12`, same non-positive/`x ≤ 0`/non-finite → `fallback_intercepts` rule, same ASF weight `w = 1/1e-6`). Empty input → zero-length result with unit intercepts (same as frozen). Non-finite inputs: frozen code assumes finite; new API rejects non-finite at the boundary with `MathError` (`InvalidNotFinite`) — the only authorized semantic hardening (core `ValidFloat` invariant), recorded as such. | No domain content in file; constants `1e-6`/`1e-12` are algorithmic and travel. The `[1e12; 5]` sentinel and `1e11` gates in `optimizer.rs` MUST NOT be smuggled in as "default intercepts" (doc 03 §5 item 5). |
| 4 | `nsga3/niching.rs` (306 LOC): `perp_distance`, `associate`, `build_niche_counts`, `select_from_partial_front` | `src/niching.rs` | **EXTRACT** (generalize `&[f64;5]` to slices) | **[f64;5] → dynamic-N (see B2).** `perp_distance(p: &[f64], r: &[f64])` with `debug_assert`-turned-`MathError` on length mismatch; degenerate-ref guard `1e-30` unchanged; `associate`/`build_niche_counts` arity-agnostic. **Drop the unused `rng: &mut dyn RngCore` parameter** from `select_from_partial_front` (it is reserved/unconsumed at frozen `niching.rs:97-98,105`; `rand::RngCore` cannot cross the ARC-002 boundary anyway). Tie-break stays smallest-index deterministic. If a future ticket needs stochastic tie-breaking, it re-adds `&mut dyn RngPort` — not this ticket. | Nothing else; file has no domain imports. |
| 5 | `nsga3/selection.rs` (254 LOC): `select_environmental`, `sel_nsga3` | `src/selection.rs` | `select_environmental` **EXTRACT**; `sel_nsga3` **REWRITE-USING-LIBRARY** (Hydraulics side) | `select_environmental(fitnesses, n, ref_points, …) -> Vec<usize>` generalized to dynamic-N; association table indexed by original population index (frozen `selection.rs:86-90` logic preserved). RNG param becomes `&mut dyn RngPort` (threaded, currently unconsumed — same reserved status, now with a MAMUT-owned type). **`sel_nsga3` (5-obj/p=4/70-point fixing) is NOT ported** — Hydraulics re-expresses it as `select_environmental(five_obj_fitness, n, &uniform_reference_points(5, 4), rng)` in its compatibility layer (doc 02 row verbatim). | The `(5, 4)` lattice choice is Hydraulics policy, not library content. |
| 6 | `operators.rs` (628 LOC) SUBSET: `adaptive_eta_value`, `sbx_crossover` + private `sbx_betaq`, `polynomial_mutation` + private `poly_mutation_delta`, `var_or` | `src/operators.rs` (single module; sub-split into `sbx.rs`/`mutation.rs`/`var_or.rs` is FORBIDDEN this ticket — one module preserves the frozen review surface) | **EXTRACT** (all four + two private helpers) | Signature transplant: `&[GeneSpec]` → `&[Variable]` (core `VariableKind::Continuous` bounds; `Integer` bounds map to their `i64` interval operated as raw `f64`, rounded only by the caller's decoder — frozen `operators.rs:10-13` semantics preserved); `&mut ChaCha20Rng` → `&mut dyn RngPort` (draws via `next_unit_f64`; `gen_range`-equivalent uniform sampling expressed over the port). `Individual { genes, fitness }` → core `Candidate` (genes) + `Option<ObjectiveValues>` (fitness) — new crate-owned `Individual` struct composed of core types, or caller-managed parallel vectors; contract recommends a small crate-owned `Individual { candidate: Candidate, fitness: Option<ObjectiveValues> }` (MAMUT-owned, core-composed). Documented divergences TRAVEL: `var_or` keeps BOTH SBX children (frozen `operators.rs:190-202` vs DEAP child1-only) and `polynomial_mutation` invalidates fitness only when a gene actually changed (frozen `operators.rs:127-130` vs DEAP always-clear). `adaptive_eta_value` verbatim (pure interpolation). | `init_population` is SPLIT-SEAM (see row 7), NOT part of this subset. `GeneSpec`/`GENE_SPECS`/`SolverType` (doc 03 §5 items 1–2) MUST NOT come. |
| 7 | `operators.rs`: `init_population(solver_type, pop_size, rng)` + `encoding.rs:IndividualEncoder::random_individual` mechanics | `src/session.rs` (population-init path, §C) | **Split-seam → EXTRACT core only** (doc 02) | Library takes `init_population(variables: &[Variable], pop_size: usize, rng: &mut dyn RngPort)` with uniform-box sampling per `VariableKind` (Continuous: uniform in `[lower, upper]`; Integer: uniform over the `i64` interval as raw `f64`; Discrete/Categorical: uniform index draw — extends frozen semantics, which only knew Float/Int, to the full frozen core `VariableKind`; the extension rule must be specified in the implementation ticket and pinned by new fixtures). `SolverType`-keyed dispatch (`init_population` frozen signature takes `SolverType`) stays Hydraulics-side. | `SolverType` enum, all six `GENE_SPECS` rows (names/bounds/dtypes), `decode` table coupling, `params_to_solver_params` 11-field mapping (doc 03 §5 items 1–2). |
| 8 | `rng.rs` (35 LOC): `root_rng`, `child_rng` | **NO target module** — consume frozen core `rng` | **EXTRACT-equivalent already owned by core** (doc 02 EXTRACT satisfied by MATH-RNG-001) | NSGA session construction takes `Seed` (or an existing `MamutRng`); per-generation/per-individual streams use `MamutRng::derive_child` (keyed, state-independent). The frozen `child_rng(seed, gen, idx)` splitmix64 formula is NOT re-implemented (that would be a second RNG — forbidden). **Consequence (parity-relevant, see §D/E): the new crate's keystream differs from `rand_chacha::ChaCha20Rng::seed_from_u64`**; stochastic-op bit streams are therefore new-fixture-pinned, not cross-implementation bit-identical. | `rand`/`rand_chacha` types MUST NOT appear in any signature (ARC-002 isolation; frozen public path already took only `u64` seeds — doc 03 §4). `Arc<Mutex<S>>` sequential-evaluation discipline stays Hydraulics-side (doc 02 KEEP constraint). |
| 9 | `lib.rs` (79 LOC) + `nsga3/mod.rs` (16 LOC): 6-item REQ-014 API + `sel_nsga3` re-export | `src/lib.rs` (new crate root) | **REWRITE (visibility + ownership)** | New root declares `pub mod reference_points, nondom_sort, normalize, niching, selection, operators, session, config, error` and re-exports a MAMUT-owned public API (§C). Nothing is `pub(crate)`-locked: NSGA-III steps are first-class public API (unlike frozen `pub(crate)` + metrics `allow(dead_code)` gap, anomaly A5 — though metrics promotion itself is OUT of scope, see B3). Error type is crate-owned (e.g. `Nsga3Error` wrapping core `MathError`), never `OptimizationError` (doc 03 §5 item 7). | `GeneticOptimizer` struct + `new`/`optimize`/`evaluate_population`, `OptimizationConfig` domain flags, `ParetoResults` 5-objective selectors, `Diagnostics` re-export, `SolverType` doc-hidden export, `Solver` trait, six solvers, `SolverParams`, all fixtures/gates (doc 03 §5 items 6–9). |
| 10 | `metrics.rs`, `constraints.rs` (beyond `check_bounds` shape), `objective.rs`, `encoding.rs` table, `optimizer.rs` loop, `config.rs` domain flags, `results.rs`/`progress.rs` 5-obj bindings, `routing/` | **NO target module this ticket** | KEEP / REWRITE-USING-LIBRARY per doc 02 (metrics = REWRITE visibility, future ticket; `check_bounds` tolerance-param shape + `FeasibilityReport` combiner are future-ticket candidates) | — | Entire doc 03 §5 must-not-extract list (10 items) applies; quoted in full in B4. |

### B2. The `[f64; 5]` → generic-N decision (ARCHITECTURE DECISION, binding on the implementation ticket)

**DECISION: dynamic objective count `M` (validated at call time). No fixed-N generic parameter, no `[f64; N]` const-generic.**

- Frozen bindings to cut: `dominates(p: &[f64; 5])`, `fast_nondominated_sort(&[[f64; 5]])`, `NormalizeResult { normalized: Vec<[f64; 5]>, ideal/intercepts: [f64; 5] }`, `find_extreme_points -> [[f64; 5]; 5]`, `compute_asf(f: &[f64; 5])`, `compute_intercepts` (hand-coded 5×5 Gauss), `fallback_intercepts`, `perp_distance(p: &[f64;5], r: &[f64])`, `associate(&[[f64; 5]])`, `select_environmental(&[[f64; 5]])` (evidence: `nondom_sort.rs:16,37`; `normalize.rs:18-224`; `niching.rs:21,50`; `selection.rs:28`).
- Rationale:
  1. Core contracts are already dynamic: `ObjectiveValues` is a non-empty `Vec<ValidFloat>` (core `objective.rs:44`); `Candidate` likewise. A fixed `M` (const-generic or hardcoded 5) would force a second arity system beside core's, violating the ticket DoD ("generic Candidate/Evaluation contracts accepted").
  2. Precedent inside the frozen source: `reference_points.rs` is already dynamic over `nobj`, and `metrics.rs:30,107` (`hypervolume`, `igd_plus`) are already `&[Vec<f64>]`-arbitrary-M. Dynamic-N unifies the crate with its own most-generic modules.
  3. The Hydraulics 5-objective arity is domain policy (five named objectives, `objective.rs:132-139`), not mathematics. Freezing the library at 5 would bake Hydraulics policy into MAMUT Math, breaching AGENTS.md §4 and doc 03 §4 (`Individual<M>` row: "today hardcoded 5 … must be generic over arity M").
  4. Alternatives rejected: (a) `const M: usize` generics — rejected: infects every signature (`NormalizeResult<M>`, session, fixtures), complicates the ask/tell boundary and codec integration, with zero additional parity benefit (all frozen evidence is 5-objective; dynamic-N covers it); (b) documented fixed-5 library — rejected: fails the ticket DoD's "dynamic objective count or documented fixed-N choice" clause by choosing the non-generic branch against the characterization's explicit direction (doc 02: "generalize signature", "generalize 5×5 to M×M", "generalize `&[f64;5]` to slices").
- Uniformity rule (binding): every target module validates `M ≥ 1` and uniform arity across its inputs, returning typed `MathError` (`EmptyCollection` for `M = 0`, `DimensionMismatch` for ragged/mismatched inputs, `InvalidNotFinite` for non-finite entries). Reference-point dimensionality must equal `M` (`DimensionMismatch` otherwise — replaces frozen `debug_assert_eq!(r.len(), 5)`).
- Parity consequence: the 5-objective Hydraulics behavior is preserved as the `M = 5` instance, pinned by ported 5-objective fixtures (§D). Loop-order preservation (§E) keeps `M = 5` outputs within documented tolerance of the frozen implementation.

### B3. Explicitly OUT of scope this ticket (non-goals re-stated as module cuts)

- `metrics.rs` (`hypervolume`, `hypervolume_slices`, `igd_plus`, golden `HV=784.0 / IGD+=0.6889635872182424 @ 1e-12`): doc 02 REWRITE (visibility) — belongs to a future metrics-promotion ticket with fresh library-side oracles. NOT ported here.
- Geometry kernel (`constraints.rs:902-1097` + `objective.rs:428-509` duplicates) and `routing/` (A*/RDP/visibility/PathSmoother): EXTRACT-classified but outside the ticket Scope; future tickets.
- `check_bounds` (tolerance-parametrized) + `FeasibilityReport` combiner: future constraint-ticket candidates; this ticket consumes only core `Feasibility`/`evaluate_feasibility` at the session boundary.
- Archive/progress/stats shapes (`ParetoFront`, `GenerationStats`, `ProgressEvent`): consumed via core `evidence` + session records (§C), not ported as modules.

### B4. MUST-NOT-come list (doc 03 §5, cited verbatim by reference — each item is a review gate)

1. `SolverType` and all six `GENE_SPECS` rows — `encoding.rs:50-318`.
2. `params_to_solver_params` mapping and `base_params` merge — `optimizer.rs:512-551`; `hydro-cli/src/lib.rs:280-292`.
3. All five objective calculators, cost catalogues, constants — `objective.rs:25-421`, incl. Todini and Rc **as applied**.
4. Full geometric constraint stack, `DesignConstraints` defaults, violation taxonomy — `constraints.rs:28-900`; config zone structs — `config.rs:17-47`.
5. Penalty constants (1e12/1e9), sentinel `[1e12; 5]`, feasibility threshold 1e11 — `constraints.rs:94-95`; `optimizer.rs:217,277,285,464-490,585`.
6. Domain config flags — `config.rs:90-124`.
7. `OptimizationError::Solver/NormValidationFailure`, `Diagnostics` re-export — `errors.rs:31-34`; `lib.rs:45`.
8. `Solver` trait, six solvers, `SolverParams`, `TerrainModel` usage.
9. All oracle fixtures, ENG-022 lock, performance budgets (stay authoritative Hydraulics-side).
10. `check_solution`-dormant and empty-`PipeNetwork` shaping behaviors (A1/A2) — record, do not replicate.

Reviewer gate: any symbol from B4 appearing in the implementation diff (beyond test-stub domain doubles clearly marked as such) FAILS the ticket.

---

## C. Ask/tell-compatible session API sketch (MAMUT-owned types only)

All names are sketches — the implementation ticket finalizes identifiers, but the SHAPE below is binding: MAMUT-owned types only, no third-party leakage, `RngPort`-threaded determinism, core `StopReason` mapping.

### C1. Config (generic GA controls only — doc 02 split-seam row)

```rust
// crates/mamut-nsga3/src/config.rs (sketch)
use mamut_math_core::{ValidFloat, Variable};

pub struct Nsga3Config {
    pub population_size: usize,   // > 0  (frozen default precedent: 100, config.rs:188-219)
    pub generations: u32,         // logical budget; 0 allowed = evaluate-only run
    pub crossover_prob: f64,      // ∈ [0,1] (frozen cx 0.9)
    pub mutation_prob: f64,       // ∈ [0,1] — per-offspring probabilities for var_or
    pub eta_min: f64, eta_max: f64, // adaptive-eta schedule (> 0, eta_min ≤ eta_max)
    pub divisions: usize,         // Das-Dennis p ≥ 1 (Hydraulics precedent: p = 4)
    pub seed: Seed,               // core Seed — NOT u64 (frozen u64 seeds map via Seed::from_u64)
    pub variables: Vec<Variable>, // caller-supplied boxes/dtypes (replaces GeneSpec table)
}
// Nsga3Config::new validates: population_size ≥ 1, probs ∈ [0,1], eta > 0 ordered,
// divisions ≥ 1, variables non-empty. Errors: MathError (DimensionMismatch/EmptyCollection/InvertedBounds…).
// NOTE: cxpb + mutpb ≤ 1.0 invariant from frozen var_or travels (frozen operators.rs var_or guard).
```

What is NOT in `Nsga3Config`: spatial zones, bend/cover/slope/clearance flags, smoothing toggle, 5 weights, norm strings, time budgets (wall-clock is caller-side; see C4).

### C2. Ask/tell session (the evaluate-callback boundary)

```rust
// crates/mamut-nsga3/src/session.rs (sketch)
use mamut_math_core::{Candidate, EvaluationOutcome, ObjectiveValues, RngPort, Seed, StopReason, MathError};

pub struct Nsga3Session {
    config: Nsga3Config,
    rng: MamutRng,                       // root stream from config.seed
    population: Vec<Individual>,         // crate-owned { candidate: Candidate, fitness: Option<ObjectiveValues> }
    generation: u32,
    evaluations: u64,
}

impl Nsga3Session {
    pub fn new(config: Nsga3Config) -> Result<Self, MathError>;  // builds root MamutRng, init_population
    pub fn ask(&self) -> Vec<Candidate>;                        // unevaluated candidates needing objectives
    pub fn tell(&mut self, outcomes: Vec<EvaluationOutcome>) -> Result<SessionProgress, MathError>;
    // tell accepts Success { objectives, constraints, feasibility, diagnostics } per candidate
    // (in-documented order); Failure outcomes are recorded as typed errors — NEVER mapped to
    // sentinel fitness (core EvaluationOutcome contract: "never substitutes a silent sentinel").
    pub fn step(&mut self, evaluate: &mut dyn FnMut(&[Candidate]) -> Vec<EvaluationOutcome>)
        -> Result<SessionProgress, MathError>;                   // convenience: ask → evaluate → tell → select
    pub fn selected_indices(&self) -> Vec<usize>;               // last environmental-selection result
    pub fn stop_reason(&self) -> Option<StopReason>;             // None while running
    pub fn evaluations(&self) -> u64; pub fn generation(&self) -> u32;
}
```

- Generation loop (inside `tell`/`step` after objectives arrive): nondominated sort → front-by-front fill → normalize (definite + partial) → associate → niche counts → partial-front niching — the frozen `select_environmental` orchestration (frozen `selection.rs:28-100`), now over dynamic-N `ObjectiveValues` and caller-supplied `uniform_reference_points(M, config.divisions)`.
- Offspring production (inside `step`, before next `ask`): `adaptive_eta_value(generation, max_gen, eta_min, eta_max)` → `var_or(population, variables, lambda = population_size, cxpb, mutpb, eta, rng)` with `λ = pop` (frozen `optimizer.rs` precedent `var_or(λ=pop)`).
- `Individual` is crate-owned and MAMUT-composed: `{ candidate: Candidate, fitness: Option<ObjectiveValues> }` (`None` = unevaluated/invalidated — frozen `Individual.fitness: Option<[f64;5]>` semantics, `encoding.rs:327-332`).
- Constraints at the boundary: caller returns core `Feasibility` per candidate (computed via core `evaluate_feasibility` from `ConstraintDescriptor`s + `ConstraintViolation`s, or caller-owned). Selection admits `Feasible` + `SoftViolation` (`is_admissible`), excludes `InfeasibleHard` — the generic form of the frozen `fitness[0] < 1e11` gate WITHOUT its domain constants.

### C3. Population init consuming `&mut MamutRng` (or `RngPort`)

```rust
pub fn init_population(
    variables: &[Variable],
    pop_size: usize,
    rng: &mut dyn RngPort,          // object-safe port — caller may pass &mut MamutRng
) -> Result<Vec<Individual>, MathError>;
```

- Uniform sampling per `VariableKind` (see B1 row 7 for the Discrete/Categorical extension rule — the ONLY authorized semantic extension, pinned by new fixtures).
- Determinism: same `Seed` + same `variables` + same `pop_size` → identical population, guaranteed by the frozen core RNG contract (keyed derivation, no `thread_rng`, no `HashMap` on the stochastic path — same discipline as frozen `rng.rs`/design §12).

### C4. Stop-reason mapping to the frozen core taxonomy (binding)

| Library event | Core `StopReason` (frozen `evidence/stop.rs`) |
|---|---|
| `generation == generations` budget reached | `IterationBudgetExhausted` |
| evaluation-count budget reached (if configured) | `EvaluationBudgetExhausted` |
| caller-requested early halt (drop/`abort()` handle) | `Aborted` |
| typed internal failure (arity/validation/RNG exhaustion) | `Failed` (message-free — carries no payload per frozen stop-contract) |
| convergence criterion (only if a future ticket defines one; NOT this ticket) | `Converged` — forbidden to emit this ticket |
| wall-clock deadline | NOT mapped — wall-clock stays caller-side (the frozen `TimeBudgetExceeded(f64)`/`max_time` 300 s is Hydraulics policy; the library is wall-clock-free for determinism). If a caller needs a logical tick deadline, `LogicalDeadlineReached { ticks }` is available. |
| target achievement (caller-supplied target predicate) | `TargetAchieved` (optional; default: never emitted) |

### C5. Determinism guarantees with seeds

- Root: `MamutRng::root(&Seed)`; `Seed::from_u64(n)` reproduces the canonical default chain (`42` → frozen default seed precedent, `config.rs:199`, `hydro-cli` seed chain).
- Per-generation independence: `rng.derive_child(generation as u64)` for the offspring stream; per-individual reproducibility available via further derivation. Streams depend only on `(parent.key, index)` (frozen core header) — scheduling-independent by construction.
- Evidence: `RngSpec` recorded from `RngPort::metadata()` (`algorithm_id = "mamut.chacha20-std"`, version 1) into the caller's `RunManifest`; seed fingerprint never logs raw seed bytes (core `Seed` Debug invariant).
- What determinism does NOT promise (see §E): cross-implementation bit-identity with the frozen `rand_chacha` streams.

---

## D. Parity evidence plan

### D1. Ported expected-value fixtures (from the frozen implementation's own unit tests)

Port = re-encode the frozen test's INPUT vectors + EXPECTED outputs as committed fixtures (JSON via `include_str!`, no `serde_json` — hand-rolled parser or `const` Rust tables; the architecture test §A4 forbids the dev-dependency). Each fixture pins `M = 5` (the Hydraulics arity instance) plus at least one `M ∈ {2, 3}` case where the frozen tests already cover it (reference points). Tolerance: exact integer/count assertions bit-exact; `f64` assertions at `1e-12` absolute (matching the frozen metrics-gate precedent, doc 04) unless §E re-association analysis justifies wider per-case tolerance (recorded per fixture).

| Frozen test(s) (file @ 6db4862d) | Seed / constants | Fixture to port (expected values) |
|---|---|---|
| `reference_points.rs` tests: `test_ref_points_count_5obj_p4` (70 pts), `test_ref_points_count_2obj_p3` (4), `test_ref_points_count_3obj_p2` (6), `test_ref_points_count_5obj_p1` (5), sum-to-one, non-negativity, dimensionality | No RNG (pure combinatorics); lattice constants incl. `C(8,4)=70` | Full 70-point lattice for `(5,4)` (exact rationals `k/4`) + counts for `(2,3)/(3,2)/(5,1)` + sum-to-one @ `1e-12` (frozen test uses `1e-12`; doc 01 §9 cites `1e-9` — fixture uses the CODE value `1e-12`) |
| `nondom_sort.rs` tests: known-3-point, all-identical, strict-chain, single, empty, index-coverage (6 tests) | No RNG (deterministic ordering; insertion-order tie-break) | Input fitness matrices + expected `Vec<Vec<usize>>` fronts, BIT-EXACT (integer outputs — no tolerance). All-identical → single front preserving insertion order is the critical tie-break pin |
| `normalize.rs` tests: ideal=per-obj-min, translated-nonnegative, well-conditioned positive intercepts, known-corner intercepts, degenerate all-identical no-panic, empty input, corner-maps-to-unit (7 tests) | No RNG | Input matrices + expected `ideal`, `intercepts`, `normalized` @ `1e-12`; degenerate + empty cases pin fallback (`translated max`, zero-guard `1.0`) and empty→`ideal 0/intercepts 1` behavior BIT-EXACT in structure |
| `niching.rs` tests: aligned/orthogonal/diagonal `perp_distance`, association, niche counts, low-niche preference, distance tie-break, count increments (11 tests) | Fixture RNG `ChaCha20Rng::seed_from_u64(42)` (`niching.rs:168`) — reserved param, unconsumed: port the DISTANCE/ASSOCIATION values, not the RNG stream | `perp_distance` values + `associate` tables + `select_from_partial_front` index sequences, BIT-EXACT for indices/counts; distances @ `1e-12` |
| `selection.rs` tests: exact-n, large-n, pop≤n, index-validity, no-duplicates, lower-niche preference, determinism (10 tests) | `seed_from_u64(0)` (fixture RNG, `selection.rs:117`); determinism pair `seed_from_u64(123)×2` (`selection.rs:243-251`) | Selection index vectors BIT-EXACT for structure (counts/validity/uniqueness/lower-niche preference); determinism test becomes same-`Seed`-twice → identical selection (new-RNG stream, so expected indices are RE-PINNED, not copied — see D2) |
| `operators.rs` tests: eta endpoints/midpoint/monotonicity (5), SBX bounds/fitness/determinism (`root_rng(42/1/7)`, `run(99)` vs `run(100)` differ), mutation bounds/determinism (`root_rng(42)`, `0..200` existence sweep), varOr count/bounds/determinism/`λ=1`/pure-reproduction, init size/bounds/None/determinism/all-types (20 tests) | `root_rng(42/1/7/0/99/100)`, `0..200` sweep, Int-gene contract seeds `1..=5/99` (doc 05 §1.2) | `adaptive_eta_value` values BIT-EXACT (pure arithmetic on identical expression — still gated @ `1e-12` for safety); SBX/mutation/varOr/init outputs RE-PINNED under `MamutRng` (see D2); `var_or` λ-count/bounds/pure-reproduction STRUCTURE bit-exact; `0..200` existence sweep re-run under new RNG (sweep range may be widened ONLY if documented); Int-gene raw-float + round-at-decode contract preserved via `VariableKind::Integer` |

### D2. Differential protocol (frozen old implementation vs new crate)

Harness: a throwaway differential driver (NOT shipped; lives in the implementation worktree `target/` or a `#[cfg(test)]` helper that shells `git show 6db4862d…` vectors in as constants — it MUST NOT add a hydro-* dependency; frozen vectors are copied as literal constants with provenance comments).

1. **Deterministic integer/choice paths — BIT-EXACT required**: reference-point lattices, nondominated-sort fronts, association tables, niche counts, selection index structure (validity/uniqueness/counts), `var_or` offspring COUNT, `adaptive_eta_value`. Same logical input → identical output, no tolerance. Any mismatch = implementation bug, fix and re-run (max 1 fix + 1 re-review per constitution §5, then `HUMAN_DECISION_REQUIRED`).
2. **Deterministic `f64` paths — `1e-12` absolute tolerance, documented exceptions**: `perp_distance`, normalized objectives, intercepts, SBX/mutation offspring genes. Rationale for non-zero tolerance even with "same algorithm": Rust gives NO cross-build reassociation guarantee once loop order, iterator desugaring (`iter().zip().map().sum()` vs indexed loop), or M-generalized accumulation order differs from the frozen `[f64; 5]`-unrolled path; `debug` vs `release` float behavior is identical for IEEE ops but inlining/vectorization may reorder. The implementation ticket MUST preserve frozen accumulation order verbatim (indexed `for j in 0..M` loops mirroring frozen `for i in 0..5`) and record any deviation per-fixture with a widened, justified tolerance (never silently).
3. **Stochastic paths — DISTRIBUTIONAL + structural parity, RE-PINNED expectations**: SBX/mutation/varOr/init offspring gene VALUES and selection-tie outcomes that consumed `rand_chacha::ChaCha20Rng::seed_from_u64` streams CANNOT be bit-identical under `MamutRng` (different keystream by design — frozen core `mamut.chacha20-std` v1 vs `rand` crate ChaCha; see §E risk R1). For these: (a) assert structural invariants bit-exact (bounds, counts, fitness-invalidation rule, both-children divergence, determinism-twice-same-Seed); (b) record new expected values generated by the implementation itself as the committed fixtures (self-generated goldens, reviewed line-by-line against algorithm intent); (c) statistical parity: same-moment checks (mean/variance of offspring spread over the `0..200`-style sweep within documented bands) — exact bands set by the implementation ticket and reviewed.
4. **Deb/Jain independent review (MATH_VERIFY=ON)**: verifier checks each ported module against Deb & Jain 2014 §3 (Das-Dennis), §4.1 (normalization), §4.2 (niching), Deb 2002 (sort), Deb & Agrawal 1995 (SBX), Deb 2001 (polynomial mutation) — step-by-step, independent of the frozen code. Reviewer (L3 gate) approves the B2 dynamic-N generalization as mathematically faithful.
5. **Hydraulics compatibility consumption check**: a compile-only (or stub-evaluator) test proving a future Hydraulics layer CAN call `select_environmental(five_obj, n, &uniform_reference_points(5, 4), rng)` + `var_or(λ=pop)` + `init_population(hydro_variables, pop, rng)` — no hydro types involved, no behavior asserted beyond wiring.

### D3. What is NOT parity-gated this ticket

Metrics goldens (`HV=784.0`, `IGD+=0.6889635872182424`), solver smoke (`seed 42, 8/3, fitness[0] < 1e11`), ENG-022 lock, performance budgets — all stay Hydraulics-side gates (doc 03 §5 item 9, doc 07 §2.2). The implementation ticket runs the NEW crate's suite only, plus workspace `cargo test` to prove core untouched.

---

## E. Risk register

| ID | Risk | Severity | Mitigation in this contract | Needs HUMAN_DECISION? |
|---|---|---|---|---|
| R1 | **RNG keystream change breaks cross-impl bit parity.** Frozen stochastic ops consume `rand_chacha::ChaCha20Rng::seed_from_u64(seed)`; the library MUST consume `MamutRng` (frozen core, different expansion: `Seed::from_u64` → `chacha20_block(key,0,"MAMUT-SEED1\\0")[..32]` as key + nonce `"MAMUT-RNG-v1"` vs rand's construction). Same `seed=42` yields DIFFERENT draws. Any DoD reading that demands "same seed → same offspring as the frozen implementation" bit-exact is unsatisfiable. | HIGH | Contract redefines parity per-path (§D2): bit-exact only for RNG-independent paths; re-pinned + structural + statistical for RNG-consuming paths. Ticket canon ("seeded compact fixtures preserve expected selection/offspring behavior") is satisfied by NEW fixtures with expected behavior, not old bytes. | **YES — confirm**: parity is path-dependent (§D2), not cross-implementation bit-exact. If governance requires bit-exact offspring identity with the frozen binary, the ticket is infeasible as scoped (would require embedding `rand_chacha`, breaching ARC-002). |
| R2 | **`[f64;5]` → dynamic-N reassociation drift.** Generalizing fixed-5 unrolled/fixed-loop arithmetic to `M`-loops (esp. 5×5→M×M Gauss elimination, ASF extreme-point scan, `perp_distance` dot/norm accumulation) risks operation-reorder `f64` differences even at `M=5`. | MEDIUM | Binding loop-order preservation rule (§D2.2) + `1e-12` default tolerance with per-fixture justification for any widening + degenerate/fallback path structural pins. Independent Deb/Jain review validates the generalized math, not just the diff. | No — unless a fixture needs tolerance wider than `1e-9` (then escalate: indicates algorithmic, not rounding, divergence). |
| R3 | **Domain leakage vectors.** Highest-risk seams: (a) `Variable` bounds copied from `GENE_SPECS` oracle values as "test constants" that calcify into defaults; (b) `(5, 4)` lattice or `λ=pop`/`p=4` re-appearing as library DEFAULTS instead of caller choices; (c) 1e12/1e9/1e11-magnitude thinking leaking into fallback/intercept defaults or `Failure` handling; (d) `SolverType`-shaped enums re-introduced as "convenience"; (e) Todini/connectivity-flavored "resilience" helpers. | HIGH | B4 ten-item must-not-come list as a review gate (any occurrence beyond clearly-marked test doubles FAILS); config contains NO domain default (§C1); `sel_nsga3` NOT ported (§B1 row 5); implementation diff reviewed against B4 line-by-line. | No — unless the implementation ticket requests a domain-flavored convenience (then: refuse, Hydraulics-side only). |
| R4 | **Dropped `rng` param in niching/selection changes call arity vs frozen/DEAP.** Frozen `select_from_partial_front(..., _rng: &mut dyn RngCore)` and `select_environmental(..., rng: &mut ChaCha20Rng)` thread an RNG that is never consumed; DEAP parity "mirroring" was the stated reason. Removing it is a deliberate API break vs the frozen shape. | LOW | Contract binds removal (§B1 row 4) with rationale (ARC-002 isolation + honesty about determinism); session still threads `RngPort` where consumed. | **YES — confirm** the removal (or instruct: keep `&mut dyn RngPort` reserved param through selection for future stochastic tie-breaks). Default recommended: remove. |
| R5 | **Documented divergences vs DEAP must be preserved, not "fixed".** `var_or` both-children (frozen `operators.rs:190-202`) and fitness-invalidation-only-on-change (frozen `operators.rs:127-130`) differ from DEAP; a well-meaning implementer may "correct" them to DEAP-exact, silently changing selection/offspring behavior the fixtures are supposed to pin. | MEDIUM | Contract binds divergences as specified behavior (§B1 row 6); fixtures pin the DIVERGED behavior; reviewer checks the two code comments exist in the port. | **YES — confirm** divergences travel (recommended: yes, per doc 02 EXTRACT rows). |
| R6 | **Scope creep: metrics/geometry/routing/archive.** All are EXTRACT-classified in doc 02 but OUT of this ticket's Scope (§B3). Porting them "while at it" violates the frozen snapshot and the no-big-bang invariant. | MEDIUM | §B3 explicit out-of-scope list; workspace diff limited to `crates/mamut-nsga3/` + two-line workspace `members` edit. | No — any addition beyond §B/§C requires a new ticket snapshot. |
| R7 | **`Discrete`/`Categorical` sampling extends frozen semantics.** Frozen operators knew only Float/Int (`operators.rs:10-13`); core `VariableKind` has four variants. The uniform-index rule for Discrete/Categorical (§B1 row 7) is new math with no frozen equivalent — it cannot be "differentially" validated, only reviewed. | LOW | Rule specified in-contract; new fixtures pin it; verifier reviews against uniformity definition, not against frozen code. | No — but reviewer must explicitly accept the extension rule. |
| R8 | **Empty-`M`/ragged-input hardening changes edge behavior.** Frozen code panics-or-UB-adjacent on empty input only in `normalize` (handled: returns zeros) and assumes uniform arity elsewhere; the contract mandates typed errors. Callers relying on frozen panics see errors instead. | LOW | Only authorized semantic change besides RNG (§B1 rows 2–4); documented here, not silent. | No. |

### HUMAN_DECISION flags (consolidated — orchestrator routes before/at implementation kickoff)

1. **H1 (blocking)**: Accept path-dependent parity (§D2) given the mandatory RNG change (R1). Alternative (embed `rand_chacha` for bit-identity) breaches ARC-002 — not recommended, listed only to make the trade-off explicit.
2. **H2 (blocking)**: Confirm dynamic-N decision (§B2) vs const-generic `M` (rejected in-contract with rationale; override requires re-scoping the core-contract alignment).
3. **H3 (non-blocking)**: Confirm dropping the reserved RNG param in niching/selection (R4) vs keeping `&mut dyn RngPort` as a reserved hook.
4. **H4 (non-blocking)**: Confirm the two DEAP divergences travel as specified behavior (R5).

---

## F. Source-verification appendix (this session, read-only)

Method: `git -C "…/optimizador-hidraulico-rust" show 6db4862d…:<path>` (direct external reads are permission-denied; git-blob route used throughout). Results:

- `git rev-parse HEAD` = `6db4862d7a4035de8d494cdc62607b12b4edb103` (== NSGA-001 SHA, zero drift); `git status --short` = ONLY `M mamut.engine.json` (untouched, stays untouched).
- `reference_points.rs`: `uniform_reference_points(nobj, p)` + `gen_refs` confirmed (dynamic signature, `C(nobj+p-1,p)` lattice); tests confirm `C(8,4)=70`, `C(4,3)=4`, `C(4,2)=6`, `C(5,1)=5`, sum-to-one @ `1e-12` in code.
- `nondom_sort.rs`: `dominates(p: &[f64; 5])` + `fast_nondominated_sort(&[[f64; 5]])` confirmed; 6 tests confirmed.
- `normalize.rs`: `NormalizeResult` + `normalize`/`find_extreme_points`/`compute_asf`/`compute_intercepts`/`fallback_intercepts` confirmed; 7 tests confirmed.
- `niching.rs`: `perp_distance(&[f64;5], &[f64])` + `associate`/`build_niche_counts`/`select_from_partial_front(..., _rng: &mut dyn RngCore)` confirmed; 11 tests + `seed_from_u64(42)` fixture RNG confirmed.
- `selection.rs`: `select_environmental(&[[f64;5]], n, ref_points, &mut ChaCha20Rng)` + `sel_nsga3` (5-obj/p=4) confirmed; 10 tests + `seed_from_u64(0)`/`seed_from_u64(123)×2` confirmed.
- `operators.rs`: exactly 5 `pub(crate)` fns (`adaptive_eta_value`, `sbx_crossover`, `polynomial_mutation`, `var_or`, `init_population(SolverType, …)`) + private `sbx_betaq`/`poly_mutation_delta` confirmed; 20 tests + `root_rng(42/1/7/0/99/100)` + `0..200` sweep confirmed.
- `rng.rs`: exactly `root_rng`/`child_rng` (splitmix64 `(gen,idx)`) confirmed — superseded by frozen core RNG, no port.
- `lib.rs` + `nsga3/mod.rs`: 6-item REQ-014 API + `#[doc(hidden)] SolverType` + `pub(crate) use selection::sel_nsga3` confirmed.

**Conclusion: doc-02 EXTRACT rows match the frozen source exactly — zero drift. This contract is grounded, no HUMAN_DECISION on provenance.**

---

## G. Contract acceptance checklist (for verifier/reviewer)

- [ ] Crate name `mamut-nsga3` / path `crates/mamut-nsga3` matches frozen DAG (§A1).
- [ ] Deps ONLY `mamut-math-core` + std; architecture test specified (§A3–A4).
- [ ] Every Scope module mapped with KEEP/EXTRACT/ADAPT + adaptation + must-not-come (§B1).
- [ ] Dynamic-N decision recorded with rejected alternatives (§B2).
- [ ] Ask/tell sketch uses MAMUT-owned types only; stop-reasons map to frozen taxonomy; wall-clock excluded (§C).
- [ ] Parity plan names exact frozen tests/seeds/tolerances + differential protocol + Deb/Jain review (§D).
- [ ] Risk register + H1–H4 flags (§E). H1/H2 block implementation kickoff until resolved.
- [ ] Zero-drift verification evidenced (§F).

*End of EXTRACTION CONTRACT — MATH-NSGA-002. No implementation code in this document.*
