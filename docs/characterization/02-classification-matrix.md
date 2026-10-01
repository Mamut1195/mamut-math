# 02 — Classification matrix: KEEP-IN-HYDRAULICS vs EXTRACT vs REWRITE-USING-LIBRARY

Frozen SHA `6db4862d7a4035de8d494cdc62607b12b4edb103`. Every row cites file:line evidence VERIFIED in source.
Acceptance driver: **no hydraulic objective/constraint/penalty semantics misclassified as generic** — EPANET
(adjacent lock, not code), Hazen-Williams/Manning (in `hydro-hydraulics`, never imported here), norm profiles,
unit/cost policy all stay KEEP.

Legend: **EXTRACT** = generic math, moves verbatim (modulo arity-generalization noted).
**REWRITE-USING-LIBRARY** = Hydraulics-owned code that must be re-expressed against future `mamut-nsga3` traits
without copying domain semantics into the library. **KEEP** = never enters MAMUT Math.
"Split-seam" = generic core with a domain wrapper; the cut line is stated.

## Reference points (Notion surface)

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `uniform_reference_points`, `gen_refs` — `nsga3/reference_points.rs:23-51` | Pure Das–Dennis combinatorics; no domain input | **EXTRACT** | Zero domain references in file; inputs are `(nobj, p)` integers |
| `sel_nsga3` 5-obj/p=4 fixing — `nsga3/selection.rs:105-108` | Convenience binding of generic selection to the hydraulic 5-objective arity + 70-point lattice | **REWRITE-USING-LIBRARY** | Hardcodes `uniform_reference_points(5, 4)`; Hydraulics side must call generic `select_environmental` with its own lattice |

## Nondominated sort

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `fast_nondominated_sort` algorithm — `nsga3/nondom_sort.rs:37-84` | Deb-2002, domain-free | **EXTRACT** (generalize signature) | No domain symbols; only the `[f64; 5]` binding is hydraulic-arity-specific |
| `dominates` — `nondom_sort.rs:16-27` | Generic strict dominance | **EXTRACT** | Same as above |
| `dominates_5` + `ParetoFront::{update,len,into_pairs}` — `optimizer.rs:40-107` | Archive mechanics generic; fixed arity + duplicate of `dominates` | **Split-seam → REWRITE** | `update` eviction logic is generic (EXTRACT core), but its `[f64;5]` binding and duplication (anomaly A3) mean the Hydraulics side rewrites against a generic `ParetoArchive<M>` |

## Normalization

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `normalize`, `NormalizeResult`, `find_extreme_points`, `compute_asf`, `compute_intercepts`, `fallback_intercepts` — `nsga3/normalize.rs:18-224` | Deb & Jain §4.1 verbatim; no domain input | **EXTRACT** (generalize 5×5 Gauss elimination to M×M) | File imports nothing domain; constants are algorithmic (`1e-6` ASF weight, `1e-12` degeneracy guard) |

## Niching

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `perp_distance`, `associate`, `build_niche_counts`, `select_from_partial_front` — `nsga3/niching.rs:21-158` | Deb & Jain §4.2; deterministic index tie-break | **EXTRACT** (generalize `&[f64;5]` to slices) | No domain imports; unused `rng` param is a reserved hook (lines 97-98), not a dependency |
| `select_environmental` — `nsga3/selection.rs:28-100` | Generic orchestration (sort→fill→normalize→associate→niche) | **EXTRACT** | Operates on `&[[f64;5]]` + caller-supplied `ref_points`; arity is the only cut |

## Operators

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `adaptive_eta_value` — `operators.rs:34-37` | Pure linear interpolation | **EXTRACT** | No inputs beyond numbers |
| `sbx_crossover`, `sbx_betaq` — `operators.rs:48-119` | Deb & Agrawal 1995 / DEAP exact; bounds from generic `GeneSpec.lo/hi` | **EXTRACT** | Operates on `&[GeneSpec]` boxes + `&mut ChaCha20Rng`; Int genes treated as raw floats by design (lines 10-13) |
| `polynomial_mutation`, `poly_mutation_delta` — `operators.rs:131-186` | Deb 2001 / DEAP exact; documented fitness-invalidation improvement (lines 127-130) | **EXTRACT** | Same generic signature; improvement travels with the algorithm |
| `var_or` — `operators.rs:207-257` | DEAP-varOr-equivalent; documented both-children divergence (lines 190-202) | **EXTRACT** | Divergence is algorithmic trade-off, not domain; parity validated at HV/IGD+ level per module docs |
| `init_population` — `operators.rs:268-276` | Generic given a sampler + RNG | **Split-seam → EXTRACT core** | Body delegates to domain-coupled `random_individual` (SolverType table); library takes `init_population(sampler_fn, n, rng)` |

