# Literature Checklist — NSGA-III mechanics mapped to MAMUT code paths

Each mechanic of Deb & Jain (2014) NSGA-III is mapped to the MAMUT source path
that implements it, with the independent oracle used to validate it.
`src/` is `crates/mamut-nsga3/src/`.

| # | Mechanic (Deb & Jain 2014) | MAMUT code path | Independent oracle |
|---|---|---|---|
| 1 | Reference directions (das-dennis simplex lattice) | `src/reference_points.rs:19` (`uniform_reference_points`), `:40` (`gen_refs`) | pymoo `get_reference_directions("das-dennis", ...)` |
| 2 | Ideal point (per-objective minimum) | `src/normalize.rs:54` | pymoo `util.normalization.find_ideal` |
| 3 | Extreme points via ASF | `src/normalize.rs:105` (`find_extreme_points`), `:126` (`compute_asf`) | pymoo `util.normalization.get_extreme_points_c` |
| 4 | Hyperplane intercepts (`solve M x = 1`) | `src/normalize.rs:140` (`compute_intercepts`), back-substitution `:182` | pymoo `get_nadir_point` intercept solve (`pymoo/util/normalization.py` 269-274) |
| 5 | Degenerate intercept fallback (per-axis max) | `src/normalize.rs:203` (`fallback_intercepts`), branches `:167`, `:195` | pymoo `get_nadir_point` worst-of-front fallback |
| 6 | Objective normalization (translate + intercept divide) | `src/normalize.rs:64` (translate), `:82` (divide) | pymoo `associate_to_niches` (`N = (F - utopian)/denom`) |
| 7 | Association (perpendicular distance to reference line) | `src/niching.rs:31` (`perp_distance`), `:92` (`associate`) | pymoo `calc_perpendicular_distance` |
| 8 | Niche counts | `src/niching.rs:154` (`build_niche_counts`) | pymoo `calc_niche_count` |
| 9 | Partial-front selection (least niche count, then min distance) | `src/niching.rs:176` (`select_from_partial_front`) | pymoo `niching` |
| 10 | Environmental selection (front fill -> split -> normalize -> associate -> niche) | `src/selection.rs:44` (`select_environmental`) | survivor-set identity vs pymoo: Jaccard = 1.000 (92/92) in `provenance.md` |
| 11 | Fast nondominated sort (Deb 2002) | `src/nondom_sort.rs:90` (`fast_nondominated_sort`), dominance `:27`, front extraction `:154`, set purity `:178` | DEAP `tools.sortNondominated` |
| 12 | Variation operators assumption (SBX + polynomial mutation) | `src/operators.rs:105` (`sbx_crossover`), `:192` (`polynomial_mutation`), `:275` (`var_or`), `:89` (`adaptive_eta_value`), `:348` (`init_population`) | DEAP `cxSimulatedBinaryBounded` / `mutPolynomialBounded` (exact) |
| 13 | Hypervolume quality metric (While et al. 2006) | `src/metrics.rs:97` (`hypervolume`), `:137` (`hypervolume_slices`) | DEAP compiled-core exact 3D HV; independent in-script exact HV |
| 14 | IGD+ quality metric (Ishibuchi et al. 2015) | `src/metrics.rs:188` (`igd_plus`) | independent in-script numpy IGD+ |
| 15 | DTLZ test problems (Deb & Thiele 1999 / Deb 2001) | `tests/support/dtlz.rs` (`dtlz1`, `dtlz2`, `dtlz7`) | pymoo DTLZ1/2/7 (max abs diff <= 1.5e-14) |

## Assumptions and documented divergences

* **Variation composition is a caller choice.** The crate's `var_or`
  (`src/operators.rs:275`) mirrors DEAP `algorithms.varOr`
  (crossover-XOR-mutation) and its `sbx_crossover` crosses every differing gene
  with deterministic child assignment. pymoo NSGA3 applies SBX (per-variable
  `prob_var=0.5`, `prob_bin=0.5` exchange) then polynomial mutation to the same
  offspring. The quality loop emulates the pymoo defaults in the test harness
  only; the crate operators are untouched. See `provenance.md`.
* **Frozen RNG only.** All stochasticity flows through
  `mamut_math_core::rng::RngPort` / `MamutRng`; the conformance component tests
  use a scripted `RngPort` double so DEAP's uniform stream can be replayed.
* **Selection orientation.** All objectives are minimized on every kernel;
  maximization callers negate their objective columns at the boundary.

## References

* Deb & Jain, *IEEE TEC* 18(4), 2014 — NSGA-III.
* Deb, Thiele, Laumanns & Zitzler, *CEC*, 2002 — DTLZ1/DTLZ2.
* Deb, *Multi-Objective Optimization using Evolutionary Algorithms*, Wiley,
  2001 — DTLZ7.
* While, Hingston, Barone & Huband, *IEEE TEC* 10(5), 2006 — hypervolume.
* Ishibuchi, Masuda, Tanigaki & Nojima, *EMO*, 2015 — IGD+.
* DEAP 1.4; pymoo 0.6.2.
