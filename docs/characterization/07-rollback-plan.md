# 07 — Rollback plan @ `6db4862d7a4035de8d494cdc62607b12b4edb103`

## 1. Rollback for THIS ticket (characterization only)

**Rollback = discard `docs/characterization/04–07` (plus any 01–03 touch-ups) in the
math worktree. Nothing else can break, because nothing else was touched.**

- This ticket moves no code by charter (Non-goals: no code move/refactor). Deliverables
  are documentation artifacts inside the math worktree
  `C:\Users\Jonnathan\Mamut system\mamut-math-nsga-001\docs\characterization\` only.
- Proof of non-interference (VERIFIED): `git status --short` in
  `optimizador-hidraulico-rust` shows **only** `M mamut.engine.json` (pre-existing
  local manifest change; `git diff --stat` confirms it is the sole diff — 78+/16−,
  no `.rs`/`.json` fixture/`.yml` touched); `git status --short` in
  `mamut-hydraulics` shows only `M AGENTS.md` (pre-existing); no `cargo` command was
  run in either Hydraulics repo, so no `target/` artifacts exist from this ticket.
- `mamut.engine.json` dirt is pre-existing, untouched by this ticket, and MUST never
  be bundled into any change (doc 03 §6; snapshot caveats A/C in doc 00).
- Math-worktree procedure: `git checkout -- docs/characterization/` (or delete the
  four files) on branch `ticket/MATH-NSGA-001-extraction-seam` from base
  `dc7dcdfd7633baa9e239a6634bff0e263e72b27c`; no Hydraulics action required at all.

## 2. Rollback posture RECOMMENDED for the future extraction ticket

### 2.1 Authority invariant (non-negotiable, grounded)

- Repo constitution §4 (AGENTS.md): **the existing Hydraulics optimizer remains the
  authority until the corresponding E2E gate approves substitution.**
- Code-level corroboration (VERIFIED at frozen SHA): the README "Two hydraulics"
  section keeps inner-loop fitness kernels authoritative and EPANET/SWMM as
  post-run verify; the ENG-022 executable lock
  (`hydro-optimizer/tests/eng022_no_epa_inner_loop.rs`, 195 lines) forbids
  EPA-per-individual invocation and is enforced in CI via
  `cargo test -p hydro-optimizer --test eng022_no_epa_inner_loop`.
- Consequence: extraction NEVER deletes or rewrites Hydraulics behavior in place.
  The Hydraulics optimizer keeps shipping; library adoption is per-call-site and
  reversible until the E2E substitution gate passes.

### 2.2 Regression anchor: parity fixtures frozen at this SHA

The extraction ticket re-freezes (by re-running the enumeration in doc 04 §1) and
then holds green as **row-level revert triggers**:

| Anchor | Location (frozen SHA) | Gate command (existing CI contract) |
|---|---|---|
| 201 inline + 9 integration `#[test]` in hydro-optimizer; 539 workspace attribute-declarations; zero `#[ignore]` | `hydro-optimizer/src/**`, `hydro-optimizer/tests/` | `cargo test --workspace` (`.github/workflows/ci.yml`) |
| 6-solver smoke (seed 42, 8/3, `fitness[0] < 1e11`) | `hydro-optimizer/tests/solver_wiring_smoke.rs` | `cargo test -p hydro-optimizer --test solver_wiring_smoke` |
| ENG-022 no-EPA-inner-loop lock | `hydro-optimizer/tests/eng022_no_epa_inner_loop.rs` + `benchmarks/performance_budgets.json` (`inner_loop` note) | `cargo test -p hydro-optimizer --test eng022_no_epa_inner_loop` |
| HV=784.0 / IGD+=0.6889635872182424 @ 1e-12 (DEAP provenance) | `tests/oracle/fixtures/optimizer_quality_metrics_golden.json` ← `metrics.rs:257-276` | `cargo test -p hydro-optimizer` (metrics module) |
| Path length 21.328780519261954 @ 2% | `tests/oracle/fixtures/path_smoothing_golden.json` ← `routing/mod.rs:186-210` | `cargo test -p hydro-optimizer` (routing module) |
| 474-vector hydraulics parity @ 1e-9 | `tests/oracle/fixtures/hydraulics_golden.json` ← `hydro-hydraulics/tests/oracle_parity.rs` | `cargo test -p hydro-hydraulics --test oracle_parity` |
| Solver goldens (6 + variants) | `tests/oracle/fixtures/solvers_*` ← `oracle::solvers` consumers (doc 04 §3.1) | `cargo test -p hydro-solvers` + solver smoke |
| Perf ceilings (incl. `pump_station_optimizer_8x3` 5 ms) | `benchmarks/performance_budgets.json` + `hydro-cli/benches/performance.rs` | `cargo bench -p hydro-cli --bench performance -- --noplot` + `scripts/check_benchmark_budgets.py` |