## RNG

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `root_rng`, `child_rng` (splitmix64 `(gen,idx)` derivation) — `rng.rs:15-35` | Determinism primitive; Design §5 exact formula | **EXTRACT** | Depends only on `rand`/`rand_chacha`; consumed by operators, selection, optimizer uniformly |
| `Arc<Mutex<S>>` sequential-evaluation discipline + `&mut ChaCha20Rng` threading — `optimizer.rs:111-127,429-458` | Concurrency architecture forced by `Solver::solve(&mut self)` | **KEEP** (constraint) | Library must offer a single-threaded `evaluate` hook; the mutex lives Hydraulics-side (REQ-017 defers rayon) |

## Encoding

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `GeneType`, `GeneSpec` shape, `Individual { genes, fitness }` — `encoding.rs:24-44,327-332` | Generic box-dtype descriptor + chromosome container | **EXTRACT** (as library `Variable`/`Individual` concepts; CON-001 already owns sibling contracts) | No hydraulic names in the shapes themselves |
| `SolverType` enum + string conversions — `encoding.rs:50-95` | Six hydraulic project types | **KEEP** | Domain taxonomy; must become a generic type parameter, never a library enum |
| `GENE_SPECS` values / `gene_specs()` table — `encoding.rs:108-318` | Oracle-exact hydraulic parameter names, bounds, dtypes (e.g. `manhole_spacing [60,120]`, `source_head [20,60]`) | **KEEP** | Domain calibration data |
| `IndividualEncoder::random_individual` mechanics — `encoding.rs:344-357` | Uniform-box sampling (generic) keyed by domain table | **Split-seam** | EXTRACT the sampler shape; KEEP the table |
| `IndividualEncoder::decode` (clamp + `round()`) — `encoding.rs:366-395` | Mechanics generic; spec names domain | **Split-seam** | Library `decode(genes, specs)` over generic specs; Hydraulics supplies hydraulic spec rows |
| `params_to_solver_params` 11-field mapping — `optimizer.rs:512-551` | Gene-name → `SolverParams` hydraulic wiring | **KEEP** | Domain adapter, thin by design |

## Constraints

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `check_bounds` (tol 0.001, raw-distance magnitude) — `constraints.rs:488-530` | Box-constraint check; tolerance is a ported oracle constant, not physics | **EXTRACT** (tolerance becomes a parameter) | No `PipeNetwork`/domain input; the only checker wired into production (`optimizer.rs:463`) |
| `ConstraintReport` shape + spec penalty `Σ(1e12+mag·1e9)` — `constraints.rs:74-102` | Report/aggregation shape generic; 1e12/1e9 magnitudes are spec policy | **Split-seam** | EXTRACT `FeasibilityReport` + penalty-combiner; KEEP the 1e12/1e9 constants and `1e12+[..]` sentinel application (`optimizer.rs:465-466`) |
| `check_route/check_solution/check_full` + all `_inner` detectors + constructors `new_geometric/new_with_cover/new_with_clearance/new_with_slopes/new_full/canonical new` — `constraints.rs:251-900` | Forbidden-zone / mandatory-corridor / bend / cover / slope / clearance evaluation over `PipeNetwork` with domain defaults (0.9 m, 6.0 m, 0.003–0.15, 0.3/1.0 m) | **KEEP** | Hydraulic semantics throughout; note dormant-in-production (A1) — KEEP does not mean wire up |
| `DesignConstraints`, `ExistingNetworkEntry`, `ConstraintViolationKind`, config zone structs — `constraints.rs:28-145`, `config.rs:17-47` | Domain violation taxonomy + limits | **KEEP** | Must not enter generic API |
| 2-D geometry helpers (`seg_intersect`, `point_in_polygon`, `polygon_area`, distances, `angle_between`, `solution_pipe_lines`) — `constraints.rs:902-1097` | Deterministic computational geometry, zero domain content | **EXTRACT** | Pure `(f64,f64)` math; `solution_pipe_lines` stays KEEP (PipeNetwork coupling) — cut at the `PipeNetwork` boundary |

