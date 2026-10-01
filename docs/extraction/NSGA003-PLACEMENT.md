# NSGA-003 PLACEMENT DECISION — Generic Pareto metrics & quality primitives

- Ticket: MATH-NSGA-003 · Generic Pareto metrics and quality primitives (Risk L3, MATH_VERIFY=ON).
- Worktree/branch: `ticket/MATH-NSGA-003-pareto-metrics` @ `6cc7378` (verified clean, correct branch).
- Status: DECISION ONLY — no `.rs` created, moved, or edited by this document.
- Frozen inputs: EXTRACTION_CONTRACT MATH-NSGA-002 (esp. §A/§B1-row-10/§B3: metrics = REWRITE-visibility, OUT of scope there); docs 02 (metrics rows: `hypervolume`/`hypervolume_slices` EXTRACT, `igd_plus` EXTRACT, `pub(crate)+allow(dead_code)` REWRITE-visibility), 05 (golden `HV=784.0 / IGD+=0.6889635872182424`, ref `[6;5]`, tol 1e-12), 06 (exact N-D slicing HV + sign-corrected Ishibuchi IGD+, both `pub(crate)` dormant, zero non-test callers, arbitrary-M `&[Vec<f64>]` already); live crate `mamut-nsga3` (dynamic-M `dominates`/`fast_nondominated_sort`/`normalize`, `MathError`-only errors per `src/error.rs`, `internal::validate_fitness_matrix`) + `mamut-math-core` (`MathError`/`ErrorKind`, `ObjectiveValues`/`ValidFloat`, f64 canonical).

## 1. Placement (binding)

**All NSGA-003 work goes into the existing crate `mamut-nsga3`. NO new crate.** Concretely: one new module `crates/mamut-nsga3/src/metrics.rs` (single file; sub-split into `hv.rs`/`igd.rs` FORBIDDEN this ticket — preserves review surface, same rule as extraction-contract §B1-row-6) + one thin addition inside existing `src/nondom_sort.rs`. Re-export from `src/lib.rs`. `mamut-math-core` gains NO metrics code.

Rationale per frozen DAG responsibilities:
- `mamut-math-core` = algorithm-free contracts (variables, candidates, objectives, errors, RNG, evidence, codec). HV/IGD+/front-extraction are algorithms over those contracts, not contracts — putting them in core would violate the core's algorithm-free invariant.
- `mamut-nsga3` = generic Pareto / NSGA-III mathematics (sort, normalization, niching, selection, operators). Doc 02 classifies `hypervolume`/`igd_plus` as EXTRACT generic math and the only gap as visibility; the natural home is the Pareto crate, next to the dominance primitive they are defined over.
- ARC-002 freeze (`core/nsga3/sensitivity/gradient/hybrid/backend/testkit/wasm`) names no metrics crate. A new `mamut-metrics`/`mamut-pareto` crate would be a contract change requiring HUMAN (see §5). The workspace currently has only `core + nsga3`; staying inside `nsga3` needs no freeze amendment.

Detailed mapping:
- **Dominance + front extraction — REUSE, plus one thin helper.** `dominates` + `fast_nondominated_sort` exist in `mamut-nsga3::nondom_sort` (dynamic-M, validated, Deb-2002 loop order preserved). REUSE verbatim. ADD one pure helper in the same module: `extract_pareto_front(fitnesses) -> Result<Vec<usize>, MathError>` = `fast_nondominated_sort(fitnesses)` then `fronts[0]` (empty input → `Ok(vec![])`). ADD one validator for the property-test hook: `is_nondominated_set(fitnesses, indices) -> Result<bool, MathError>` (pairwise `dominates_unchecked` over validated matrix). No re-export wrapper crate, no archive struct this ticket (`ParetoFront` stays Hydraulics-side per doc 02 split-seam).
- **Hypervolume — NEW `metrics.rs`, exact-slicing port.** Port frozen `metrics.rs:30-90` (`hypervolume` + recursive `hypervolume_slices`) to `mamut_nsga3::metrics::hypervolume`, generalized validation but identical arithmetic (sort+dedup levels on last axis, append ref level, `Σ slice·Δ`, 1-D base `(ref−min).max(0)`). Arbitrary M including **5D natively supported — NO delegation** (frozen algorithm is already arbitrary-M; the 5D golden `784.0` proves the Hydraulics arity instance). Complexity caveat documented (`§4`), not an error.
- **IGD+ — same `metrics.rs`, direct port.** Port frozen `metrics.rs:107-137` to `metrics::igd_plus` (Ishibuchi 2015, minimization sign `max(0, a−r)`, `(1/|R|)·Σ min d+`). Requires explicit non-empty reference front of uniform arity M (see §3).
- **Normalization helpers — NEW metrics-local helpers; `normalize.rs` gap confirmed.** `mamut-nsga3::normalize` implements Deb&Jain §4.1 intercept normalization (ideal → ASF extremes → M×M solve → fallback). That is NSGA-III-internal and NOT reusable as metric preprocessing (it needs the full population + ref lattice context and its fallback is selection-oriented). ADD two small pure helpers in `metrics.rs` reusing `internal::validate_fitness_matrix`, NOT intercept logic: `normalize_to_unit_box(front, lower, upper)` (min-max affine map to `[0,1]^M`, typed errors on `upper<=lower`/arity) and its inverse `denormalize_from_unit_box`. HV/IGD+ themselves take raw f64 fronts + explicit ref/bounds; they never implicitly normalize (no hidden scaling placeholder).
- **Unsupported-case API — typed `MathError`, never a placeholder number.** Uses the existing frozen `ErrorKind` taxonomy (no new variant — taxonomy is stable/single-sourced): `EmptyCollection`, `DimensionMismatch`, `InvalidNotFinite`. Full matrix in §3. In particular the frozen silent behaviors (`hypervolume` filtering + `→0.0`, `igd_plus →∞`) are hardened: mathematically-defined zeros stay `Ok` (empty approximation HV), everything else becomes `Err`.

