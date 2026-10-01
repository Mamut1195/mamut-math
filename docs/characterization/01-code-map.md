# 01 — Exact code map: `hydro-optimizer` crate @ frozen SHA

- **Repo**: `C:\Users\Jonnathan\Mamut system\mamut-harness\motores\optimizador-hidraulico-rust`, branch `main`
- **Frozen SHA**: `6db4862d7a4035de8d494cdc62607b12b4edb103` (VERIFIED via `git rev-parse HEAD`)
- **Working-tree state**: one dirty tracked file `mamut.engine.json` (VERIFIED via `git status --short`); it is a manifest, not a source file. All `*.rs` evidence below matches HEAD.
- **Method**: every path below confirmed via `git ls-files`; every symbol confirmed by reading the working-tree source (which matches HEAD for all source files). Claims marked **VERIFIED**.
- **Crate**: `hydro-optimizer` (`hydro-optimizer/Cargo.toml`, VERIFIED). Deps: `hydro-types`, `hydro-hydraulics`, `hydro-terrain`, `hydro-norms`, `hydro-solvers` (all path deps), `thiserror`, `serde`, `serde_json`, `rand`, `rand_chacha`, `geo`. NOTE (VERIFIED, `hydro-optimizer/Cargo.toml` comments): `rayon` is deliberately absent — deferred to Phase 7 solver-pool pattern (REQ-017).
- **File census**: `git ls-files "hydro-optimizer/src/*.rs" "hydro-optimizer/src/**/*.rs"` returns **23 files** (not ~25; the ticket's parenthetical enumeration lists exactly these 23 — no file is missing). Total **8098 physical lines** including inline `#[cfg(test)]` suites. All 23 mapped below: **23/23**.

Public-API discipline (VERIFIED, `lib.rs:8-16,39-62`): exactly 6 public items per REQ-014
(`GeneticOptimizer`, `OptimizationConfig`, `ParetoResults`, `ProgressEvent`, `OptimizationError`, `Diagnostics`
re-exported from `hydro_types::response`). Every module is `pub(crate)` (`lib.rs:23-37`). One `#[doc(hidden)]`
re-export of `encoding::SolverType` exists solely so integration tests can call `GeneticOptimizer::new`
(`lib.rs:53-62`) — anomaly A4, see §3.

---

## 1. `hydro-optimizer/src/lib.rs` — 79 LOC — VERIFIED

Crate root. Module declarations (`pub(crate)`, lines 23–37): `config, constraints, diagnostics, encoding,
errors, metrics, nsga3, objective, operators, optimizer, progress, results, rng, routing`.
Public re-exports (lines 41–51): `OptimizationConfig`, `OptimizationError`, `Diagnostics`
(`hydro_types::response`), `GeneticOptimizer`, `ProgressEvent`, `ParetoResults`.
`#[doc(hidden)] pub use encoding::SolverType` (lines 61–62). `#[cfg(test)] mod tests` with one
compile-smoke test (lines 73–79).

## 2. `hydro-optimizer/src/config.rs` — 576 LOC — VERIFIED

GA hyperparameters + constraint flags; port of Python `genetic.py` config + design §2 field list.

| Symbol | Lines | Visibility | Math content |
|---|---|---|---|
| `ForbiddenZone { vertices, label }` | 17–23 | pub (via `pub(crate)` module; struct fields `pub`) | Polygon region type (geometry container, domain-populated) |
| `MandatoryRoute { waypoints, corridor_width, label }` | 27–35 | pub | Corridor type (geometry container, domain-populated) |
| `ExistingNetwork { segments, clearance_distance, label }` | 39–47 | pub | Clearance descriptor (geometry container) |
| `ProgressCallback` (type alias `Box<dyn Fn(&ProgressEvent)+Send+Sync>`) | 55 | pub | Callback type, not serializable (`#[serde(skip)]`, lines 5, 128–129) |
| `OptimizationConfig` (30+ fields: pop/gen/cxpb/mutpb/max_time/adaptive_eta/eta_min/eta_max/seed/num_workers/spatial vecs/bend/cover/slope/clearance/smoothing/5 weights/norm_profile/strict/callback) | 67–130 | pub (`pub use` in lib.rs) | Hyperparameter container; defaults = oracle values (pop 100, gen 50, cx 0.9, mut 0.1, 300 s, seed 42 — lines 188–219) |
| `set_spatial_constraints` | 224–244 | pub | Converts raw `Vec<Vec<[f64;2]>>` into typed zones (called by hydro-cli, `hydro-cli/src/lib.rs:270`) |
| `validate` | 250–304 | pub | Range checks (pop/gen>0, probs∈[0,1], time>0, eta>0 & ordered, weights≥0); called by `GeneticOptimizer::new` (`optimizer.rs:150`) |
| `impl Debug / Clone` (callback mapped to `"<callback>"` / `None`) | 132–186 | — | Non-cloneable callback handling |
| inline tests (REQ-002/008/013/015) | 312–576 | cfg(test) | 20 tests: defaults, serde round-trip, 9 validation rejections, RNG reproducibility, error display |

## 3. `hydro-optimizer/src/constraints.rs` — 1536 LOC — VERIFIED (lines 1–1351 read verbatim; 1352–1536 are further inline tests of the same symbols)

Constraint checker (REQ-004); port of oracle `constraints.py`. Spec penalty formula (lines 5–6, 94–95):
`penalty = Σᵢ (1e12 + magnitudeᵢ × 1e9)`. Determinism rule: no `HashMap` (line 12; one `BTreeMap` at line 677).

| Symbol | Lines | Visibility | Math content |
|---|---|---|---|
| `ConstraintViolationKind` (Bounds, ForbiddenZone, MandatoryRoute, BendAngle, CoverDepth, MaxDepth, ExistingClearance, Slope) | 28–45 | pub-in-`pub(crate)` | Violation taxonomy |
| `ConstraintViolation { kind, magnitude, description }` | 54–61 | pub-in-`pub(crate)` | magnitude = scalar distance to feasible region, ≥ 0 |
| `ConstraintReport { feasible, violations, penalty }` + `feasible()` / `infeasible()` constructors | 74–102 | pub-in-`pub(crate)` | Feasibility report; penalty 0.0 iff feasible |
| `ExistingNetworkEntry { coords, depth, setback }` | 110–117 | pub-in-`pub(crate)` | Clearance descriptor (default depth 1.5 m) |
| `DesignConstraints { min_cover 0.9, max_depth 6.0, min_slope 0.003, max_slope 0.15 }` + `Default` | 125–145 | pub-in-`pub(crate)` | Domain design limits (defaults mirror `hydro_engine.core.constraints`) |
| `ConstraintChecker` (fields: lower/upper bounds, forbidden/mandatory polys, max_bend_angle, design+3 enforce flags, existing nets + separations 0.3/1.0 m, slope flag) | 162–199 | pub-in-`pub(crate)` | Layered checker |
| 8 constructors: `new_bounds_only` 213, `new_no_bounds` 232, `new_geometric` 251, `new_with_cover` 287, `new_with_clearance` 311, `new_with_slopes` 335, `new_full` 359, canonical `new` 402 (+`new_raw` 450) | 209–480 | pub-in-`pub(crate)` | Only canonical `new` is used in production (`optimizer.rs:159-177`) |
| `check_bounds` (tolerance 0.001, raw-distance magnitude — lines 488–530) | 488–530 | pub-in-`pub(crate)` | Box-constraint check; **the only constraint call wired into `optimize()`** (`optimizer.rs:463`) |
| `check_route` | 535–559 | pub-in-`pub(crate)` | Polyline-vs-polygon feasibility |
| `check_solution` (forbidden + mandatory + bend + cover + slope + clearance) | 566–654 | pub-in-`pub(crate)` | Full geometric check — **NOT called by the main loop** (anomaly A1) |
| `check_full` (bounds gate → geometric) | 660–666 | pub-in-`pub(crate)` | Oracle-DEAP short-circuit; also unwired in production |
| inner checks: `check_bend_angles_inner` 671, `check_cover_depth_inner` 724, `check_slopes_inner` 795 (adverse ≤0 slopes skipped — pumping objective owns them, line 812), `check_existing_clearance_inner` 839 | 668–900 | private | Geometric violation detectors over `PipeNetwork` |
| geometry helpers: `solution_pipe_lines` 910, `polylines_intersect` 933, `polyline_intersects_polygon` 948, `seg_intersect` (cross-product, 1e-12 parallel guard) 973, `point_in_polygon` (ray casting) 990, `polygon_area` (shoelace) 1007, `polyline_min_distance` 1019, `polyline_distance` 1033 (`pub(crate)`), `polyline_within_buffer` 1038, `pt_to_seg_distance` 1055, `seg_to_seg_distance` 1072, `angle_between` (deg) 1088 | 902–1097 | private (except noted) | Deterministic 2-D computational geometry, zero domain content |
| inline tests | 1103–1536 | cfg(test) | Penalty-formula, bounds, zones, routes, bend, cover (to line 1351 read; remainder continues same suites) |

## 4. `hydro-optimizer/src/diagnostics.rs` — 94 LOC — VERIFIED

`OptimizerDiagnostics` (lines 26–43, `pub(crate)`): counters `pareto_candidates`, `skipped_infeasible`,
`discarded_by_norm`, `norm_violations` (always 0 — norm validation deferred to Phase 7, lines 33–34),
`conversion_errors` (always 0 — decode errors swallowed in `evaluate_population`, lines 38–39). Rationale for
the parallel schema documented lines 1–14: public `hydro_types::response::Diagnostics` keeps legacy HTTP field
names and cannot change. Tests lines 47–94.

## 5. `hydro-optimizer/src/encoding.rs` — 680 LOC — VERIFIED

Gene encoding; port of oracle `encoding.py`. `BTreeMap`, never `HashMap` (line 10).

| Symbol | Lines | Visibility | Math content |
|---|---|---|---|
| `GeneType::{Float, Int}` | 24–27 | pub-in-`pub(crate)` | Dtype tag |
| `GeneSpec { name, lower_bound, upper_bound, dtype }` | 35–44 | pub-in-`pub(crate)` | Per-gene box + dtype |
| `SolverType::{Sewer, WaterSupply, Conveyance, Distribution, PumpStation, Intake}` + `From→&str`, `Display`, `TryFrom<&str>` | 50–95 | pub-in-`pub(crate)` (+`#[doc(hidden)]` root re-export) | 6 hydraulic project types — domain enum |
| `GENE_SPECS: OnceLock<BTreeMap<&str, Vec<GeneSpec>>>` + `gene_specs()` initializer | 103–318 | pub-in-`pub(crate)` | Static registry; sewer 5 / water_supply 5 / conveyance 4 / distribution 5 / pump_station 4 / intake 4 genes with oracle-exact names+bounds (lines 112–314) |
| `Individual { genes: Vec<f64>, fitness: Option<[f64;5]> }` | 327–332 | pub-in-`pub(crate)` | Chromosome + cached fitness (`None` = unevaluated/invalidated) |
| `IndividualEncoder::random_individual` (uniform `gen_range(lo..=hi)` per gene) | 344–357 | pub-in-`pub(crate)` | Uniform box sampler |
| `IndividualEncoder::decode` (clamp + `round()` for Int; length-mismatch `Err`) | 366–395 | pub-in-`pub(crate)` | Decoder: mechanics generic, table domain |
| inline tests (REQ-001) | 402–680 | cfg(test) | Table completeness, bounds for all 6 types, decode round/clamp/reject, 6 round-trips |

## 6. `hydro-optimizer/src/errors.rs` — 42 LOC — VERIFIED

`OptimizationError` (lines 14–42, public via lib.rs): `InvalidConfig(String)`, `AllInfeasible`,
`EvaluatorFailure(String)`, `NormValidationFailure(String)`, `Solver(#[from] hydro_solvers::SolverError)`,
`TimeBudgetExceeded(f64)`, `Internal(String)`. Design §10 (lines 11–12): most evaluation failures become
`[1e12; 5]` penalty fitness; errors are for setup/non-recoverable failures.

## 7. `hydro-optimizer/src/metrics.rs` — 277 LOC — VERIFIED

Quality metrics (REQ-016 infrastructure). Both functions `pub(crate)` + `#[allow(dead_code)]`
(lines 17, 93) — **not part of the public API** (anomaly A5).

| Symbol | Lines | Math content |
|---|---|---|
| `hypervolume(front, reference_point)` | 30–51 | Exact dominated HV, arbitrary M, minimization; filters non-finite / wrong-dim / outside-box points; empty/non-finite → 0.0 |
| `hypervolume_slices` (recursive slice, dedup levels, 1-D base case) | 53–90 | N-dimensional slicing algorithm (added to close the 2-D-only gap — see parity docs) |
| `igd_plus(front, reference_front)` | 107–137 | Ishibuchi-2015 IGD+: `(1/\|R\|)·Σ min d+`, `d+_j = max(0, a_j − r_j)`; empty → ∞ |
| fixture test `test_five_objective_metrics_match_python_oracle` | 257–276 | Consumes `tests/oracle/fixtures/optimizer_quality_metrics_golden.json` at `1e-12` absolute tolerance |

## 8. `hydro-optimizer/src/nsga3/mod.rs` — 16 LOC — VERIFIED

Module declarations + `pub(crate) use selection::sel_nsga3` (line 16). Cited by ENG-022 lock test
(`eng022_no_epa_inner_loop.rs:134-140` asserts `sel_nsga3` is still exported).

## 9. `hydro-optimizer/src/nsga3/reference_points.rs` — 166 LOC — VERIFIED

`uniform_reference_points(nobj, p)` (lines 23–29, `pub(crate)`): Das–Dennis lattice, `C(nobj+p-1,p)` points,
each summing to 1.0. `gen_refs` recursive composition enumerator (lines 32–51). Tests (lines 53–166):
counts C(8,4)=70, C(4,3)=4, C(4,2)=6, C(5,1)=5; sum-to-one 1e-9; non-negativity; dimensionality. Zero domain content.

## 10. `hydro-optimizer/src/nsga3/nondom_sort.rs` — 211 LOC — VERIFIED

`dominates(p, q)` (lines 16–27, private): strict Pareto dominance, minimization.
`fast_nondominated_sort(fitnesses: &[[f64; 5]])` (lines 37–84, `pub(crate)`): Deb-2002 O(M·N²), returns
`Vec<Vec<usize>>` fronts; equal objectives non-dominating (insertion-order tie-break, line 36). Tests
(lines 86–211): 3-point known case, all-identical, strict chain, single, empty, index coverage.
NOTE: signature hardcodes `[f64; 5]` — the algorithm is arity-generic, the binding is not (seam cut, doc 02).

## 11. `hydro-optimizer/src/nsga3/normalize.rs` — 341 LOC — VERIFIED

Deb & Jain 2014 §4.1 (lines 1–11): ideal-point translation → ASF extreme points → 5×5 hyperplane solve →
fallback → divide. `NormalizeResult { normalized, ideal, intercepts }` (lines 18–26). `normalize` (38–101).
`find_extreme_points` (106–121). `compute_asf` with `w=1 / 1e-6` (128–136). `compute_intercepts` — hand-coded
5×5 Gaussian elimination with partial pivoting, degenerate (`|pivot|<1e-12`) or non-positive/`x≤0`/non-finite
→ `fallback_intercepts` = per-objective translated max, zero-guarded to 1.0 (142–224). Tests (226–341):
ideal=per-obj-min, non-negativity, known corner intercepts, degenerate no-panic, empty input, unit mapping.
Zero domain content; hardcoded 5-arity is the seam cut.

## 12. `hydro-optimizer/src/nsga3/niching.rs` — 306 LOC — VERIFIED

Deb & Jain 2014 §4.2 (lines 1–12). `perp_distance(p: &[f64;5], r: &[f64])` (21–39):
`√(‖p‖² − (p·r)²/‖r‖²)`, degenerate-ref guard 1e-30. `associate` (50–65): argmin perp distance.
`build_niche_counts` (76–87). `select_from_partial_front` (109–158): min-niche-count ref → min-distance
individual → smallest-index tie-break; `rng: &mut dyn RngCore` reserved/unconsumed (lines 97–98, 105).
Tests (160–306): aligned/orthogonal/diagonal distances, association, niche counts, low-niche preference,
distance tie-break, count increments. Hardcoded 5-arity in `perp_distance` signature is the seam cut.

## 13. `hydro-optimizer/src/nsga3/selection.rs` — 254 LOC — VERIFIED

`select_environmental(fitnesses, n, ref_points, rng)` (lines 28–100, `pub(crate)`): nondom sort →
front-by-front fill → normalize definite+partial → associate (table indexed by original population index,
lines 86–90) → niche counts → partial-front niching. `sel_nsga3` (105–108): fixes 5 objectives / p=4 /
70 Das–Dennis points. Tests (110–254): exact-n, large-n, pop≤n, index validity, no-duplicates,
lower-niche preference, determinism.

## 14. `hydro-optimizer/src/objective.rs` — 817 LOC — VERIFIED

5-objective function (REQ-003); port of oracle `objective.py`. ALL MINIMIZED (lines 5–10):
`[cost_usd, excavation_m3, pumping_score, interference_count, resilience_deficit]`.

| Symbol | Lines | Math content (domain constants — DO NOT EXTRACT) |
|---|---|---|
| `pipe_cost_catalogue` (16-entry diameter→USD/m table, oracle-exact) + `pipe_unit_cost` (nearest-diameter) | 25–74 | Cost catalogue, fallback 0.3 m = 28.0 |
| `MANHOLE_COST 2500, EXCAVATION_COST_M3 12, TRENCH_WIDTH 0.8, PUMP_STATION_COST 50000, MIN_H_SEP 1.0, MIN_V_SEP 0.3, H_MIN 15.0 mca` | 76–86 | Hydraulic cost/separation policy |
| `ExistingNetworkSpec { coords, depth (default 1.5) }` | 94–99 | Interference input descriptor |
| `ObjectiveFunction { existing }` + `new` (drops nets with <2 pts) | 107–126 | Evaluator struct |
| `score → [f64; 5]` | 132–139 | Fan-out |
| `calc_cost` (unit-cost × effective_length + manholes + excavation) | 143–153 | Construction cost |
| `calc_excavation` (avg cover × effective_length × 0.8) | 157–171 | Earthworks |
| `calc_pumping` (CAPEX pump_count×50000 + lifecycle OPEX; adverse-slope implicit pumps; `bypassed_by_pump`/`pump_lift` metadata; BTreeSet dedup of explicit PUMP nodes) | 175–238 | Pumping energy economics |
| `calc_interference` (crossing→vertical-sep / parallel→horizontal-sep counting) | 242–287 | Utility-conflict count |
| `calc_resilience` (pressure metadata present → Todini else connectivity) | 291–304 | Resilience switch |
| `todini_index` (`pub(crate)`, lines 313–378): demand-weighted surplus over H_MIN ÷ (source-head input − demand-head), clamped [0,1] | 307–378 | Todini-2000 pressurized-network index |
| `connectivity_redundancy` (`pub(crate)`, `(E−N+1)/(2N−5)`, N<3→0) | 385–396 | Gravity-network redundancy |
| `lifecycle_opex_usd` (ρ=1000, g=9.81, η=0.7, 4000 h/yr, 0.10 USD/kWh, 20 yr) | 406–421 | Pump lifecycle economics |
| geometry helpers (cross-product `seg_intersect` 444, `polyline_min_distance` 463, `pt_to_seg_distance` 477, `seg_to_seg_distance` 498 — duplicates of constraints.rs geometry) | 423–509 | 2-D geometry (duplicated, not shared — anomaly A3) |
| inline tests (REQ-003) | 515–817 | 12 tests incl. oracle-formula checks (cost 2800+5000+1440; excavation 1.5·0.8·100; Todini 0.05/0.17) |

## 15. `hydro-optimizer/src/operators.rs` — 628 LOC — VERIFIED

SBX / polynomial mutation / varOr / adaptive eta. DEAP-port header (lines 1–14) with two documented
divergences: `var_or` keeps BOTH SBX children (DEAP keeps child1 only, lines 190–202); `polynomial_mutation`
invalidates fitness only when a gene actually changed (DEAP always clears, lines 127–130).

| Symbol | Lines | Math content |
|---|---|---|
| `adaptive_eta_value(gen, max_gen, eta_min, eta_max)` — linear schedule | 34–37 | Pure deterministic interpolation |
| `sbx_crossover` (bounded, Deb & Agrawal 1995; identical-gene skip 1e-14; clamp; fitness=None) | 48–107 | SBX |
| `sbx_betaq` (DEAP exact formula) | 113–119 | Spread factor |
| `polynomial_mutation` (`indpb = 1/num_genes`; degenerate-gene skip 1e-14) | 131–169 | Bounded polynomial mutation (Deb 2001) |
| `poly_mutation_delta` (DEAP inner formula) | 176–186 | Normalized delta |
| `var_or` (μ+λ; cxpb/mutpb/reproduction; exactly `lambda_` offspring) | 207–257 | Offspring production |
| `init_population` (via `random_individual`; fitness None) | 268–276 | Population init |
| inline tests | 280–628 | 20 tests: eta endpoints/midpoint/monotonicity, SBX bounds/fitness/determinism, mutation bounds/determinism, varOr count/bounds/determinism/λ=1/pure-reproduction, init size/bounds/None/determinism/all-types, Int-gene contract |

All take `&mut ChaCha20Rng` (REQ-008/015); integer genes operated as raw floats, rounded only at decode
(lines 10–13). Zero domain content (operates on `GeneSpec` bounds — generic box data).

## 16. `hydro-optimizer/src/optimizer.rs` — 712 LOC — VERIFIED

NSGA-III main loop (REQ-009/015/016/017). Generic `S: Solver + Send + Sync`, solver behind `Arc<Mutex<S>>`
(lines 111–127); sequential evaluation because `Solver::solve(&mut self)` (lines 8–10, 118–119, 429–432).

| Symbol | Lines | Content |
|---|---|---|
| `ParetoFront { individuals, fitnesses }` + `new`/`update` (dominance-eviction, insertion-order ties)/`len`/`into_pairs` | 40–92 | Incremental Pareto archive (REQ-010) — generic |
| `dominates_5` | 96–107 | Fixed-arity dominance (duplicates `nondom_sort::dominates` — anomaly A3) |
| `GeneticOptimizer::new(solver, solver_type, config, base_params)` — validates config, builds bounds vectors from `gene_specs`, wires `ConstraintChecker::new` (empty existing nets, `None` design constraints, separations 0.3/1.0, bend iff >0) and `ObjectiveFunction::new(vec![], None)` | 144–187 | Construction; `base_params` merge is a documented PR-8f deviation from Design §2 (lines 136–141) |
| `optimize` — init → evaluate → AllInfeasible gate (`fitness[0]<1e11`, lines 215–221) → per-gen: wall-clock break, adaptive eta, `var_or(λ=pop)`, evaluate combined, `sel_nsga3`, archive update (feasible→pareto/discarded counters), stats + progress callback → sort by cost → Solution shaping → Diagnostics | 192–425 | Main loop |
| `evaluate_population` — bounds-check gate (`penalty = 1e12 + report.penalty`, line 465) → decode (`[1e12;5]` on error) → `params_to_solver_params` → `solver_guard.solve` (mutex; solver error → `[1e12;5]`) → `objective.score(first)` / `[1e12;5]` if empty | 433–506 | Fitness pipeline; ENG-022 asserts `solver_guard.solve` here, no EPA (`eng022` test lines 142–155) |
| `params_to_solver_params` (`pub(crate)`, 11 gene→param mappings incl. casts `as u32/i32`) | 512–551 | Domain gene→solver wiring |
| `compute_gen_stats` (min/avg per obj, feasibility rate, pareto size) | 554–606 | Generic monitoring |
| PathSmoother hookup (0.5 m clearance / 0.5 m RDP; gene[0]/gene[last] as x-offsets placeholder — lines 326–370) | 321–370 | Minimal Phase-7 wiring; coordinate derivation is a placeholder |
| Pareto→Solution shaping: EMPTY `PipeNetwork::new("pareto", vec![], vec![])` (line 374), `total_cost/total_excavation` mapped, full fitness in `metadata.fitness`, genes as JSON in `metadata.genetic_params` | 346–388 | Lossy shaping — network geometry is NOT preserved (anomaly A2) |
| inline tests (REQ-010) | 608–712 | 9 ParetoFront/dominance tests |

## 17. `hydro-optimizer/src/progress.rs` — 22 LOC — VERIFIED

`ProgressEvent { generation, evaluations, min_fitness: [f64;5], avg_fitness: [f64;5], elapsed_seconds }`
(lines 10–22, public). Emitted per generation via config callback (`optimizer.rs:296–305`).

## 18. `hydro-optimizer/src/results.rs` — 246 LOC — VERIFIED

`GenerationStats { generation, min_fitness, avg_fitness, feasibility_rate, pareto_size, elapsed_seconds }`
(lines 23–36, `pub(crate)` struct with `pub` fields). `ParetoResults` (lines 43–59, public):
`solutions`, `convergence`, `diagnostics` (hydro-types), `optimizer_diagnostics` (`pub(crate)`),
`elapsed_seconds`, `config_snapshot`. Accessors: `optimizer_diagnostics()` (pub(crate), 72),
`best_by_cost` (77), `best_by_excavation` (81), `best_by_resilience` via `fitness[4]` (96),
`best_balanced(weights)` max-normalized weighted sum (109–132), `comparison_table` (137–142),
`summary` (145–156). `weighted_score` (159–174). Tests 177–246.

## 19. `hydro-optimizer/src/rng.rs` — 35 LOC — VERIFIED

`root_rng(seed)` = `ChaCha20Rng::seed_from_u64` (lines 15–17). `child_rng(root_seed, generation,
individual_idx)` (lines 25–35): `s = seed·0x9E3779B97F4A7C15 + (gen<<32) + idx`, then 3-round splitmix64
(`0xBF58476D1CE4E5B9`, `0x94D049BB133111EB`), `seed_from_u64(s)`. Design §5 exact formula. Zero domain content.

## 20–23. `routing/` — 1060 LOC total — VERIFIED

Opt-in PathSmoother (REQ-012), instantiated only when `enable_path_smoothing` (`optimizer.rs:326`).
Design §8 (mod.rs:6–12): clearance-check instead of geometric buffer (geo 0.28 lacks Shapely mitre-join buffer).

| File | LOC | Symbols (all `pub(crate)`) |
|---|---|---|
| `routing/mod.rs` (268) | 268 | `PathSmoother { clearance, rdp_epsilon }` (46–51); `new` (59); `smooth` — visibility graph → adjacency → A* (0→1) → RDP; disconnected → direct-segment fallback (76–111); `path_length` (115–121); tests incl. oracle parity `test_path_length_matches_python_oracle_within_two_percent` vs `path_smoothing_golden.json` @ 2% rel. tol. (186–210) |
| `routing/astar.rs` (171) | 171 | `astar(nodes, adj, start, end)` (16–97): Euclidean admissible heuristic, `BTreeMap` g-scores (no HashMap), open set as `(f.to_bits, idx)` sort (non-negative-float bit ordering documented 56–61), stale-entry eviction; 5 tests |
| `routing/rdp.rs` (181) | 181 | `rdp_simplify` (16–63); `perp_distance` point-to-line, degenerate→0.0 (68–79); 10 tests |
| `routing/visibility.rs` (440) | 440 | `VNode { pos }` (20–22, indices 0/1 = start/end); `VEdge` (25); `build_visibility_graph` — clearance-offset (miter, capped 5×) candidate vertices (42–91); `signed_area` 94; `offset_vertex` 110; `unit` 142; `outward_normal` 151; `is_edge_visible` 162; `segments_intersect` (proper-straddle + shared-endpoint-tolerant, 190–220); `cross` 223; `on_segment` 230; `segment_too_close` 238; `segment_segment_min_dist` 255; `point_to_segment_dist` 269; `euclidean` 283; 10 tests incl. vertex-offset regression (401–416) |

---

## 2. Seam contacts OUTSIDE `hydro-optimizer/src/` — VERIFIED

**`hydro-types` contracts consumed** (grep `hydro_types::` over `hydro-optimizer/src`):
- `network::PipeNetwork` (+ `get_node`, `node_count`, `pipe_count`) — `constraints.rs:21`, `objective.rs:18`, `optimizer.rs:374`
- `response::Solution` — `constraints.rs:22`, `objective.rs:19`, `optimizer.rs:16`, `results.rs:9`
- `response::Diagnostics` (public re-export `lib.rs:45`; legacy HTTP field names per `diagnostics.rs:7-9`) + `SolutionScore`, `SolutionMetadata`, `ConvergenceRecord` — `optimizer.rs:16-18`, `results.rs:9`
- `enums::NodeType` (PUMP-node dedup `objective.rs:187-192`; pressure branching `objective.rs:293-304`), `PipeMaterial` (tests only)
- Request-side types (`request::DesignRequest`, `PolygonConstraint`, …) are consumed one layer up, in `hydro-cli`, not in the optimizer.

**`hydro-solvers` seam**: `Solver` trait + `SolverParams` (`optimizer.rs:15`; trait `solve(&mut self)->Vec<Solution>` /
`evaluate(&self)->SolutionScore`, `hydro-solvers/src/solver.rs:18-103`); `SolverError` via `#[from]`
(`errors.rs:33`). Six concrete solvers exercised through the GA in `hydro-optimizer/tests/solver_wiring_smoke.rs`
(pop 8 / gen 3 / seed 42; feasibility = `fitness[0] < 1e11`).

**`hydro-cli` call sites** (`hydro-cli/src/lib.rs`): imports `GeneticOptimizer, OptimizationConfig,
OptimizationError, SolverType` (line 17); `build_optimization_config` maps DesignRequest→config incl. seed
resolution `seed_override.unwrap_or(req.seed.unwrap_or(42))` + spatial constraints via
`set_spatial_constraints` (lines 225–273); `build_solver_params` (lines 280–292); `run_optimizer`
constructs + runs + shapes to `DesignResult` (lines 337–351); six dispatch arms map each `SolverType`
(lines 518–747). `hydro-cli/src/main.rs:30,178` explicitly instantiates NO optimizer (validate-only path).

**`mamut-adapter`**: NO direct `GeneticOptimizer` use (VERIFIED — grep for `GeneticOptimizer|optimiz` in
`server.py`/`mcp_translate.py` shows only CLI-binary invocation language; `server.py:3` declares the optimizer
is a CLI binary fed DesignRequest JSON over stdin). The adapter boundary is process-level, not API-level.

**Negative seam (VERIFIED — grep returns zero hits in `hydro-optimizer/src`)**: `hydro_norms`, `hydro_hydraulics`,
`hydro_terrain` are never imported. Norm compliance travels as opaque `norm_profile: Option<String>` +
`strict_norm_compliance` config strings with enforcement deferred (see anomaly A1).

**ENG-022 executable lock** (`hydro-optimizer/tests/eng022_no_epa_inner_loop.rs`, 215 lines): forbids
`Command`/process/FFI/EPANET/SWMM tokens in `hydro-optimizer`+`hydro-solvers`+`hydro-hydraulics` sources and
`epanet|swmm|wntr|pyswmm` in their Cargo.tomls; pins `nsga3/mod.rs`→`sel_nsga3`, `optimizer.rs`→
`evaluate_population`+`sel_nsga3`+`solver_guard.solve`, all six solver files, and the
`benchmarks/performance_budgets.json` inner-loop lock. Any extraction must keep this gate green.

---

## 3. Anomalies (with evidence)

- **A1 — Geometric/norm enforcement unwired in production.** `check_solution` (`constraints.rs:566`) and
  `check_full` (`constraints.rs:660`) are never called by `optimize()`; only `check_bounds` is
  (`optimizer.rs:463`). `GeneticOptimizer::new` passes `None` design constraints and empty existing networks
  to both checker and objective (`optimizer.rs:159-178`); `norm_violations`/`conversion_errors` are hard 0
  (`diagnostics.rs:33-43`). The geometric machinery is tested but dormant — extraction must not mistake
  tested code for executed code.
- **A2 — Pareto Solutions are lossy placeholders.** Final `Solution.network` is always empty
  (`optimizer.rs:374`); only cost/excavation map to `SolutionScore`, full fitness survives in
  `metadata.fitness`, genes in `metadata.genetic_params`. Downstream consumers depend on this shape
  (`hydro-cli/src/lib.rs:359+`).
- **A3 — Duplicated logic.** 5-arity dominance exists twice (`nondom_sort.rs:16` vs `optimizer.rs:96`);
  segment-intersection/distance geometry exists twice (`constraints.rs:932+` vs `objective.rs:428+`).
  Unification belongs to a future refactor ticket, not this characterization.
- **A4 — Doc-hidden 7th export.** `SolverType` is `#[doc(hidden)] pub` (`lib.rs:61-62`) outside the REQ-014
  six-item contract, required by integration-test visibility. A future library API must decide its fate
  (generic project-type parameter vs removal).
- **A5 — Metrics outside the public API.** `hypervolume`/`igd_plus` are `pub(crate)` + `#[allow(dead_code)]`
  (`metrics.rs:17,93`); reachable only via tests. No production caller exists — the "known HV/IGD+ gap" is
  partly a visibility gap (detailed in `03-seam-notes.md`).
- **A6 — Census note.** `git ls-files` yields 23 `hydro-optimizer/src` files; "~25" in the ticket snapshot is
  approximate. The snapshot's own file enumeration covers exactly these 23 — coverage is complete, nothing is
  unmapped.