## Objectives

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `ObjectiveFunction::{new,score,calc_cost,calc_excavation,calc_pumping,calc_interference,calc_resilience}` — `objective.rs:107-304` | Cost catalogue, trench economics, pump CAPEX/OPEX, utility-conflict counting, Todini-vs-connectivity switch | **KEEP** (all five) | Every calculator embeds hydraulic policy (USD tables, H_MIN=15 mca, separations, pump metadata) |
| `pipe_cost_catalogue`, `pipe_unit_cost`, cost/separation constants — `objective.rs:25-86` | Oracle-exact price tables + policy constants | **KEEP** | Unit/cost policy owned by Hydraulics per AGENTS §3 |
| `todini_index`, `connectivity_redundancy` — `objective.rs:313-396` | Todini-2000 applied to pressure networks (domain application); Rc graph formula used as gravity-resilience proxy (domain selection) | **KEEP** | Literature formulas applied with hydraulic meaning (H_MIN, Tank/Reservoir roles, `pressure_mca` metadata); a generic graph-ratio helper could be EXTRACTed only if a future ticket proves domain-free need — not this one |
| `lifecycle_opex_usd` (ρ,g,η=0.7,4000 h,0.10 USD/kWh,20 yr) — `objective.rs:406-421` | Energy-economics policy | **KEEP** | Tariff/lifetime assumptions are domain policy |
| `ExistingNetworkSpec` — `objective.rs:94-99` | Interference input descriptor | **KEEP** | Domain input shape |
| Objective geometry duplicates — `objective.rs:428-509` | Same generic 2-D math as constraints side | **EXTRACT** (deduplicated with constraints side into one library geometry module) | Anomaly A3; single shared implementation |

## Penalties

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| Aggregation formula `Σ(1e12 + mag·1e9)` — `constraints.rs:94-95` | Spec formula, REQ-004/tasks-#578 authoritative | **Split-seam** (combiner EXTRACT, constants KEEP) | Formula shape generic; magnitudes domain policy |
| Application: `penalty = 1e12 + report.penalty`, `[1e12;5]` sentinels, `fitness[0] < 1e11` feasibility gates — `optimizer.rs:217,277,285,464-466,490,499,585` | Infeasibility encoding + thresholds scattered through the loop | **REWRITE-USING-LIBRARY** | Hydraulics side adopts a library `Infeasible(f64)` encoding; thresholds stay domain constants |
| `OptimizationError` variants (esp. `Solver`, `NormValidationFailure`) — `errors.rs:14-42` | Setup/non-recoverable error taxonomy | **KEEP** (shape reference only) | Library defines its own MAMUT-owned error type (ARC-002 isolation); `Solver` variant cannot cross the boundary |

## Metrics

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `hypervolume`, `hypervolume_slices` — `metrics.rs:30-90` | Exact N-D dominated HV, minimization | **EXTRACT** | No domain input; arbitrary-M already |
| `igd_plus` (Ishibuchi 2015, sign-corrected) — `metrics.rs:107-137` | Pareto-respecting IGD+ | **EXTRACT** | Same; sign fix is algorithmic fact (parity docs) |
| `pub(crate)+allow(dead_code)` status — `metrics.rs:17,93` | Visibility gap, not math gap | **REWRITE (visibility)** | Library exposes these as first-class public API with MAMUT-owned signatures (`&[Vec<f64>]`-in/`f64`-out — no third-party types, satisfying ARC-002) |

## Public API