## 2. Public API signatures (binding sketches; implementer finalizes doc comments)

Conventions (all binding): MAMUT-owned types only; kernels on plain `f64` (`&[Vec<f64>]` in / `f64` or `Vec<usize>` out, mirroring `nondom_sort`/`normalize`); `V0.x` f64 canonical; **all objectives MINIMIZED** — stated on every signature; maximization callers transform (negate) before calling; no `ObjectiveSense` flag on kernels (sense lives at the core `ObjectiveDescriptor`/session boundary). Errors are `mamut_math_core::MathError` only (per live `src/error.rs`, not the contract-sketch `Nsga3Error`).

```rust
// crates/mamut-nsga3/src/nondom_sort.rs — ADD:
pub fn extract_pareto_front(fitnesses: &[Vec<f64>]) -> Result<Vec<usize>, MathError>;
pub fn is_nondominated_set(fitnesses: &[Vec<f64>], indices: &[usize]) -> Result<bool, MathError>;

// crates/mamut-nsga3/src/metrics.rs — NEW (all minimization; see §3 for Errors):
/// Exact dominated hypervolume of `front` w.r.t. `reference_point` (N-D recursive slicing).
pub fn hypervolume(front: &[Vec<f64>], reference_point: &[f64]) -> Result<f64, MathError>;
/// IGD+ (Ishibuchi 2015) of `front` against explicit `reference_front`.
pub fn igd_plus(front: &[Vec<f64>], reference_front: &[Vec<f64>]) -> Result<f64, MathError>;
/// Min-max affine map of a front into the unit box; inverse below.
pub fn normalize_to_unit_box(front: &[Vec<f64>], lower: &[f64], upper: &[f64]) -> Result<Vec<Vec<f64>>, MathError>;
pub fn denormalize_from_unit_box(normed: &[Vec<f64>], lower: &[f64], upper: &[f64]) -> Result<Vec<Vec<f64>>, MathError>;
```

`src/lib.rs`: add `pub mod metrics;` + `pub use metrics::{hypervolume, igd_plus, normalize_to_unit_box, denormalize_from_unit_box};` and `pub use nondom_sort::{extract_pareto_front, is_nondominated_set};` (extend existing `nondom_sort` re-export line). Module docs cite Deb 2002 (sort/front), Deb&Jain 2014 (normalization context), Ishibuchi et al. 2015 (IGD+), and the exact-slicing HV recurrence + DEAP/independent-Python oracle provenance.

## 3. Dimension-aware validation + explicit unsupported-case matrix (binding)

Every entry validates via `internal::validate_fitness_matrix` (M ≥ 1, uniform arity, finite) plus its own ref/bounds arity check. Duplicates are deduplicated (documented, HV/IGD+-neutral). Points of `front` strictly beyond the reference box contribute zero by HV definition (documented mathematical filtering — the ONLY silent filtering allowed; ragged/non-finite inputs are never filtered, they are `Err`).

