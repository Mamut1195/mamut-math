# 04 — Frozen baseline: tests & oracles @ `6db4862d7a4035de8d494cdc62607b12b4edb103`

- **Repo**: `C:\Users\Jonnathan\Mamut system\mamut-harness\motores\optimizador-hidraulico-rust`, branch `main`
- **Frozen SHA**: `6db4862d7a4035de8d494cdc62607b12b4edb103` (VERIFIED `git rev-parse HEAD`)
- **Tree state**: only dirty tracked file is `mamut.engine.json` (VERIFIED `git status --short`, `git diff --stat` touches that file only). All `.rs`/`.json`/`.yml` evidence below matches HEAD.
- **Rule compliance**: no file in the Hydraulics repos modified; `cargo test` NEVER run in the Hydraulics repo (static enumeration + recorded baselines only, per ticket guardrails).
- **Status legend**: VERIFIED = confirmed at frozen SHA via git; PROVISIONAL = record exists but predates the frozen SHA or was not re-executed; BLOCKED = location uninspectable, evidence of search recorded.

## 1. Static test-declaration census (independently recomputed)

### Method (exact, reproducible by the verifier)

PowerShell 5.1, from the repo root at the frozen SHA:

```powershell
$files = git ls-files '*.rs'
$total = 0; $perCrate = @{}
foreach ($f in $files) {
  $top = ($f -split '/')[0]
  if ($top -notlike 'hydro-*' -and $top -ne 'tests') { $top = 'other' }
  $n = (Select-String -LiteralPath $f -Pattern '#\[test\]|#\[proptest\]|#\[bench\]' -AllMatches | Measure-Object).Count
  $total += $n
  if (-not $perCrate.ContainsKey($top)) { $perCrate[$top] = 0 }
  $perCrate[$top] += $n
}
```

Pattern counts literal `#[test]` / `#[proptest]` / `#[bench]` attribute lines in
`git ls-files '*.rs'` (98 files). Companion sweeps (same method):

- `git grep -n "tokio::test" -- '*.rs'` → **zero hits**.
- `git grep -n "#\[bench\]" -- '*.rs'` → **zero hits**.
- `git grep -c "#\[proptest\]|#\[bench\]"` → **zero hits** (no outer `#[proptest]` attribute anywhere).
- `git grep -n "proptest!" -- '*.rs'` → **exactly one hit**: `hydro-terrain/tests/steiner.rs:292` (macro invocation wrapping one inner `#[test]`, see §2).
- `git grep -n "#\[ignore\]" -- '*.rs'` → **zero hits** (no ignored tests remain at the frozen SHA).

### Result (VERIFIED)

| Crate / member | `#[test]` declarations |
|---|---|
| hydro-optimizer | 210 |
| hydro-solvers | 71 |
| hydro-cli | 57 |
| hydro-types | 52 |
| hydro-terrain | 45 |
| hydro-hydraulics | 45 |
| hydro-norms | 32 |
| tests/oracle (`tests`) | 16 |
| hydro-kernels | 11 |
| **TOTAL** | **539** |

### Delta vs the orchestrator's 540 (explained, not forced-reconciled)

The orchestrator's census reads 540 with hydro-terrain at 46. Independent recount
gives **539** with hydro-terrain at **45** (per-file: `cost.rs` 3, `graph.rs` 5,
`terrain.rs` 10, `k_shortest_paths.rs` 3, `route_variant.rs` 4, `steiner.rs` 8,
`terrain_graph.rs` 5, `terrain_model.rs` 7 = 45). The +1 is identified: the single
`proptest! {` macro invocation at `hydro-terrain/tests/steiner.rs:292` was counted
as an additional test declaration. That block contains exactly one inner `#[test] fn
steiner_always_connects_terminals` (lines ~297–304, `cases: 20` config at lines
293–296), which IS included in the 539. So: **539 = pure `#[test]` attribute
count; 540 = 539 + 1 `proptest!` invocation**. Both numbers describe the same tree;
the method above is the reproducible definition. No `#[bench]`, no `#[tokio::test]`.

### hydro-optimizer split: inline `#[cfg(test)]` vs `tests/` integration (VERIFIED)

- Inline in `hydro-optimizer/src/**/*.rs` (23 files): **201** `#[test]` — matches the
  `docs_dev/optimizer_parity_progress.md` "201 unit" record verbatim (§3).
- Integration in `hydro-optimizer/tests/`: **9** = 6 in `solver_wiring_smoke.rs`
  (lines 75ff, 6 solver smoke tests, pop 8 / gen 3 / seed 42) + 3 in
  `eng022_no_epa_inner_loop.rs` (**215 lines total** — 195 non-blank, 20 blank — at
  HEAD. Doc 01's "215 lines" is correct; the former "195-line" refinement was a
  non-blank line count mislabeled as total lines and is WITHDRAWN — doc 06 §6/A7).
