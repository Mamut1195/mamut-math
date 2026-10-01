# MATH-VV-001 — NSGA-III parity and quality regression suite

This document maps the VV-001 regression suite: its five evidence categories
(A..E), the fast and slow lanes, the machine-readable reports, and what each
gate protects. It is the human-readable companion to the executable tests under
`crates/mamut-nsga3/tests/` and the fixture
`crates/mamut-nsga3/tests/fixtures/regression/baseline_quality.json`.

> **Legacy parity vs conformance.** Category A is a *same-build determinism*
> lane, not a correctness proof. The verbatim semantic distinction is:
>
> **legacy parity means we preserve what we decided to preserve, NOT mathematical authority**
>
> The mathematical expected values are the external fixtures: the NSGA-002 ported
> goldens (frozen Hydraulics SHA `6db4862d7a4035de8d494cdc62607b12b4edb103`) and
> the NSGA-004 oracle JSONs (`pymoo_dtlz_oracle.json`, `component_oracle.json`).
> Category A answers "did the code change between builds?"; the conformance
> categories (B/C) own "is the algorithm right against an independent oracle?".

## Categories A..E

| Tag | Category | Owns | Executable evidence |
|---|---|---|---|
| **Category A** | Legacy parity | Same-build bit-exact seed replay + frozen provenance | `tests/regression_legacy_parity.rs` (reuses NSGA-002 `ported_golden.rs`/`scripted_parity.rs`/`session.rs`) |
| **Category B** | Component fixtures parity old↔new | Component kernels vs DEAP/independent oracles; certified DTLZ7 reference | existing `conformance_components.rs`, `ported_golden.rs`; slow: `certified_dtlz7_checks()` |
| **Category C** | Stochastic quality from the baseline distribution | HV/IGD+ distribution vs pre-registered oracle bands | `tests/regression_quality_fast.rs` (fast), `tests/regression_quality_slow.rs` (slow) |
| **Category D** | Machine-readable regression report | `VV001RegressionReport/1` schema, fixture SHA-256 registry | `tests/regression_report.rs` |
| **Category E** | Corruption / sensitivity | Gate sensitivity to corrupted components and to flipped pins | existing `conformance_corruption.rs`; `tests/regression_corruption_sensitivity.rs` |

### Category A — self-generated hashes are determinism pins only

`LEGACY_EXPECTED_HASHES` in `tests/support/regression.rs` are SHA-256 digests of a
canonical byte trace produced by the crate itself
(`init_population -> evaluate -> sbx_crossover -> polynomial_mutation ->
select_environmental` under the frozen `MamutRng`). Per ticket note (i), those
expected values are **self-generated same-build determinism regression pins**.
They are acceptable for category A *only* in that role. Any claim of mathematical
validity continues to trace to the external fixtures listed above. The
seed set is the ticket slice `0..=3` plus the frozen NSGA-002 seeds
`{1, 7, 42, 99, 100, 123}`.

## Lanes and commands

### FAST lane (every commit / PR)

```text
cargo test --workspace
```

Runs all existing suites plus:

* `regression_legacy_parity.rs` (Category A);
* `regression_quality_fast.rs` (Category C fast slice: DTLZ2 `seeds 0..=1`,
  exact `1e-12` same-build replay and band check);
* `regression_report.rs` (Category D; writes `target/vv001_fast_report.json`);
* `regression_corruption_sensitivity.rs` (Category E).

### SLOW lane (explicit, not in the default run)

```text
cargo test -p mamut-nsga3 --test regression_quality_slow -- --ignored
```

The slow test is annotated `#[ignore = "..."]`. `#[ignore]` is the mechanism by
which the default `cargo test --workspace` **compiles but does not execute** it
(cargo reports it as `ignored`); this is the first `#[ignore]` use in the
repository, chosen precisely so the slow lane cannot silently join the default
run. The slow lane runs the full frozen `seeds 0..=7` DTLZ1/2/7 corpus against
the pymoo oracle bands, the DTLZ2 strict band, the NSGA-004 certified DTLZ7
reference at full `n_per_interval` resolution, and writes
`target/vv001_slow_report.json`. The slow binary also hosts the ignored offline
entry point `regenerate_baseline_quality_fixture`, which recomputes the
committed baseline fixture (sensitivity ledger included) byte-for-byte from the
generator; it is never part of any lane.

## Reports

| Lane | Path | Schema |
|---|---|---|
| Fast | `target/vv001_fast_report.json` | `VV001RegressionReport/1` (test-written) |
| Slow | `target/vv001_slow_report.json` | `VV001RegressionReport/1` (test-written) |

Each report records `code_sha` (`MAMUT_GIT_SHA`, else the committed base SHA),
`algorithm` (`mamut-nsga3` version), `rng` (`mamut.chacha20-std` v1),
`corpus`, `oracle_provenance` (pymoo/DEAP/numpy/python versions,
`mamut_crate_used = false`, DTLZ7 PF certification), `gates` (each with category,
lane, status, thresholds, observed, rationale) and `artifact_hashes` (the
fixture SHA-256 registry). It reuses the MATH-EVD-001 evidence types
(`ArtifactHash`, `RngSpec`, `SolverVersion`).

## What each gate protects

* **A.legacy-seed-replay** — any change to the core RNG, init, operators or
  selection changes the trace digest; catches accidental behavioral drift.
* **A.legacy-provenance** — the committed provenance still names the frozen SHA
  and the `legacy-parity` label.
* **B.component-fixtures-parity** — existing NSGA-002/004 component parity
  (DEAP/independent oracles) stays green; **B.certified-reference** re-checks the
  certified DTLZ7 front at full resolution (R1/R2/R3).
* **C.baseline-exact-replay / C.full-corpus-bands** — same-build per-seed HV/IGD+
  equals the committed baseline (`1e-12`) and the pre-registered per-problem
  bands (DTLZ2 strict 5%/2×, DTLZ1 degenerate-absolute, DTLZ7 applied 0.85) still
  pass. Thresholds are reused from NSGA-004 and are **not retuned**.
* **D.report-schema** — the registry hashes and the A..E structure stay complete
  and machine-checkable.
* **E.corruption-sensitivity** — the corruption suite still fails the gate for
  mutated components, and two real one-off local mutations each failed their
  lane before being restored byte-exact: (category A) a flipped digit of
  `LEGACY_EXPECTED_HASHES[0]` failed `seed_replay_matches_committed_hashes`;
  (category C) a flipped 1e-9 digit of `problems.dtlz2.per_seed[0].hv` in the
  baseline fixture failed `dtlz2_fast_slice_matches_committed_exactly` and only
  that test. **Disclosed:** an earlier probe that mutated the fixture
  `applied_band.hv_min_ratio` did not detect corruption and is withdrawn —
  `applied_band` fixture rows are registry duplicates and are not currently the
  source of the `regression_quality_fast` band gate, which reads the
  pre-registered constants in `applied_band_row` (the report-path
  `fast_quality_status` does read the fixture rows; the two are kept equal).
  Wiring `applied_band_row` to the fixture is a follow-up outside this ticket.

## Determinism and feasibility caveats

* The distributional bands are **distributional**: a different RNG stream and
  operator order make per-seed identity vs pymoo impossible and were never
  claimed.
* The per-seed HV/IGD+ values are honest only for **admissibility-filtered**
  lanes; the quality loop evaluates the unconstrained DTLZ surface. Infeasibility
  mechanics are covered separately in `conformance_quality.rs`.