| Case | `hypervolume` | `igd_plus` |
|---|---|---|
| Empty `front`, valid ref (`M≥1`, finite) | `Ok(0.0)` — measure of the empty set, NOT a placeholder | `Err(EmptyCollection)` (frozen `∞` replaced; `∞` in an `f64` return is a misleading placeholder) |
| Empty / non-finite / `M==0` reference | `Err(EmptyCollection` / `InvalidNotFinite)` | `Err(EmptyCollection)` for empty `reference_front` |
| Ragged rows, `ref.len() != M`, `reference_front` arity `!= M` | `Err(DimensionMismatch)` | `Err(DimensionMismatch)` |
| Any `NaN`/`±Inf` entry | `Err(InvalidNotFinite)` | `Err(InvalidNotFinite)` |
| `front == reference_front` (IGD+) | — | `Ok(0.0)` exact |
| Large M / large N (perf, NOT correctness) | `Ok` — documents `O(N^M)`-class slicing cost; caller subsamples or delegates offline; library never fabricates | `Ok` — `O(\|front\|·\|R\|·M)`; same delegation note |
| Degenerate (all-identical / single-point) | defined single-box volume, no error | defined (0 iff on reference) |

There is NO correctness-unsupported M: exact slicing covers arbitrary M, so the 5D Hydraulics DoD branch resolves to **natively supported** (golden §4), delegation branch unused. No `0.0`/`∞` is ever returned for an invalid input.

## 4. Dependency directions + parity/oracle hooks (binding)

- **Deps:** `metrics.rs` imports ONLY `mamut_math_core::{MathError, ErrorKind}` + `crate::internal` (+ `std`). No domain terms (no cost/coverage/pressure/resilience/solver names, no 5-objective constants, no `[6;5]` default, no `1e12/1e9/1e11` magnitudes). No new `[dependencies]`. Extend `tests/architecture.rs` with `metrics_uses_only_core_types` (public API expressible with core+std+crate types) — same technique as the existing no-third-party surface tests.
- **Oracle hooks for the verifier (MATH_VERIFY=ON):** (a) analytical cases re-pinned library-side: HV 1-pt/2-pt 2-D, 1-pt 5-D `= 32.0`, 2-pt 3-D union `= 6.0`, empty `→ 0.0`; IGD+ perfect-match `0`, ref-better `√2`, front-better `0`, self-distance `0`; (b) 5D DEAP-offline golden re-committed as a library fixture (approximation + 4-pt reference fronts, ref `[6;5]`, `HV=784.0`, `IGD+=0.6889635872182424`, `1e-12` abs tol — values from docs 05/06, vectors re-encoded from the frozen Hydraulics golden file WITHOUT widening Hydraulics visibility); (c) property tests: `is_nondominated_set(extract_pareto_front(f)) == true`, concatenation of all fronts `== 0..N`, HV monotonicity (adding a nondominated point never decreases HV), `igd_plus(F,F) == 0`. Frozen Hydraulics golden file stays the Hydraulics-side gate; the library fixture is fresh, not a visibility widening (per doc 06 §4c).
- Source refs documented in `metrics.rs` rustdoc: HV definition + slicing recurrence, IGD+ formula + minimization sign, orientation convention, validation matrix pointer, complexity note, oracle provenance (DEAP exact HV + independent-Python IGD+).

## 5. Files the implementer creates/modifies (closed list)

Create: `crates/mamut-nsga3/src/metrics.rs`; `crates/mamut-nsga3/tests/metrics_parity.rs` (analytical + 5D golden); `crates/mamut-nsga3/tests/fixtures/metrics_5d_golden.json` (re-encoded 3-pt/4-pt 5-D fronts + ref + oracle values, `include_str!`, no new dev-deps).
Modify: `crates/mamut-nsga3/src/nondom_sort.rs` (add 2 helpers); `crates/mamut-nsga3/src/lib.rs` (mod + re-exports + module-map docs); `crates/mamut-nsga3/tests/architecture.rs` (metrics surface test); `crates/mamut-nsga3/tests/properties.rs` (front + HV-monotonicity + IGD+-identity properties). Touch nothing else (no core, no workspace `Cargo.toml`, no session/config/operators).

**Contested-placement HUMAN_DECISION option (single, only if invoked):** promote metrics to a new crate (e.g. `mamut-metrics`) sibling to `mamut-nsga3`. NOT recommended: it adds an ARC-002 crate-list contract change, a second Pareto-adjacent home for dominance-coupled primitives, and cross-crate arity-validation duplication for zero mathematical benefit. If HUMAN orders it, the freeze amendment + `members` + `mamut-metrics → mamut-math-core` direction must precede any implementation.