| Symbol / file | Generic vs hydraulic | Class | Justification (evidence) |
|---|---|---|---|
| `OptimizationConfig` hyperparameter core (pop/gen/probs/eta/seed/time/workers) — `config.rs:67-89` | Generic GA controls | **Split-seam** | Library owns a generic `Nsga3Config`; Hydraulics keeps domain flags (zones, bend, cover/slope/clearance, smoothing toggle, 5 weights, norm strings) and maps them (`hydro-cli/src/lib.rs:225-273` is the mapping precedent) |
| `GeneticOptimizer<S: Solver>` struct + `new`/`optimize`/`evaluate_population` — `optimizer.rs:120-506` | Loop wired to hydraulic `Solver` trait, `SolverParams` merge, domain sentinels | **REWRITE-USING-LIBRARY** | Future Hydraulics optimizer implements library traits (`Encoder`, `Evaluator`, `ConstraintChecker`, `RngSource` — see doc 03); the struct itself does not move |
| `ParetoResults` + `best_by_cost/excavation/resilience/balanced`, `comparison_table`, `summary`, `GenerationStats`, `ProgressEvent` — `results.rs`, `progress.rs`, `optimizer.rs:554-606` | Pareto-set reporting; selectors encode the hydraulic 5-objective choice | **Split-seam** | EXTRACT generic shapes (front container, weighted-sum selector, convergence stats, progress event); KEEP the 5-objective bindings + weight values |
| `Diagnostics` re-export (`hydro_types`) — `lib.rs:45` | HTTP-layer legacy schema | **KEEP** | Cannot change without breaking downstream (`diagnostics.rs:7-14`); library defines its own diagnostics |
| `SolverType` doc-hidden export — `lib.rs:61-62` | Test-visibility hack inside REQ-014 contract | **KEEP-out** | Must not propagate; library uses a generic parameter (anomaly A4) |

## Parity fixtures

| Fixture / consumer | Content | Class | Evidence |
|---|---|---|---|
| `tests/oracle/fixtures/optimizer_quality_metrics_golden.json` → `metrics.rs:257-276` | 3-pt approximation + 4-pt reference fronts, ref point `[6;5]`, oracle HV=784.0, IGD+=0.6889635872182424, tol 1e-12 | **KEEP (reference)** | Library adds its own oracle fixtures; this file stays the Hydraulics gate |
| `tests/oracle/fixtures/path_smoothing_golden.json` → `routing/mod.rs:186-210` | Blocking square, oracle waypoints (6.9,1.9)/(13.1,1.9), length 21.328780519261954, 2% tol | **KEEP (reference)** | Same; routing geometry EXTRACTs are validated against fresh library oracles |
| `solvers_*_golden.json` + `solver_wiring_smoke.rs` (pop 8/gen 3/seed 42) | End-to-end GA-through-solver feasibility gates | **KEEP** | Hydraulic solver behavior; unchanged by extraction |
| `eng022_no_epa_inner_loop.rs` + `benchmarks/performance_budgets.json` | EPA-out-of-inner-loop lock | **KEEP + must stay green** | Extraction must preserve every asserted path/symbol |

## Summary counts

- **EXTRACT (incl. split-seam cores)**: reference-points 2 · sort 2 · normalize 6 · niching 4+selection 1 · operators 7 · RNG 2 · encoding shapes 3 · bounds-check 1 · report-combiner 1 · geometry 15+4 · metrics 3 · routing (A*/RDP/visibility/euclidean/PathSmoother-core) ~12 · archive/progress/stats shapes 5 ≈ **65 symbols, all domain-free**.
- **KEEP**: SolverType, GENE_SPECS values, decode-table coupling, params mapping, all 5 objective calculators + catalogues + Todini/Rc application + OPEX policy, full geometric constraint stack + taxonomies + defaults, domain config flags, error variants, Diagnostics re-export, all fixtures/gates ≈ **45 symbols**.
- **REWRITE-USING-LIBRARY**: `sel_nsga3` binding, archive arity binding, penalty application + sentinels, `GeneticOptimizer` loop, config mapping, Pareto selectors/bindings, metrics visibility ≈ **12 sites**.
- **Acceptance check**: every symbol carrying EPANET-adjacent, Hazen-Williams/Manning-adjacent (no such imports exist — VERIFIED zero hits), norm, unit, or cost semantics classifies KEEP. No violation found.
