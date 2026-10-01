# 06 — Known HV / IGD+ gaps @ `6db4862d7a4035de8d494cdc62607b12b4edb103`

`hydro-optimizer/src/metrics.rs` (277 lines) read **fully and verbatim** at the frozen
SHA (working tree = HEAD for this path). Line citations below are to that file unless
stated otherwise. Cross-references: anomaly A5 (doc 01 §3), metrics row (doc 02),
parity history (`docs_dev/optimizer_parity_{findings,progress,task_plan}.md` at HEAD).

## 1. What is actually implemented (VERIFIED)

| Symbol | Lines | Signature | Algorithm |
|---|---|---|---|
| `hypervolume` | 30–51 | `pub(crate) fn hypervolume(front: &[Vec<f64>], reference_point: &[f64]) -> f64` | Exact dominated HV, **arbitrary M** (minimization); filters non-finite / wrong-dim / outside-box points; empty / non-finite ref → 0.0 |
| `hypervolume_slices` | 53–90 | private recursive `fn(points: &[Vec<f64>], reference_point: &[f64]) -> f64` | N-D recursive slicing: sort + dedup levels on last axis, append ref level, `Σ slice·Δ`; 1-D base case `(ref − min).max(0)` |
| `igd_plus` | 107–137 | `pub(crate) fn igd_plus(front: &[Vec<f64>], reference_front: &[Vec<f64>]) -> f64` | Ishibuchi-2015 IGD+: `(1/\|R\|)·Σ min d+`, `d+(a,r) = √(Σ max(0, aⱼ−rⱼ)²)`; either side empty → `∞` |
| 11 tests | 141–276 | `#[cfg(test)] mod tests` | 5 HV (1-pt/2-pt 2-D, 1-pt 5-D = 32.0, 2-pt 3-D union = 6.0, empty→0) + 5 IGD+ (perfect-match 0, ref-better √2, front-better 0, empty→±∞) + 1 golden-parity test |

Both public-in-crate functions are already **arity-generic** (`&[Vec<f64>]` — no
`[f64; 5]` binding, unlike sort/normalize/niching). Module docs (lines 1–14) state the
HV and IGD+ definitions exactly as implemented; the implementation matches the docs.

## 2. Wiring status: implemented & dormant (VERIFIED)

- Visibility: `#[allow(dead_code)]` + `pub(crate)` on **both** functions (lines 17, 93)
  — outside the REQ-014 six-item public API (`lib.rs:41-51`).
- Callers: `git grep -n "hypervolume|igd_plus" -- '*.rs' ':!hydro-optimizer/src/metrics.rs'`
  returns **zero hits outside `metrics.rs`** — no production caller exists.
  `optimize()` (`optimizer.rs:192-425`) and `compute_gen_stats` (`optimizer.rs:554-606`)
  never call either function. Reachable **only** via the 11 unit tests.
- This is anomaly A5 (doc 01 §3): the gap is **partly a visibility gap**, confirmed
  mechanically, not by inspection alone.

## 3. IGD+ specifically: present, corrected, gated

- **IGD+ is present** (`igd_plus`, lines 107–137) with the correct minimization sign
  `max(approx − ref, 0)` (line 122; comment line 117 states the convention).
- History (VERIFIED `docs_dev/optimizer_parity_findings.md` + `..._progress.md`,
  2026-06-19): the original code had the sign reversed (`max(ref − approx)`);
  fixture generation exposed it; direction-sensitive RED tests then GREEN fix.
  Companion fixes in the same work unit: Rust HV was **2-D-only** (returned 0 for
  N-D) → exact recursive-slicing N-D HV (lines 53–90, added to close the gap);
  A* clearance/visibility defect (direct-segment fallback 20.0 vs oracle 21.328…,
  caught at 6.23% vs the 2% contract) → miter-offset candidates.
