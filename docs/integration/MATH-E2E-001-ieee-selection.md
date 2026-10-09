# MATH-E2E-001 M2 — shared IEEE-f64 environmental-selection route

Status: M2 Math-side **selection** implementation (`as authored` candidate). This
note covers the additive `mamut_nsga3::ieee_selection` raw-`f64` entry and the
shared finite-policy kernels it drives. It does **not** close the M2 work unit,
implement the consumer thin adapter, establish the exact Math Git pin, or make
any final E2E / `MATH_VERIFY` / L3 claim. The M1 variation note
(`docs/integration/MATH-E2E-001-ieee-compatibility.md`) is frozen and read-only
here.

## Quick path

1. Use `mamut_nsga3::select_environmental` for the checked contract: the fitness
   matrix, reference points and normalized intermediates are finite-validated.
2. Use `mamut_nsga3::ieee_selection::select_environmental` only when a raw `f64`
   fitness matrix may hold `NaN`/`±Inf` and the caller owns the finite
   projection.
3. Validate downstream: the raw route returns selected indices only and promises
   **no** engineering validity.

## Scope and authorization

Human authorization `authorize-bounded-shared-ieee-kernels-and-exact-pin`
(memory 10865) permits one shared IEEE route for the active selection chain with
the checked validation / error / early-return / RNG timing preserved. Option B
(`authorize-m2-format-prose-note-and-checks`, memory 11141) authorizes only the
five formatting hunks, the derivation-comment repair and this note. The scope is
Math-only; "implementation" means repository/library source, not a deployed
system.

The bounded M2 candidate touches nine paths:

- `crates/mamut-nsga3/src/nondom_sort.rs`
- `crates/mamut-nsga3/src/normalize.rs`
- `crates/mamut-nsga3/src/niching.rs`
- `crates/mamut-nsga3/src/selection.rs`
- `crates/mamut-nsga3/src/ieee_selection.rs` (new: raw entry + private policy)
- `crates/mamut-nsga3/src/lib.rs` (additive module export only)
- `crates/mamut-nsga3/tests/ieee_selection_contract.rs` (new)
- `crates/mamut-nsga3/tests/architecture.rs` (additive signature lock only)
- `docs/integration/MATH-E2E-001-ieee-selection.md` (this note, new)

No `internal.rs`, core/RNG/version/pin, metrics/config/session/reference_points,
manifest/lock, fixture/oracle, consumer, Git/config/cache change.

## Public API

```rust
pub mod ieee_selection;

pub fn ieee_selection::select_environmental(
    fitnesses: &[Vec<f64>],
    n: usize,
    ref_points: &[Vec<f64>],
    rng: &mut dyn RngPort,
) -> Result<Vec<usize>, MathError>;
```

Exactly one raw environmental-selection seam: no blanket raw-sort / raw-normalize
/ raw-associate / raw-perp-distance wrappers, no new raw helper exports, no
third-party stable type and no public generic. The checked root re-exports
(`select_environmental`, `normalize`, `associate`, `perp_distance`, ...) are
unchanged.

## Shared single body

A crate-private `ieee_selection::FinitePolicy` (`Checked`/`Raw`) abstracts only
whether the finiteness gates run. Each stage keeps one arithmetic body:

| Stage | Shared kernel | Checked wrapper | Raw |
|---|---|---|---|
| Nondominated sort | `nondom_sort::fast_nondominated_sort_kernel` | `nondom_sort::fast_nondominated_sort` | same kernel, `Raw` |
| Normalization | `normalize::normalize_kernel` | `normalize::normalize` | same kernel, `Raw` |
| Association | `niching::associate_kernel` | `niching::associate` | same kernel, `Raw` |
| Perp distance | `niching::perp_distance_kernel` | `niching::perp_distance` | same kernel, `Raw` |
| Env. selection | `selection::select_environmental_kernel` | `selection::select_environmental` | `ieee_selection::select_environmental` |

The policy and kernels are `pub(crate)`/private: no second legacy algorithm body
is copied into the repository.