Any red gate reverts **that row only** (see §2.4), never the whole extraction.

### 2.3 Branch / worktree strategy + no-big-bang rule

- One ticket = one isolated branch/worktree (constitution §5); one executor per
  worktree; implementers never commit/push/merge (primary integrates).
- **No big-bang migration** (constitution §4): extraction proceeds **one EXTRACT row
  at a time** (doc 02 §Summary: ~65 EXTRACT-able symbols across reference-points,
  sort, normalize, niching/selection, operators, RNG, encoding shapes, bounds-check,
  report-combiner, geometry, metrics, routing, archive/progress/stats shapes).
- Only Hydraulics files an extraction row may edit: the `hydro-optimizer/Cargo.toml`
  dependency edge (`[dependencies] mamut-nsga3 = { … }` alongside the existing
  path-deps block, VERIFIED current content in doc 05-context) plus the row's
  `REWRITE-USING-LIBRARY` call sites (§2.4). Everything else stays byte-identical.

### 2.4 Per-crate cutover with feature flags + explicit revert sketch

- Cutover order (lowest domain coupling first): `rng` → `reference_points` →
  `nondom_sort` → `normalize`/`niching`/`selection` → `operators` → geometry kernel →
  `metrics` (visibility promotion) → config/archive/progress shapes. Each row lands
  behind its existing gate row (§2.2) before the next begins.
- Where a flag is needed (behavioral seam, e.g. `var_or` both-children divergence
  `operators.rs:190-202`, fitness-invalidation improvement `operators.rs:127-130`,
  `child_rng` activation at `optimizer.rs:458`), gate the new path on a
  `OptimizationConfig`-level boolean defaulting to legacy behavior, with the flag
  and its removal tracked as separate commits.
- **Explicit revert sketch** (the plan this characterization recommends the
  extraction ticket adopt verbatim): `revert = delete the mamut-nsga3 usage in the
  Hydraulics adapter, restore GeneticOptimizer call sites`. Concretely, per row:
  1. remove the `mamut-nsga3` dependency edge from `hydro-optimizer/Cargo.toml`;
  2. restore the listed call sites to their frozen-SHA form —
     `sel_nsga3` 5-obj/p=4 binding (`nsga3/selection.rs:105-108`), archive arity
     binding + `dominates_5` (`optimizer.rs:40-107`), penalty/sentinel application
     (`optimizer.rs:217,277,285,464-490,585`), `GeneticOptimizer::new/optimize/
     evaluate_population` (`optimizer.rs:144-506`), config mapping
     (`hydro-cli/src/lib.rs:225-273`, seed chain `:229,482-485` + `main.rs:76`),
     Pareto selectors (`results.rs:77-142`), metrics visibility (keep `pub(crate)` +
     `#[allow(dead_code)]` at `metrics.rs:17,93`);
  3. re-run the row's gate commands (§2.2) to confirm the tree is back at the frozen
     baseline (539 / zero-ignore / goldens green).
- The process-level adapter boundary needs no revert path: `mamut-adapter/server.py:3`
  declares the optimizer is a CLI binary fed DesignRequest JSON over stdin
  (VERIFIED lines 1–20), and `hydro-cli/src/main.rs:30,178` keeps a validate-only
  path that instantiates no optimizer — both are untouched by library adoption.
- Dormant behaviors stay dormant through reverts: `check_solution`/`check_full`
  unwired (A1), empty-`PipeNetwork` shaping at `optimizer.rs:374` (A2), norm counters
  hard 0 (`diagnostics.rs:33-43`). A revert must reproduce them exactly, not fix them
  (fixes belong to future tickets, per doc 02 KEEP decisions).

(End of file)