- Differential gate (executable, committed): `test_five_objective_metrics_match_python_oracle`
  (lines 257–276) loads `tests/oracle/fixtures/optimizer_quality_metrics_golden.json`
  via `include_str!` (3-pt approximation + 4-pt reference 5-D fronts, ref `[6;5]`,
  oracle HV = **784.0**, IGD+ = **0.6889635872182424**) at **1e-12 absolute tolerance**.
  Oracle provenance per fixture description + parity docs: **DEAP exact HV** and an
  **independent-Python IGD+** implementation. No `#[ignore]` remains anywhere
  (VERIFIED `git grep "#\[ignore\]"` empty).
- Structural limitation the gate accepts deliberately (parity-findings record): the
  gate is **metric-level differential parity**, because end-to-end optimizer-quality
  comparison is infeasible in CI (Python minimum 4/1 pump-station run >10 min on one
  CPU). Full quality comparison belongs to the benchmark/regression suite.
- Plain (non-plus) IGD is **absent** — and correctly so: no gate, doc, or consumer
  requires it; IGD+ Pareto-dominates plain IGD for quality assessment (Pareto
  respecting by construction). Absence of plain IGD is NOT a gap.

## 4. Classification

- (a) **Implemented & used**: the *algorithms* are used — by the committed differential
  gate at 1e-12 (HV 784.0 / IGD+ 0.6889635872182424, DEAP + independent-Python
  provenance). Nothing about the math is placeholder.
- (b) **Implemented & dormant**: both functions as *production capabilities* —
  `pub(crate)` + `#[allow(dead_code)]`, zero non-test callers, invisible to
  `hydro-cli`, the adapter (subprocess boundary), and downstream consumers.
- (c) **Absent from Hydraulics but required by future MAMUT Math parity gates**:
  a **public, MAMUT-owned, third-party-free** metrics API (`&[Vec<f64>]`-in/`f64`-out,
  no `rand`/`geo` leakage — the current signatures already satisfy this shape, so the
  port is a visibility promotion, not a rewrite) **plus fresh library-side oracle
  fixtures**. Doc 02 classifies this as REWRITE (visibility): the library must NOT
  widen Hydraulics visibility as a shortcut; it re-exposes the algorithms with its
  own oracles while the Hydraulics golden file stays the Hydraulics-side gate.

## 5. Verbatim DoD statement ("Known HV/IGD+ gap documented")

> At frozen SHA `6db4862d`, HV (exact N-D recursive slicing) and IGD+ (Ishibuchi
> 2015, sign-corrected) are correctly implemented in `hydro-optimizer/src/metrics.rs`
> and differentially gated against a committed DEAP/independent-Python fixture at
> 1e-12, but remain `pub(crate)` + `#[allow(dead_code)]` with no production caller;
> end-to-end optimizer-quality parity is infeasible in CI (>10 min Python minimum
> run) and is explicitly deferred to the benchmark/regression suite. The known gap is
> therefore architectural (visibility + scope of the gate), not algorithmic.

## 6. New anomaly / refinement found during this pass (with evidence)

- **A7 — WITHDRAWN / reversed (docs only, no code impact).**
  An earlier pass claimed doc 01's description of
  `hydro-optimizer/tests/eng022_no_epa_inner_loop.rs` as "215 lines" was stale and
  that the file is **195 lines** (via `git show HEAD:... | Measure-Object -Line`).
  That was WRONG: the 195 figure is a **non-blank** line count (a `Measure-Object`
  quirk) mislabeled as total lines. Verified truth: the file is **215 lines total**
  (195 non-blank, 20 blank) at the frozen SHA. **Doc 01's original "215 lines" is
  correct and stays unchanged.** Content matches doc 01's description (20
  forbidden-invoke tokens, Cargo-token scan, six-solver pin, budgets-lock). No code
  finding; no doc-01 edit (doc 04 §3.1/§3.3 counts adjusted accordingly).
- No other new code anomalies: metrics module contains no domain constants, no norm
 /profile/unit references, and no third-party types in its signatures (stdlib +
  `serde_json` in tests only) — consistent with doc 02's EXTRACT classification.

(End of file)