- 201 + 9 = **210**. `git grep -n "mod tests" -- hydro-optimizer/src` returns 18
  inline `mod tests` blocks (17 source modules + `lib.rs:74`; `lib.rs:70` carries the
  note that former integration tests were moved inline).
- `hydro-optimizer` dev-dependencies (VERIFIED `hydro-optimizer/Cargo.toml`):
  `approx`, `proptest`, `oracle = { path = "../tests/oracle" }` (workspace).

## 2. Recorded historical baselines with provenance

| # | Record | Content | SHA it refers to | Status |
|---|---|---|---|---|
| B1 | `docs/TEST-REPORT.md` (2026-06-02, auditor Claude Code Sonnet 4.6) | ~510 total (178 integration + ~332 inline est.), 34 integration files, 38 src files w/ inline tests, REQ-001–018, **2 ignored** (`hydro-optimizer/tests/pr8f_parity_skeleton.rs` HV/IGD+ + path-length skeletons) + 7 gaps (Gap 1 = REQ-012/016 unexecuted) | `c73e677` "v1 boundary" (commit VERIFIED to exist via `git cat-file -t c73e677` → `commit`) | **PROVISIONAL** — predates frozen SHA; skeleton file since deleted, both gaps since closed (§4) |
| B2 | `docs_dev/optimizer_parity_progress.md` (2026-06-19, final entry) | **201 unit + 6 integration smoke, zero ignored**; strict Clippy + fmt clean | Contemporaneous with `5dcf715` "activate quality metric parity" / `456f0f8` "honor path smoothing clearance" (both in history before frozen SHA) | **VERIFIED as record** (file at HEAD, content read verbatim); execution not re-run per ticket guardrails → counts corroborated statically (201 inline still exact; integration 6→9 via ENG-022 addition) |
| B3 | "523 tests" claim | Lives **only** in the scaffold repo README (`mamut-hydraulics` @ `2a74e9d`, line: "8 productive crates + oracle, 6 design solvers, own NSGA-III, 15 norm profiles, **523 tests declared**, JSON CLI, MAMUT adapter"). `git grep -n "523"` over the optimizer repo at the frozen SHA returns **no markdown/README hit** (only float fragments inside fixture JSON + one Cargo.lock checksum) | N/A (Notion-era declaration, undated in-repo) | **PROVISIONAL** — location VERIFIED, counting era unknown; delta vs 539/540 is a known baseline discrepancy: different era (pre-ENG-022 +3, pre/post skeleton ±2, attribute-vs-macro counting ±1, estimate-vs-enumeration). Do not force-reconcile |
| B4 | CI contract `.github/workflows/ci.yml` (at HEAD) | `cargo fmt --all --check` → `cargo clippy --workspace --all-targets -- -D warnings` → **`cargo test --workspace`** (ubuntu + windows; Rust 1.95.0); security job (`secret_scan.py`, `cargo audit`, `cargo deny check`); performance job (`cargo bench -p hydro-cli --bench performance -- --noplot` + `python scripts/check_benchmark_budgets.py` + Criterion artifact upload) | HEAD (file listed by `git ls-files`) | **VERIFIED** (content read verbatim from `git show HEAD:`) |
| B5 | `benchmarks/performance_budgets.json` (at HEAD) | Criterion-slope ceilings: `terrain_graph_from_grid` 100000 ns, `pump_station_solver` 100000 ns, `pump_station_optimizer_8x3` 5000000 ns, `validate_only_json` 100000 ns; `inner_loop` note = ENG-022 lock text (fitness = kernels, verify = EPANET/SWMM post-run, gate = eng022 test path) | HEAD | **VERIFIED** |

Net baseline story: B1 (~510, 2 ignored) → B2 (201+6, zero ignored, both parity gates
activated) → frozen SHA (201 inline unchanged, integration 6→9 with ENG-022 lock,
zero `#[ignore]`, 539 attribute-declarations / 540 with the `proptest!` invocation).

## 3. Oracle locations

### 3.1 `tests/oracle` workspace member (VERIFIED)

- Member of workspace (`Cargo.toml` members list includes `tests/oracle`; crate name
  `oracle`, `publish = false`, "dev-only fixture loader — not published",
  `tests/oracle/Cargo.toml` VERIFIED). Deps: `serde`, `serde_json`, `thiserror`;
  dev-dep `approx`.
- `src/` files (line counts via `git show HEAD:`): `lib.rs` 89 (loader:
  `fixtures_dir()` via `CARGO_MANIFEST_DIR`, generic `load_fixture::<T>(name)`,
  `OracleError::{FileNotFound,Io,Json}`, 1 self-test `fixtures_dir_exists`),
  `solvers.rs` 1178 (`load_*_golden` per solver incl. alternatives/valves variants),
  `terrain.rs` 331, `hydraulics.rs` 221 (`load_hydraulics_golden`), `norms.rs` 144.
  Fixtures "generated by the Python `hydro_engine` oracle", regenerated via
  `python tests/oracle/gen/generate_*.py` with the Python engine on PYTHONPATH
  (`src/lib.rs` docs + `tests/oracle/README.md:24,34,38` VERIFIED).
