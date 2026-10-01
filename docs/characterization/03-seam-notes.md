# 03 — Seam notes for the library side (`mamut-nsga3` future)

Frozen SHA `6db4862d7a4035de8d494cdc62607b12b4edb103`. No speculation about future code beyond what the
current code requires of any successor: each note cites the exact constraint the present implementation imposes.
Governing architecture: ARC-002 crate freeze (`core/nsga3/sensitivity/gradient/hybrid/backend/testkit/wasm`),
V0.x canonical `f64`, no third-party types in stable public API, MAMUT-owned API.

## 1. RNG determinism model (must be preserved exactly)

- **Root**: `ChaCha20Rng::seed_from_u64(seed)` with `seed` default 42 (`rng.rs:15-17`; `config.rs:199`).
  Provenance: `seed_override.unwrap_or(req.seed.unwrap_or(42))` (`hydro-cli/src/lib.rs:229`).
- **Child derivation** (Design §5 exact formula, `rng.rs:25-35`):
  `s = seed·0x9E3779B97F4A7C15 + (generation<<32) + idx`, then 3-round splitmix64
  (`0xBF58476D1CE4E5B9`, `0x94D049BB133111EB`, shifts 30/27/31), `seed_from_u64(s)`.
  Purpose: rayon-worker-independent reproducible streams.
- **Threading**: one `&mut ChaCha20Rng` through every stochastic call (operators `operators.rs:48-276`,
  `sel_nsga3`/`select_environmental` `selection.rs:28-108`, `init_population`). No `thread_rng`, no `HashMap`
  anywhere on the stochastic path — `BTreeMap`/`BTreeSet`/sorted-`Vec` only (design §12; e.g.
  `astar.rs:37`, `constraints.rs:677`, `objective.rs:187`).
- **Tie-breaks**: insertion order (archive `optimizer.rs:39`, sort `nondom_sort.rs:36`), smallest index
  (niching `niching.rs:136-146`), `(f_bits, idx)` ordering (A* `astar.rs:56-62`).
