# NSGA-III Independent Conformance — Provenance

This directory holds the committed fixtures and the provenance for the
MATH-NSGA-004 independent NSGA-III conformance suite. All reference values
trace to an independent oracle (pymoo / DEAP) or to a closed-form literature
Pareto front sampled in-script. The MAMUT Rust crate is **never** imported by
the oracle scripts, so Rust-vs-fixture parity cannot be circular.

## Oracle environment (exact versions)

| Component | Version |
|---|---|
| Python | 3.13.7 (`C:\Users\Jonnathan\AppData\Local\Programs\Python\Python313\python.exe`) |
| pymoo | 0.6.2 |
| DEAP | 1.4 |
| numpy | 2.2.6 |

**DEAP-has-no-NSGA3 disclosure.** DEAP 1.4 ships no NSGA-III algorithm and no
IGD/IGD+ metric. Therefore:

* **Algorithm-level oracle = pymoo `NSGA3`** (Deb & Jain 2014 reference-direction
  survival).
* **Component-level oracle = DEAP** — `deap.tools.sortNondominated`
  (Deb 2002 fast nondominated sort), the compiled-core exact 3D hypervolume
  `deap.tools._hypervolume.hv.hypervolume`, `cxSimulatedBinaryBounded`,
  `mutPolynomialBounded`.
* **IGD+ and the reference Pareto fronts are computed in-script** (numpy) from
  the literature formulas; pymoo's internal Pareto fronts are not used.

## Scripts

| Script | Output |
|---|---|
| `scripts/oracles/oracle_common.py` | shared helpers: provenance, exact 3D HV, IGD+, DTLZ PF sampling, HV reference points |
| `scripts/oracles/component_oracle.py` | `component_oracle.json` |
| `scripts/oracles/pymoo_full_oracle.py` | `pymoo_dtlz_oracle.json` |

Both scripts were run offline; the committed JSON fixtures are consumed by the
Rust tests, which need no Python at `cargo test` time.

## Problems, budgets, seeds

* DTLZ1 (n=7, k=5), DTLZ2 (n=12, k=10), DTLZ7 (n=22, k=20); 3 objectives,
  all minimized, `x in [0,1]^n` (Deb & Thiele 1999 / Deb 2001).