- `fixtures/`: **22 committed JSON goldens** (VERIFIED `git ls-files`) = 5 non-solver
  (`hydraulics_golden`, `norms_smoke`, `norms_violations_golden`,
  **`optimizer_quality_metrics_golden`**, **`path_smoothing_golden`**) + 6 solver
  `solvers_{sewer,water_supply,conveyance,distribution,pump_station,intake}_golden`
  + 7 `_alternatives`/`_valves` variants + 2 `steiner*` + 2 `terrain*` = 22.
- `gen/`: **14 `generate_*.py` scripts** `generate_{hydraulics,norms_smoke,norms_violations,optimizer_quality_metrics,path_smoothing,solvers_{conveyance,distribution,intake,pump_station,sewer,water_supply},steiner,steiner_suboptimal,terrain}.py`
  + `test_eng032_yn_yc_brentq.py` (oracle self-test) = 15 `.py` total
  (VERIFIED `git ls-files`).
- Consumers (VERIFIED `git grep -ln "oracle::" -- 'hydro-*/tests/*.rs'`): 6 hydro-cli
  happy-path/reliability tests, `hydro-hydraulics/tests/oracle_parity.rs`,
  `hydro-norms/tests/{registry_smoke,validator_golden}.rs`,
  `hydro-optimizer/tests/solver_wiring_smoke.rs`, all 6 `hydro-solvers/tests/*.rs`,
  all 5 `hydro-terrain/tests/*.rs`. (`metrics.rs` golden test uses `include_str!`
  of the same fixture file rather than the `oracle` crate — `metrics.rs:258-260`.)

### 3.2 Python predecessor/oracle `motor-optimizacion-hidraulico` — BLOCKED / location-unknown

- Expected location per in-repo pointers: `tests/oracle/README.md:24` ("the Python
  oracle engine at `../motor-optimizacion-hidraulico/src/` relative to the workspace
  root") and `docs_dev/optimizer_parity_findings.md:8` ("sibling Python oracle …
  `src/hydro_engine/optimization/genetic.py`", smoother `optimization/routing.py`).
  Resolved absolute expectation: `C:\Users\Jonnathan\Mamut system\mamut-harness\motores\motor-optimizacion-hidraulico`.
- Evidence of search (read-only): `Test-Path` on that exact path → **False**;
  directory listing of `...\motores\` shows `flood-engine, motor-clasificacion,
  motor-costos, motor-cronograma, motor-geotecnico, motor-ifc, motor-tablas,
  motor-torres-transmicion, optimizador-hidraulico-rust, structural-kernel` — **no
  `motor-optimizacion-hidraulico`**; no `*motor*` directory under `C:\Users\Jonnathan`
  root matches either.
- Consequence: the Python engine is **not available for re-execution** in this
  environment. All Python-oracle claims rest on the **committed JSON goldens + `gen/`
  scripts + parity-findings record** (differential gates remain executable offline;
  regeneration is not). The committed goldens are therefore the authoritative frozen
  oracle anchor (hashes/excerpts in doc 05).

### 3.3 Parity / gate test files (VERIFIED at HEAD)

- `hydro-hydraulics/tests/oracle_parity.rs` — 474-vector `hydraulics_golden.json`
  parity (Darcy-Weisbach incl. 72-vector Swamee-Jain friction set), `max_relative ≤
  1e-9` (`TOL` const, `rel_err` helper, header lines 1–15 read verbatim).
- `hydro-optimizer/tests/eng022_no_epa_inner_loop.rs` — **215 lines total** (195
  non-blank, 20 blank; measured via `git show HEAD:`): scans `hydro-optimizer`+`hydro-solvers`+`hydro-hydraulics`
  sources for 20 forbidden invoke tokens (`Command::new`, `ENopen`, `swmm_*`, …),
  Cargo.tomls for `epanet|swmm|wntr|pyswmm`, pins `sel_nsga3` export / `solver_guard.solve`
  / six solver files / `performance_budgets.json` inner-loop note. 3 `#[test]` fns.
- `hydro-optimizer/tests/solver_wiring_smoke.rs` — 6 tests, one per solver, seed 42,
  pop 8 / gen 3, feasibility gate `fitness[0] < 1e11` (sentinel `1e12`); header
  documents the seed-42 and 8/3-budget rationales (lines 1–35 read verbatim).
- `hydro-norms/tests/validator_golden.rs` — golden-norms consumer; `hydro-cli/tests/*`
  (9 files incl. `binary_smoke.rs` subprocess contract, `validate_only.rs` <50 ms
  REQ-007 gate) exercise the optimizer end-to-end through `run()`.

(End of file)
