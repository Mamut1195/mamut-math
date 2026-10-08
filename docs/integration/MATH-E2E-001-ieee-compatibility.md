# MATH-E2E-001 M1 — shared IEEE-f64 variation route

Status: M1 Math-side **variation-only** implementation. This note covers the
additive `mamut_nsga3::ieee` raw-`f64` entry and its shared arithmetic body. It
does **not** implement M2 selection/normalization/niching, the consumer thin
adapter, the new exact Math Git pin, or any final E2E / `MATH_VERIFY` / L3 claim.

## Quick path

1. Use `crates/mamut-nsga3/src/operators.rs` for the checked contract: genes
   and cached fitness are finite-validated.
2. Use `mamut_nsga3::ieee` only when an intermediate variation gene may be
   `NaN`/`±Inf` and the caller owns the finite projection.
3. Validate downstream: the raw route stores what the arithmetic produces and
   promises **no** engineering validity.

## Scope and authorization

Human authorization `authorize-bounded-shared-ieee-kernels-and-exact-pin`
(memory 10865) permits one shared IEEE route for the active variation path with
the typed validation/error/RNG timing preserved. M1 is the Math variation half;
selection belongs to M2 and the inclusive-init consumer adapter belongs to the
future thin consumer unit. Nothing is deployed in production: "implementation"
here means repository/library source.

Only the five M1 paths were touched: `crates/mamut-nsga3/src/operators.rs`,
`crates/mamut-nsga3/src/ieee.rs` (new), `crates/mamut-nsga3/src/lib.rs`,
`crates/mamut-nsga3/tests/ieee_variation_contract.rs` (new) and this note. No
core, `internal.rs`, selection, config, init, manifest, lock, fixture, oracle or
consumer change. `mamut_math_core::ValidFloat`/`Candidate` are unchanged.

## Public API

```rust
pub mod ieee;

pub struct ieee::Individual {
    pub genes: Vec<f64>,
    pub fitness: Option<Vec<f64>>,
    pub feasibility: Option<mamut_math_core::Feasibility>,
}

pub fn ieee::sbx_crossover(...) -> Result<(), MathError>;
pub fn ieee::polynomial_mutation(...) -> Result<(), MathError>;
pub fn ieee::var_or(...) -> Result<Vec<Individual>, MathError>;
```

- MAMUT-owned types only: no third-party type crosses the boundary, and the
  feasibility cache is the MAMUT `Feasibility`, not a new classification.
- `ieee::Individual` is a **separate** type from `operators::Individual`; the
  existing typed type and every public typed signature, guard, error and RNG
  consumption are unchanged.

## Shared single body

One crate-private strategy, `operators::KernelIndividual`, abstracts only how a
kernel reads/writes genes and whether writing validates finiteness. Both the
typed `Individual` (validating through `internal::candidate_from_f64`) and the
raw `ieee::Individual` (plain assignment) implement it, so each operator has a
**single** arithmetic body:

| Operator | Shared kernel | Typed wrapper | Raw wrapper |
|---|---|---|---|
| SBX | `operators::sbx_crossover_kernel` | `operators::sbx_crossover` | `ieee::sbx_crossover` |
| Polynomial mutation | `operators::polynomial_mutation_kernel` | `operators::polynomial_mutation` | `ieee::polynomial_mutation` |
| `varOr` | `operators::var_or_kernel` | `operators::var_or` | `ieee::var_or` |

The trait is `pub(crate)`/private: no public generic, no new public signature,
and no duplicated legacy algorithm body is copied into the repository.

## Preserved exactly

- Structural guards run **before** any RNG draw: SBX arity then
  `EmptyCollection`; mutation arity then `EmptyCollection`; `varOr` validates
  `cxpb + mutpb > 1` (`InvertedBounds`) then non-empty population/variables with
  the checked crossover-1/mutation-1 timing.
- SBX identical-gene skip draws nothing; the spread math constants, `powf`
  order and draw count are unchanged.
- Both SBX children are converted and invalidated at their original sites
  inside the call (even when an odd `lambda` never keeps the second child), and
  the first assignment persists if a later one fails.
- `var_or` keeps **both** children when there is room (the documented frozen
  divergence from DEAP), reproduction clones the exact parent, and mutation
  invalidates cached fitness/feasibility only through the existing gate.
- Variable bounds/continuous/integer/discrete/categorical logic is unchanged;
  integer genes are left as raw floats (no rounding, no
  `validate_against`/integrality check).

## The only difference

Raw mode skips **only** the finite-value construction/validation of genes: an
arithmetic result of `NaN`/`±Inf` is stored instead of returned as
`ErrorKind::InvalidNotFinite`. Both routes reach that point after the identical
draw sequence.

Observed pins on the controlled corpus (debug = release, x86-64):

- Overflowed SBX spread (`0 * Inf`, parents `±f64::MAX`, `u = 0`): raw stores
  `0xfff8_0000_0000_0000` on both children; typed returns
  `InvalidNotFinite`, same draw count.
- Mutation with a raw input `NaN`: the input payload
  `0x7ff8_0000_0000_0000` is preserved (typed cannot represent that input).
- Digitally finite parents under `±f64::MAX` bounds: raw and typed produce
  identical `f64` bits.

## Not promised

- **No engineering validity.** `NaN` sorting, `NaN`-in/`NaN`-out propagation,
  objective/constraint sanity and downstream finite projection are the caller's
  responsibility. The raw route is a low-level compatibility surface, not a
  correctness or design-request guarantee.
- **No sanitization policy.** No `η` capping, sentinel conversion, alias repair,
  type weakening, new seed or new RNG policy is introduced.
- **No selection coverage.** Raw selection/normalization/niching is M2, after
  M1 is verified and reviewed. The consumer's finite `Σ > 1` branch projection
  and any historical debug assertion belong to the future thin adapter, not to
  Math.

## Unrun limits

Independent MATH_VERIFY, the strong final L3, the E2E singular-kernel gate,
consumer parity and the exact Math Git pin remain **unrun** here and are
parent/verifier-owned after M1. MSRV and 32-bit execution remain NOT RUN.