* Reference directions: das-dennis `p=12` → 91 points (pymoo
  `get_reference_directions("das-dennis", 3, n_partitions=12)`; verified
  bit-identical in ordering to the crate's `uniform_reference_points`).
* Population 92; evaluation budget 9200; 99 offspring generations
  (`92 + 99*92 = 9200`); seeds 0..7; terminate on exact `n_evals`.
* Seal: pymoo `minimize(..., ("n_evals", 9200), seed=s)`.

### Pareto-front formulas (in-script)

* **DTLZ1**: simplex `{f >= 0 : sum f = 0.5}` (das-dennis lattice x 0.5).
* **DTLZ2**: positive unit-sphere patch (simplex lattice normalised to unit
  norm).
* **DTLZ7**: `2^(M-1) = 4` disconnected patches with `g = 1` (grid over
  `f1, f2 in [0,1]`, `f3 = (1+g) h`).

### Hypervolume reference points

* DTLZ1 `[0.6, 0.6, 0.6]`, DTLZ2 `[1.1, 1.1, 1.1]`, DTLZ7 `[1.1, 1.1, 6.5]`.

### HV / IGD+ oracle implementation

* HV: independent exact 3-objective objective-sweep in `oracle_common.hv3d`,
  cross-checked in-script against a brute-force inclusion-exclusion
  (`hv_bruteforce`) and against DEAP's compiled exact core.
* IGD+: Ishibuchi et al. (2015) `d+(a,r) = sqrt(sum_j max(a_j - r_j, 0)^2)`.
* Reference-front integrity (MATHEMATICALLY CERTIFIED DTLZ7). A DTLZ7
  reference front must be a subset of the TRUE nondominated front. On the PF
  (`g = 1`, `x_3..x_n = 0.5`) the surface is `f3 = 6 - A(f1) - A(f2)` with
  `A(x) = x(1 + sin(3 pi x))`. A surface point `(x, y, f3)` is nondominated IFF
  `x` and `y` are RECORD-HIGH points of `A` (`A(t) >= A(u)` for all `u <= t`).
  The record-high set is `[0, a] U [b, c]`, derived at 50 decimal digits
  (`mpmath.findroot`, secant with brackets) and cast to `f64`:

  | endpoint | derived (f64) | human corroboration | delta |
  |---|---|---|---|
  | `a` (first local max of `A`) | 0.2514118360889171 | 0.251411836088917 | 1.19e-16 |
  | `b` (record crossing `A(b)=A(a)`) | 0.6316265307000613 | 0.631626530700061 | 2.00e-16 |
  | `c` (global max of `A`) | 0.8594008566447239 | 0.859400856644724 | 8.40e-17 |

  `!derive_dtlz7_endpoints()` asserts each derivation against the human constant
  with tolerance `1e-14` (corroboration, not the derivation source).
* **Fold refinement (derived).** `A` is strictly increasing on `[0,a]` and on
  `[b,c]`, and the branches meet only at the single value `A(a) = A(b)`. At that
  tie the smaller coordinate is better for minimization (same `f3`, smaller
  `f1`), so the point at `b` is dominated by the point at `a`. The certified
  Pareto coordinate set is therefore `[0,a] U (b,c]` — the high interval is
  sampled OPEN at `b`; `c` is kept. `pf_dtlz7_certified(n_per_interval=20)`
  builds the 4 Cartesian patches over `[0,a]` and `(b,c]` (1600 points),
  `f3 = 6 - A(f1) - A(f2)`.
* **Excluded gap.** Points with `f1` or `f2` in `(a, b)` (e.g. the legacy grid
  node `8/31 = 0.25806`, where `a = 0.25141 < 8/31 < b`) are continuously
  dominated and are NOT part of the true front. `pf_dtlz7_grid_raw` keeps the
  legacy raw grid for the regression demonstration only.
* **Regression tests** (`dtlz7_certified_reference_regression` in
  `conformance_quality.rs`): R1 — `8/31` in the gap is dominated by the
  certified `(1/4, 0)` with `A(1/4) > A(8/31)`; R2 — every certified coordinate
  is inside `[0,a] U [b,c]`; R3 — the committed reference is pairwise
  nondominated (executable `is_nondominated_set` over all 1600 points), with
  the continuous record-high proof documented in the test.
* **pymoo secondary comparison.** pymoo's DTLZ7 `pareto_front()` (9409 points,
  `f3 in [2.614037, 6.0]`) has 97.95% of points inside the certified region; the
  ~2% outside are pymoo's raw-grid gap samples, i.e. exactly the dominated
  points this certification removes. pymoo is used as corroboration only, never
  as the membership proof.
* **Effect.** This certification changed ONLY the DTLZ7 IGD+ values (HV uses a
  fixed reference point; DTLZ1/DTLZ2 PF samplers are unaffected). DTLZ7 oracle
  median IGD+ moved 0.0419 (raw grid) -> 0.0911 (filtered) -> 0.0866
  (certified); our honest-loop max IGD+ moved 0.1082 -> 0.1651 -> 0.1521. The
  component-level IGD+ fixture (`component_oracle.json`) uses hand-seeded fixed
  point pairs and does NOT sample a DTLZ PF, so it was not regenerated.
* DTLZ1 (simplex `sum f = 0.5`) and DTLZ2 (positive unit sphere) samplers are
  nondominated by construction (equal sums / equal norms), stated in their
  docstrings; no filter or certification is needed.

**Oracle bug found and fixed during this ticket:** the first in-script
`_union_area_2d` used the maximum active `z` instead of the correct union lower
boundary (minimum active `z`); the affected fixtures were regenerated. The fixed
`hv3d` now agrees with DEAP and brute force (0.531 for the component test
front).

## Operator / default differences (DoD item 6)

The crate's frozen `sbx_crossover` follows the DEAP lineage: **every differing
gene** is crossed and children are assigned deterministically. pymoo's default
NSGA3 SBX uses a **per-variable crossover probability 0.5** and a **binomial
(prob 0.5) child exchange**. The crate's unit-tested SBX is therefore validated
exactly against DEAP in `conformance_components.rs`, while the quality loop in
`support/harness.rs` uses a **pymoo-faithful SBX emulation** (`pymoo_sbx`) so
the matched-budget comparison is fair. The crate operator is untouched.

