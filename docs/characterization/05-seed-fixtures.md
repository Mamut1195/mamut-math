# 05 — Seed & randomness fixtures (by reference) @ `6db4862d7a4035de8d494cdc62607b12b4edb103`

Frozen SHA `6db4862d7a4035de8d494cdc62607b12b4edb103`; tree dirty only in
`mamut.engine.json` (manifest, excluded). Every file:line below VERIFIED via
`git grep -n` / `git show HEAD:` — nothing copied, nothing modified. Fixture files
are characterized by path + SHA-256 (working tree = HEAD for these paths) + short
excerpt; hashes recomputable with `Get-FileHash -Algorithm SHA256`.

Method for the seed census: `git grep -n "seed" -- hydro-optimizer` plus
`git grep -n "seed_from_u64|seed_override|unwrap_or(42)|DEFAULT_SEED|const.*SEED" --
'*.rs' hydro-cli/src`, each hit verified in context. Enumeration commands are quoted
in doc 04 §1; the verifier re-runs the same two commands.

## 1. Seed constants / seed values used in tests (file:line + value)

### 1.1 Production seed plumbing (the values tests pin)

| Location | Value | Role |
|---|---|---|
| `hydro-optimizer/src/config.rs:85-86,199` | `seed: u64`, default **`42`** ("Oracle default: 42", line 85) | Canonical default; `config.rs:327` asserts `cfg.seed == 42` |
| `hydro-optimizer/src/rng.rs:15-17` | `root_rng(seed) = ChaCha20Rng::seed_from_u64(seed)` | Root stream constructor |
| `hydro-optimizer/src/rng.rs:25-35` | `child_rng(root_seed, generation, individual_idx)`: `s = seed·0x9E3779B97F4A7C15 + (gen<<32) + idx`, 3-round splitmix64 (`0xBF58476D1CE4E5B9`, `0x94D049BB133111EB`, shifts 30/27/31), `seed_from_u64(s)` | Per-(generation, individual) derivation (see §3) |
| `hydro-optimizer/src/optimizer.rs:194-195,211,260` | `seed = self.config.seed` → `root_rng(seed)`; passed as `seed` into `evaluate_population(..., seed, gen, ...)` | Production threading of the configured seed |
| `hydro-optimizer/src/optimizer.rs:458` | `child_rng(seed, generation, i)` bound to `_child_rng` (reserved/unconsumed) | Deterministic hook for future worker-independent streams; currently unused |
| `hydro-cli/src/lib.rs:219,227-229,482-485` | `effective_seed = seed_override.unwrap_or_else(\|\| req.seed.unwrap_or(42))` | Resolution order: CLI `--seed` > `DesignRequest.seed` > **42** |
| `hydro-cli/src/main.rs:76` | `--seed <u64>` → `seed_override` into `run()` | CLI entry of the same chain |
| `hydro-optimizer/tests/solver_wiring_smoke.rs:75,113,173,232,292,347,402` | **`seed: 42`** in all 6 solver smoke tests (pop 8 / gen 3) | Frozen end-to-end determinism anchor |
| `hydro-cli/benches/performance.rs:53` | `seed: 42` | Benchmark budget anchor (`pump_station_optimizer_8x3`) |

### 1.2 Unit-test seeds (determinism proofs, not production values)

| Location | Seed(s) | What it pins |
|---|---|---|
| `config.rs:497-498` | `root_rng(42)` ×2 | Same seed → same sequence (Design §5) |
| `config.rs:507-508` | `child_rng(42,0,0)` vs `child_rng(42,0,1)` | Index divergence |
| `encoding.rs:407-408` (`make_rng`), callers `:454,:484` | **42** | Round-trip / clamp suites |
| `encoding.rs:552` / `:582,598,614,630,646` | **99** / **1..=5** | Int-gene contract / 6 per-type round-trips |
| `operators.rs:388-396` | `run(99)` vs `run(100)` equal / differ | SBX determinism + sensitivity |
| `operators.rs:437-448` | `for seed in 0..200u64` | At-least-one-mutation existence sweep |
| `operators.rs:499-506,574-581` (varOr/init) | `root_rng(seed)` closures, same-seed equality | varOr + init determinism |
| `nsga3/selection.rs:117` | `ChaCha20Rng::seed_from_u64(0)` | Selection fixture RNG |
| `nsga3/selection.rs:243-251` | `seed_from_u64(123)` ×2 | "identical seeds → identical selection" |
| `nsga3/niching.rs:168` | `seed_from_u64(42)` | Niching fixture RNG (reserved `rng` param path) |
| `hydro-cli/tests/mapping.rs:104-121` | override 42 over req.seed 99; fallback 42 | Seed-resolution precedence (REQ-005) |
| `hydro-terrain/src/graph.rs:287` | `(variant as u64) * 42 + 7` (`SmallRng`) | Route-variant determinism (terrain-side, not optimizer) |
| `hydro-terrain/tests/steiner.rs:292-315` | `proptest!` `cases: 20`, `seed in 0u64..1000u64` → `SmallRng::seed_from_u64(seed)` | Property corpus sampling (connectivity invariant) |

No `thread_rng`, no `HashMap` on any stochastic path (design §12; `BTreeMap`/`BTreeSet`
only — e.g. `astar.rs:37`, `constraints.rs:677`, `objective.rs:187`). Tie-breaks are
index/insertion-order deterministic (archive `optimizer.rs:39`, sort
`nondom_sort.rs:36`, niching smallest-index `niching.rs:136-146`, A* `(f_bits, idx)`
`astar.rs:56-62`).

