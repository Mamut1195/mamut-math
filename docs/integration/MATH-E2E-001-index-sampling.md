# MATH-E2E-001 T2a — generic index-sampling port (`RngPort::next_index`)

Status: T2a Math-side implementation. This note covers **only** the additive
generic index-sampling port and its native-default parity evidence. It does
**not** implement the consumer adapter, the new exact Math Git pin, or any
final E2E / `MATH_VERIFY` / L3 claim; those remain T2b and the parent-owned
gates.

## Scope and authorization

Human authorization `authorize-minimal-index-port-and-exact-pin` permits the
minimum additive generic Math index-sampling port plus, later, a new exact Math
pin for E2E-001. T2a is the Math half: public trait hook, native-default parity
tests and this note. Native Math default behavior is unchanged; the legacy
consumer override belongs to T2b in a thin adapter.

## Public API

```rust
pub trait RngPort {
    // ... existing methods unchanged ...
    fn next_index(&mut self, upper_exclusive: NonZeroUsize) -> usize;
}
```

- Added to the existing object-safe `RngPort` trait as a **provided** method
  (`crates/mamut-math-core/src/rng.rs`). Existing implementors keep compiling
  without edits and receive the default.
- `upper_exclusive: core::num::NonZeroUsize` is standard core/std; no
  third-party API enters the public contract. The `0..n` with `n >= 1` domain
  is expressed in the type.
- The default body is the **exact** pre-port multiply-high-with-rejection
  (Lemire) sampler, transferred verbatim: identical values, identical rejection
  loop, identical `next_u64` consumption for every valid bound. An
  `upper_exclusive` of `1` still consumes one raw draw — the trivial bound is
  not short-circuited, so the historical stream position of the NSGA-III
  operators is preserved.
- Overriding implementors substitute their own distribution and **must** return
  a value in `0..upper_exclusive`. The trait is unchanged in algorithm
  identity: no keystream, seed expansion, child-derivation or metadata byte
  changes, so `RNG_ALGORITHM_ID`/`RNG_ALGORITHM_VERSION` stay `mamut.chacha20-std`
  / `1`.

## Dispatch

`crates/mamut-nsga3/src/internal.rs::gen_index(rng, n)` dispatches positive
bounds through the port and preserves the historical zero-bound behavior:

```rust
pub(crate) fn gen_index(rng: &mut dyn RngPort, n: usize) -> usize {
    debug_assert!(n >= 1, "gen_index requires n >= 1");
    match NonZeroUsize::new(n) {
        Some(upper) => rng.next_index(upper),
        None => {
            let _ = rng.next_u64();
            0
        }
    }
}
```

The operators therefore hold **no private sampling body** for `n >= 1`; both
call sites route through the port:

- `operators::var_or` — parent selection in all three branches (crossover,
  mutation, reproduction);
- `operators::init_population` — `Discrete` and `Categorical` index draws.

### Zero bound (historical behavior preserved)

- Debug builds: `debug_assert!(n >= 1)` panics with
  `gen_index requires n >= 1` **before** any raw draw.
- Release builds: `n == 0` consumes exactly **one** raw `u64` and returns `0`,
  without evaluating `2^64 % 0` — byte-for-byte the pre-port private body in a
  debug-assertions-disabled build. (The earlier statement that the old release
  body panicked with a modulo-by-zero was wrong.)

A consumer can override index sampling without re-implementing any operator.

### Pre-existing 32-bit limitation (unchanged, out of scope)

The `u64 -> usize` cast in `operators::sample_gene`
(`crates/mamut-nsga3/src/operators.rs`) is untouched: on a 32-bit target a
categorical cardinality of `2^32` still truncates to `0` and reaches the zero
path above. That pre-existing cast/range behavior is neither fixed nor claimed
verified here; no 32-bit runner or MSRV toolchain was executed.

## Explicit non-changes

- Float formula, `Seed` expansion/fingerprint, child-stream derivation and
  metadata bytes.