## Preserved exactly

- Checked public signatures, guards, errors and defaults are unchanged. Checked
  policy calls `internal::validate_fitness_matrix` verbatim.
- Validation order: empty/arity, then finiteness, then Deb front order, front
  concatenation, normalization, association and niche selection. Structural
  errors (`EmptyCollection`, `DimensionMismatch`) keep their kinds and order.
- Before matrix validation, `pop == 0` or `n == 0` returns an empty vector,
  and `n >= pop` returns `0..pop`. Exact whole-front completion
  (`selected.len() == n`) returns **after** matrix validation and nondominated
  sorting, but before normalization and association (a whole
  front fits when `selected + front <= n`). The RNG is never consumed (`rng` is
  reserved and unused, as in the checked route).
- The Deb insertion-order front tie-break, the smallest-index niche tie-break and
  the deterministic ordering are unchanged.

## The only difference

Raw mode skips **only** the finiteness gates at their original sites:

- the matrix finiteness inside `internal::validate_fitness_matrix` (the
  structural `M >= 1` and uniform-arity checks still run, via
  `validate_matrix_structure`);
- the normalized-matrix and reference-entry finiteness in `associate`;
- the point/reference finiteness in `perp_distance`;
- the intercept solve-component finiteness branch in `compute_intercepts`, whose
  condition becomes `x[j] <= 0.0 || (checks_finite && !x[j].is_finite())`: raw
  keeps the `x[j] <= 0.0` fallback but lets a non-finite solve component flow.

Every other arithmetic guard is preserved in **both** modes: the
`|pivot| < 1e-12` degenerate-system fallback, the `||r||² < 1e-30`
degenerate-reference guard, the `<= 0` intercept fallback and the `.max(0.0)`
clamp. This is not a generic "raw skips all validation" claim.

## Not promised

- **No engineering validity.** `NaN` ordering, `NaN`-in/`NaN`-out propagation,
  objective sanity and any finite projection are the caller's responsibility.
  The raw route is a low-level compatibility surface, not a design-request
  guarantee.
- **No sanitization policy.** No sentinel substitution, alias repair, type
  weakening, new seed or new RNG policy is introduced.
- **No selection policy change.** The raw route does not change reference
  lattices, niche eligibility or survivor ordering.

## Evidence status (this candidate)

- Historical observed: missing-API RED (`E0432` unresolved
  `mamut_nsga3::ieee_selection`); the first focused GREEN failed 2/8 because the
  writer's hand-derived pins were wrong; those constants were later corrected to
  `[0,3,1]` / `[0,2]` under a bounded human grant on the independently verified
  committed-`ab4d8ed` base.
- Pre-edit (06) candidate: commands 1–6 passed (`ieee_selection_contract`
  debug 8/8 and release 8/8, `ported_golden` 49, `conformance_components` 7,
  `architecture` 5, ordinary `ieee_variation_contract` 13); command 7
  `cargo fmt --all -- --check` failed with exactly the five hunks now applied;
  commands 8–13 were **not run**.
- The **new B candidate verification is pending**: parent ASSESS plus a separate
  thirteen-command `gentle-ai-verify` pass. This note is `as authored` and does
  not claim a new GREEN, native approval, final `MATH_VERIFY`, L3 or E2E pass.
- Machine artifacts are external and additive:
  `C:/Users/Jonnathan/AppData/Local/Mamut/proofs/MATH-E2E-001/M2-writer-20261008-k7q2/`
  (`00`–`06` historical, `07-b-writer-*` writer evidence). Bind later
  verification to those locators, not to a hash of this note.

## Unrun limits

Independent final engineering `MATH_VERIFY`, the strong final L3, the E2E
singular-kernel gate, the consumer adapter, parity artifacts and the exact Math
Git pin/lock-cache remain **unrun** here and are parent/verifier-owned. Raw
synthetic IEEE controls do **not** establish actual valid `DesignRequest`
reachability. MSRV and 32-bit execution remain NOT RUN.