## 2. Deterministic fixture files used by optimizer tests (path + SHA-256 + excerpt)

Hashes of working-tree files (tree = HEAD for all listed paths):

| Fixture (committed, `tests/oracle/fixtures/`) | SHA-256 | Consumed by |
|---|---|---|
| `optimizer_quality_metrics_golden.json` | `275F85BC…96756FD` (`275F85BCAC8083A49F6D4C75BD2C5C15F61DC274B3CB74B04C27224CF96756FD`) | `metrics.rs:257-276` @ 1e-12 abs tol |
| `path_smoothing_golden.json` | `167FCE18…E720C83888` (`167FCE18DF5B1A08875FFEDC5382C5A7AB63AD59DDEC7C76B4654FE720C83888`) | `routing/mod.rs:186-210` @ 2% rel tol |
| `solvers_sewer_golden.json` (representative of the 6 solver goldens + 6 alternatives/valves variants) | `8B667021…C85133FB` (`8B6670216CFC157EB2865065F27B057AC28E2D24162EE192646615CFC85133FB`) | `solver_wiring_smoke.rs` via `oracle::solvers` |
| `terrain_golden.json` (terrain context for smoke terrain builder) | `E8FEB3AB…FD90CCFE` (`E8FEB3ABA508139CBE6014B85FB5E053029330131A59F05BD90CCFE75C4B2B05`) | `make_terrain` in smoke + terrain tests |

Excerpt — `optimizer_quality_metrics_golden.json` (schema_version 1; 3-pt approximation
+ 4-pt reference 5-D fronts, ref point `[6;5]`):

```json
  "description": "Exact five-objective metric parity generated with DEAP HV and independent Python IGD+.",
  "reference_point": [6.0, 6.0, 6.0, 6.0, 6.0],
  "oracle_hypervolume": 784.0,
  "oracle_igd_plus": 0.6889635872182424,
  "absolute_tolerance": 1e-12
```

Excerpt — `path_smoothing_golden.json` (blocking square [7,2]–[13,8], start
`[0,5]` → end `[20,5]`, clearance 0.1, RDP ε 0.5):

```json
  "oracle_waypoints": [[6.9, 1.9], [13.1, 1.9]],
  "oracle_length": 21.328780519261954,
  "relative_tolerance": 0.02
```

Remaining committed fixtures (names VERIFIED via `git ls-files`, same provenance,
hashes recomputable with the method above — not re-hashed here to bound the
enumeration): `hydraulics_golden.json` (474 vectors), `norms_smoke.json`,
`norms_violations_golden.json`, `solvers_{conveyance,distribution,intake,pump_station,water_supply}_golden.json`
+ `_alternatives`/`_valves(_saturated)` variants, `steiner_golden.json`,
`steiner_suboptimal_golden.json`, `terrain_irregular_golden.json`. Generators:
15 `tests/oracle/gen/generate_*.py` scripts (doc 04 §3.1).

## 3. Why `child_rng` derivation makes fixtures meaningful

`child_rng(seed, gen, idx)` (`rng.rs:25-35`) maps each (generation, individual) pair
to an independent `ChaCha20Rng` stream via a splitmix64 avalanche of
`seed·0x9E3779B97F4A7C15 + (gen<<32) + idx`. Purpose (module docs lines 1–11,
Design §5): **rayon-worker-independent reproducible streams** — the same seed
reproduces bit-identical Pareto fronts regardless of thread scheduling. Production
currently threads one `&mut ChaCha20Rng` sequentially (`optimizer.rs:195`) because
`Solver::solve(&mut self)` forces `Arc<Mutex<S>>` evaluation (REQ-017 defers rayon);
the derived child stream at `optimizer.rs:458` is reserved for the future solver
pool. Consequence for fixtures: golden values (HV = 784.0, IGD+ =
0.6889635872182424, path length 21.328780519261954, solver goldens) are functions of
`(seed, budget, code)` only — no wall-clock, thread-count, or `HashMap`-order
input — so the committed JSON stays a valid regression anchor across machines
(CI runs ubuntu + windows, `.github/workflows/ci.yml` VERIFIED).

## 4. Corpus / regression fixture directories

- `tests/oracle/fixtures/` (22 JSON) + `tests/oracle/gen/` (14 `generate_*.py` + 1
  oracle self-test) — authoritative differential corpus (doc 04 §3.1).
- `hydro-terrain/tests/snapshots/route_variant__steiner_variant0.snap` — lone `insta`
  snapshot (deterministic edge-list output; VERIFIED `git ls-files`).
- `hydro-norms/src/data/*.json` — 15 norm-profile data files (CONAGUA, EPA, ABNT, …;
  VERIFIED `git ls-files`) — domain data, not optimizer fixtures; never imported by
  `hydro-optimizer` (negative seam, doc 01 §2).
- `hydro-cli/benches/performance.rs` (+ `benchmarks/performance_budgets.json`) —
  Criterion regression corpus with seed-42 anchor (doc 04 §B5).
- `dev/clone_count_check.md` — sole `dev/` file; process note, not a fixture.
- `TESTDATA.md` (repo root) — pointer to an **external** candidate-dataset index
  (`../../ARCHIVOS-DE-PREUBA-PARA-APPS/`, outside the repo); explicitly "una guía, no
  la fuente de verdad", mapping "por confirmar" → **PROVISIONAL / not a frozen
  fixture**; uninspectable from this ticket's scope, recorded for completeness.
- Python predecessor datasets: unreachable (doc 04 §3.2 BLOCKED).

(End of file)
