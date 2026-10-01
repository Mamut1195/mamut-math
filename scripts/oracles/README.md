# Offline oracle generation — MAMUT NSGA-III

This directory holds the **offline** Python oracles used to generate the committed
JSON fixtures that the Rust tests consume. No Python runs during `cargo test`: it
is a generation-time-only toolchain (AGENTS.md §4, "Sin Python runtime en
producción"; Python oracles are offline/research only).

## Hard invariants

* **`mamut_crate_used = false`.** The oracle scripts never import, link or call
  the MAMUT Rust crate. Rust-vs-fixture parity therefore cannot be circular; every
  reference value traces to an independent library (pymoo/DEAP) or to a
  closed-form literature Pareto front sampled in-script.
* **DEAP 1.4 ships no NSGA-III and no IGD/IGD+.** The algorithm-level oracle is
  therefore **pymoo `NSGA3`** (Deb & Jain 2014 reference-direction survival);
  **DEAP** supplies only component-level oracles (`deap.tools.sortNondominated`,
  the compiled exact 3D hypervolume, `cxSimulatedBinaryBounded`,
  `mutPolynomialBounded`). IGD+ and the DTLZ Pareto fronts are computed in-script
  with numpy.
* Regenerating a fixture is a **deliberate act**, never a way to make a failing
  test pass. If a Rust conformance/quality gate regresses, fix the Rust side or
  document a bounded tolerance change with rationale — do not silently regenerate
  the oracle.

## Interpreter and pinned versions

| Component | Version |
|---|---|
| Python | 3.13.7 (`C:\Users\Jonnathan\AppData\Local\Programs\Python\Python313\python.exe`) |
| pymoo | 0.6.2 |
| DEAP | 1.4 |
| numpy | 2.2.6 |
| mpmath | 1.3.0 |

## Exact commands

Run from the repository root with the interpreter above:

```powershell
& "C:\Users\Jonnathan\AppData\Local\Programs\Python\Python313\python.exe" scripts/oracles/pymoo_full_oracle.py
& "C:\Users\Jonnathan\AppData\Local\Programs\Python\Python313\python.exe" scripts/oracles/component_oracle.py
```

| Script | Writes |
|---|---|
| `oracle_common.py` | shared helpers (provenance, exact 3D HV, IGD+, DTLZ PF sampling, HV reference points); no output of its own |
| `pymoo_full_oracle.py` | `crates/mamut-nsga3/tests/fixtures/conformance/pymoo_dtlz_oracle.json` |
| `component_oracle.py` | `crates/mamut-nsga3/tests/fixtures/conformance/component_oracle.json` |

Both scripts print a per-seed progress line and stamp a `provenance` block into
the fixture (`generated_utc`, module versions, `evidence_class =
"independent-oracle"`, `mamut_crate_used = false`).

## When to regenerate

Only when a ticket deliberately changes one of:

* the DTLZ problem set, objective count or `n_var` (`k` values);
* the reference-direction method/divisions or population/evaluation budget;
* the seed corpus (`seeds 0..7` by default);
* the Pareto-front construction or the certified DTLZ7 record-high endpoints.

Any regeneration must also update `crates/mamut-nsga3/tests/fixtures/conformance/provenance.md`
and the fixture SHA-256 registry in
`crates/mamut-nsga3/tests/fixtures/regression/baseline_quality.json`
(`regression_report.rs::fixture_registry_hashes_are_current` recomputes it).

## Record-level regression baseline (not a Python oracle)

`crates/mamut-nsga3/tests/fixtures/regression/baseline_quality.json` is generated
by **Rust test support**, not Python:
`tests/support/regression.rs::generate_baseline_quality_json()`. It records the
same-build per-seed HV/IGD+ of the honest DTLZ1/2/7 loop for `seeds 0..=7`, plus
the pre-registered bands and rationale. It is a **same-build determinism anchor
and distributional quality band**, not an independent oracle; the independent
oracle remains `pymoo_dtlz_oracle.json`.

Local Python bytecode (`__pycache__/`) is deliberately git-ignored (`.gitignore`):
this directory is staged at directory level and stray `.pyc` files must never
enter the evidence tree.