- **Deferred parallelism**: evaluation is sequential behind `Arc<Mutex<S>>` because
  `Solver::solve(&mut self)` (`optimizer.rs:111-127,429-458`); `rayon` removed from deps (Cargo.toml comment,
  REQ-017 → Phase 7 solver-pool). The library must therefore expose a **sequential-first evaluation hook**
  (parallelism is the caller's composition, e.g. a future solver pool), plus a `RngSource` abstraction
  compatible with the splitmix64 `(seed, generation, index)` scheme so bit-identical fronts remain achievable.
- **Determinism tests that pin the model**: `config.rs:493-510` (root/child reproducibility + divergence),
  SBX/mutation/varOr/init determinism (`operators.rs:385-397,421-431,493-506,572-581`), selection determinism
  (`selection.rs:239-253`).

## 2. Encoding coupling (the tightest seam)

Gene data flows one way: `GENE_SPECS` table → bounds vectors → sampler/operators/checker → decode →
`solver_params` merge:

1. `GeneticOptimizer::new` builds `lower`/`upper` Vecs from `gene_specs()` keyed by `SolverType`
   (`optimizer.rs:151-157`) and hands them to `ConstraintChecker::new` (bounds layer only in production).
2. `init_population` → `random_individual` samples uniformly per-gene boxes (`encoding.rs:344-357`).
3. `sbx_crossover` / `polynomial_mutation` / `var_or` operate on raw floats within those boxes
   (`operators.rs:48-257`); Int genes are rounded **only** at `decode` (`encoding.rs:384-392`).
4. `evaluate_population`: bounds-gate → `decode` → `params_to_solver_params` merges decoded genes over
   CLI-supplied `base_params` (unevolved fields pass through unchanged, `optimizer.rs:479-481,512-551`).
5. `hydro-cli` builds `base_params` from `DesignRequest` (`lib.rs:280-292`) — the domain-owned merge precedent.

Library implication: the encoder must be a **caller-supplied trait** (`specs() → boxes+dtypes`,
`random(rng)`, `decode(genes)`) with `Individual { genes: Vec<f64>, fitness }` generic over arity M.
The 6-type table, gene names, bounds, and the 11-field `SolverParams` mapping stay Hydraulics-side.

## 3. Constraint / penalty injection points

- **Wired**: gene-bounds gate in `evaluate_population` (`optimizer.rs:462-468`) with
  `penalty = 1e12 + report.penalty`; decode failure → `[1e12; 5]` (lines 471–477); solver error →
  `[1e12; 5]` (lines 486–494); empty solution → `[1e12; 5]` (lines 497–500); initial-feasibility gate
  `fitness[0] < 1e11` else `AllInfeasible` (lines 215–221); same `1e11` threshold reused at lines 277, 585.
- **Present but dormant**: `check_solution` / `check_full` (geometric + norm-adjacent layers) are never called
  by `optimize()`; `new` passes `None` design constraints and empty existing nets (`optimizer.rs:159-178`);
  norm counters hard 0 (`diagnostics.rs:33-43`). Adverse (uphill) slopes are explicitly excluded from slope
  violations and owned by the pumping objective (`constraints.rs:810-814`).
- Library implication: constraints enter as a **caller-supplied predicate/report trait** returning a generic
  `FeasibilityReport { feasible, violations: Vec<(kind, magnitude)> }` plus a library penalty-combiner with
  caller-supplied constants. The violation taxonomy, thresholds (0.001 tolerance, 1e12/1e9, 1e11 gates),
  and the decision to wire (or not) geometric layers remain Hydraulics policy. A successor must consciously
  decide whether to wire `check_solution` — this document records it as dormant, not as recommended.

## 4. API-shape implications for a future `mamut-nsga3` crate

To absorb the EXTRACT column without domain leakage, the library needs (at minimum) these MAMUT-owned,
`f64`-canonical, third-party-free generic parameters — each derived from an observed coupling:

| Library concept | Derived from (evidence) | Must be generic over |
|---|---|---|
| `Individual<M>` / chromosome ops | `Individual` (`encoding.rs:327`), all `[f64;5]` signatures | Objective arity M (const or dynamic); today hardcoded 5 in sort/normalize/niching/selection/archive/progress/results |
| `Encoder` trait (specs/random/decode) | `GeneSpec`/`GENE_SPECS`/`IndividualEncoder` (`encoding.rs`), `init_population` (`operators.rs:268`) | Spec row type; Int-vs-Float handling |
| `Evaluator` trait (genes → objectives) | `evaluate_population` pipeline (`optimizer.rs:433-506`), `Solver` trait (`hydro-solvers/src/solver.rs:91-103`) | sync-first `evaluate(&mut self)`-compatible hook; solution type |
| `ConstraintSource` trait | `check_bounds` (`constraints.rs:488`) as the minimal wired example | Report type with domain-defined kinds |
| `ReferencePoints` / selection config | `uniform_reference_points` + `sel_nsga3` p=4 fixing (`selection.rs:105-108`) | nobj, divisions p, association metric |
| `RngSource` | `rng.rs` model (§1) | Seed/child-derivation scheme |
| Metrics as public API | `hypervolume`/`igd_plus` `pub(crate)` gap (`metrics.rs:17,93`; anomaly A5) | `&[Vec<f64>]`-in/`f64`-out, no `rand`/`geo` leakage |
| Geometry kernel | Duplicated 2-D helpers (A3) + routing core | Pure `(f64,f64)`/`[f64;2]` functions; `ForbiddenZone` stays domain-side, passed as slices |

Non-goals confirmed by absence: the optimizer never imports `hydro_norms`/`hydro_hydraulics`/`hydro_terrain`
(VERIFIED zero hits), so the library needs **no** physics, norm, or terrain concepts. `rand_chacha` appears
only in `pub(crate)` signatures today — the public path takes a `u64` seed — so ARC-002 isolation is
achievable by keeping RNG types behind the `RngSource` trait, never in stable signatures.

## 5. Explicit MUST-NOT-EXTRACT list

1. `SolverType` and all six `GENE_SPECS` rows (names, bounds, dtypes) — `encoding.rs:50-318`.
2. `params_to_solver_params` mapping and `base_params` merge semantics — `optimizer.rs:512-551`; `hydro-cli/src/lib.rs:280-292`.
3. All five objective calculators, cost catalogues, constants (USD tables, H_MIN, separations, ρ/g/η/tariff/lifetime) — `objective.rs:25-421`, including Todini and Rc **as applied**.
4. Full geometric constraint stack, `DesignConstraints` defaults, violation taxonomy — `constraints.rs:28-900`; config zone structs — `config.rs:17-47`.
5. Penalty constants (1e12/1e9), sentinel `[1e12;5]`, feasibility threshold 1e11 — `constraints.rs:94-95`; `optimizer.rs:217,277,285,464-490,585`.
6. Domain config flags (spatial zones, bend/cover/slope/clearance, smoothing toggle, 5 weights, norm strings) — `config.rs:90-124`.
7. `OptimizationError::Solver/NormValidationFailure`, `Diagnostics` re-export — `errors.rs:31-34`; `lib.rs:45`.
8. `Solver` trait, six solvers, `SolverParams`, `TerrainModel` usage — `hydro-solvers`, `solver_wiring_smoke.rs`.
9. All oracle fixtures, ENG-022 lock, performance budgets — tests stay authoritative Hydraulics-side.
10. The `check_solution`-dormant and empty-`PipeNetwork` shaping behaviors (A1/A2) — record, do not replicate blindly.

## 6. Frozen baselines, HV/IGD+ gap, rollback

- **SHAs**: optimizer `6db4862d7a4035de8d494cdc62607b12b4edb103` (dirty: `mamut.engine.json` only, manifest);
  scaffold `2a74e9dda021bd6bd575dfb3a1a25b800767f26a` (context only, no code). Math worktree base
  `dc7dcdfd7633baa9e239a6634bff0e263e72b27c` (see `00-execution-snapshot.md`).
- **Test baselines (read-only; no cargo run per ticket guardrails)**: `docs/TEST-REPORT.md` (2026-06-02,
  at `c73e677`) records ~510 tests (178 integration + ~332 inline), REQ-001–018, with 2 ignored parity
  skeletons; `docs_dev/optimizer_parity_progress.md` (2026-06-19) records the closed gap — 201 unit + 6
  integration smoke, zero ignored. The TEST-REPORT predates the frozen SHA: treat its counts as historical,
  the parity-progress note as the proximate baseline. A future extraction ticket re-freezes counts by running
  the suite itself.
- **HV/IGD+ gap (known, documented)**: per `docs_dev/optimizer_parity_{findings,progress,task_plan}.md`
  (2026-06-19): Rust HV was 2-D-only (returned 0 for N-D) while the gate demanded 5-D; an IGD+ sign error
  (`max(ref−approx)` vs correct `max(approx−ref,0)`) was found via fixture generation; a clearance/visibility
  defect made A* fall back to the blocked direct segment (caught at 6.23% vs 2% tolerance). All three were
  fixed: exact recursive-slicing N-D HV (`metrics.rs:53-90`), corrected IGD+ (`metrics.rs:122-123`), miter-offset
  visibility nodes (`visibility.rs:58-72`). The residual, still-open gap is architectural, not algorithmic:
  the gate is **metric-level differential parity** (committed DEAP fixture, 1e-12 tol) because end-to-end
  optimizer-quality comparison is infeasible in CI (Python minimum 4/1 pump-station run >10 min on one CPU);
  full quality comparison belongs to the benchmark/regression suite. Compounding this, both metric functions
  remain `pub(crate)` + `#[allow(dead_code)]` with no production caller — the library extraction must promote
  them to public API with fresh oracles rather than widening Hydraulics visibility as a shortcut.
- **Rollback plan**: this ticket moves no code, so rollback = discard `docs/characterization/`. For the
  future extraction this characterizes: (a) the Hydraulics optimizer remains the authority until its
  corresponding E2E gate approves substitution (AGENTS §4 invariant); (b) extraction proceeds one
  EXTRACT-row at a time behind the frozen gates (`solver_wiring_smoke`, `eng022`, both golden fixtures,
  `performance_budgets.json` lock) — any red gate reverts that row only; (c) no Hydraulics file is edited
  except its `Cargo.toml` dependency edge + `REWRITE` call-sites, which are reverted as a unit if the gate
  fails; (d) `mamut.engine.json` dirt is pre-existing and untouched — never bundled into any change.