- Public existing function signatures, NSGA operators/selection/session
  mathematics, policy and default trajectories.
- No new sampling-policy framework, dependency, manifest, lock, version or tag
  change; no production Python; no domain logic.

## Evidence (`crates/*/tests/index_sampling_port.rs`)

`mamut-math-core/tests/index_sampling_port.rs` (10 tests, 6 on a 32-bit
target; wide bounds and 64-bit-only pinned values are gated by
`#[cfg(target_pointer_width = "64")]`):

- **Pre-port baseline capture** — the pinned `NATIVE_SEED42_BASELINE` values
  were observed through the public `MamutRng::next_u64` stream and the
  documented sampler *before* the port method existed, then asserted against
  `next_index` on a fresh `Seed::from_u64(42)` root. The first 20 draws are
  width-independent; the four wide bounds (`2^32`, `2^32+1`, `2^63`,
  `usize::MAX`) and their pinned indices are the 64-bit continuation of the
  same stream.
- **Crafted words / hand-computed thresholds** — rejection (`2^64 % 3 = 1`,
  `% 6 = 4`, `% 10 = 6`, `% 1000 = 616`, `% (2^32-1) = 1`,
  `% (2^64-1) = 1`), power-of-two bounds (`2`, `2^32`, `2^63`) with no
  rejection zone, non-power top buckets, and the `u32::MAX` / `usize::MAX`
  width boundaries.
- **Draw-count parity** — the native default matches an independent test-side
  transcription of the documented sampler over the same seed-42 stream, with
  equal cumulative raw `u64` draw counts per bound.
- **Existing-implementer compatibility** — `WordPort` and `CountingPort`
  implement only the pre-port methods, compile unchanged, and get correct
  default sampling through `&mut dyn RngPort`.

`mamut-nsga3/tests/index_sampling_port.rs` (7 tests):
- **Override dispatch** — a port overriding `next_index` with scripted indices
  drives `var_or` and `init_population`; the test asserts the scripted parent
  selections, one index draw per selection, and **zero** raw-word fallback
  draws. The override port returns raw word `1`, which a private multiply-high
  body would deterministically map to index `0`, so the assertion cannot pass
  through a private body.
- **All three `var_or` branches** — constant-unit runs for crossover (2 index
  draws/iteration), mutation (1) and reproduction (1), each with zero fallback
  draws.
- **Native default pins through the public operators** — pre-port index streams
  for `n = 4`, `n = 3` and `n = 1` (`var_or`) and for categorical/discrete
  `init_population`, with exact unit/raw draw counts. The `n = 1` case shows the
  single-parent draw is still consumed through the real caller path.
- **Private zero-bound regression** —
  `internal::tests::gen_index_zero_preserves_historical_behavior` (a unit test
  inside `crates/mamut-nsga3/src/internal.rs`, not in this integration file)
  drives `gen_index` with a counting port that implements only the pre-port
  methods: debug asserts the panic precedes any raw draw, release asserts a
  `0` result after exactly one raw `u64` draw. It is exercised with
  `cargo test -p mamut-nsga3 --lib --release ... -- --exact` and the matching
  debug run.

## Limits and risks

- This evidence is the **native** default equivalence only. It is distinct from
  legacy consumer adapter equivalence: the legacy root seed uses
  `ChaCha20Rng::seed_from_u64` (PCG32), so legacy and native `Seed::from_u64`
  streams differ. The known ~1000-draw index mismatch against the legacy
  sampler is a distribution-compatibility finding, not a native correctness
  bug, and is resolved by the T2b override.
- No consumer sampler, thin kernel adapter or new exact Math pin is implemented
  here; the consumer remains on its legacy path until T2b.
- 32-bit coverage is compile-time gating only: wide tests/constants are cfg'd
  out on non-64-bit targets so the files still compile, but no 32-bit test run
  or MSRV toolchain execution was performed, and the pre-existing 32-bit
  `u64 -> usize` cardinality cast is unchanged.
- Independent `MATH_VERIFY` and L3 acceptance remain T4 concerns.