Evidence (DTLZ2 seed 0, median `g` of the survivors):

| mating | gen 10 | gen 99 |
|---|---|---|
| pymoo NSGA3 oracle | 0.179 | 0.0034 |
| pymoo-faithful SBX emulation | 0.231 | 0.003 |
| crate DEAP-lineage SBX | 0.562 | 0.158 |
| polynomial mutation only | 0.454 | 0.082 |

**Selection identity.** Feeding pymoo's captured generation-10 combined
population (184x3) to the crate selection reproduces pymoo's survivor set
exactly: Jaccard = 1.000 (92/92). The selection kernel is faithful; the
matched-budget gap is an operator-default difference, not an algorithm defect.

## Per-problem quality bands

| Problem | Band | Result (ours median vs oracle median) |
|---|---|---|
| DTLZ2 | strict: ratio >= 0.95, IGD+ <= 2x median | **PASS** (0.7354 vs 0.7368, ratio 0.9981; IGD+ 0.0271 vs band 0.0512) |
| DTLZ7 | applied: ratio >= 0.85, IGD+ <= 2x + 0.10 | **PASS** applied; HV ratio 0.9300 (strict HV 5% not met, see below); IGD+ 0.1521 vs applied band 0.2731 (strict IGD+ band 0.1731 also met) |
| DTLZ1 | applied: degenerate absolute (`ours >= oracle - 0.05`, IGD+ <= 2x + 1.0) | **PASS**; strict relative band not meaningful (see below) |

* **DTLZ1 degeneracy (honesty note).** At the matched 9200 budget neither
  implementation converges: pymoo's worst-seed final DTLZ1 population has a
  maximum `sum(f) = 58.46` after 9200 evals (the PF is the `sum(f) = 0.5`
  hyperplane) and needs ~46000 evals to reach it. The oracle
  median HV is ~0.0155 (near zero), so a relative 5% band is not informative;
  an absolute floor is used and the raw medians are reported.
* **DTLZ7 gap.** The honest loop reaches median HV ratio 0.9300 (median ~7%
  below pymoo) on the disconnected front; this is attributed to deterministic
  (vs pymoo random) niche tie-breaks plus 8-seed spread, and is *reported*, not
  hidden.
* DTLZ2, the smooth/connected reference case, passes the contract's strict 5% /
  2x bands. `conformance_quality.rs` asserts the strict band for DTLZ2 and the
  pre-registered per-problem bands for the others, always printing both the
  strict and applied verdicts.

## Corruption-sensitivity findings

| Mutant | DTLZ2 strict gate | Note |
|---|---|---|
| dominance sign flip | **FAIL** (HV ratio 0.0000) | as required |
| always-first partial-front selection | **FAIL** (HV ratio 0.8650) | as required |
| normalization bypass | no-op on DTLZ2 (ratio 0.9991) | DTLZ2 objectives are near the unit simplex, so ideal translation + intercept scaling is close to identity |
| normalization bypass on DTLZ7 | **FAIL** (HV ratio 0.7041) | DTLZ7 raw scales differ strongly (`f3` ~[2.6, 6]); gate sensitive where normalization matters |

## Literature references

* Deb & Jain, "An Evolutionary Many-Objective Optimization Algorithm Using
  Reference-Point-Based Non-dominated Sorting Approach, Part I", *IEEE TEC*,
  2014 (NSGA-III).
* Deb, Thiele, Laumanns & Zitzler, "Scalable Multi-Objective Optimization Test
  Problems", *CEC*, 2002 (DTLZ1/DTLZ2).
* Deb, "Multi-Objective Optimization using Evolutionary Algorithms", Wiley,
  2001 (DTLZ7).
* While, Hingston, Barone & Huband, "A Faster Algorithm for Calculating
  Hypervolume", *IEEE TEC*, 2006.
* Ishibuchi, Masuda, Tanigaki & Nojima, "Modified Distance Calculation in
  Generational Distance and Inverted Generational Distance", *EMO*, 2015.
* DEAP 1.4 `deap.tools` / `deap.tools._hypervolume`; pymoo 0.6.2
  (`pymoo.algorithms.moo.nsga3`, `pymoo.util.normalization`,
  `pymoo.util.ref_dirs`).
