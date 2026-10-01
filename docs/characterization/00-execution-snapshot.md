# IMMUTABLE EXECUTION SNAPSHOT — MATH-NSGA-001
Frozen at: 2026-10-01T02:51:43Z (UTC). This snapshot is immutable for the duration of this ticket execution.

## Ticket (canonical source: Notion, copied verbatim)
- **Ticket**: MATH-NSGA-001 · Characterize Hydraulics NSGA-III extraction seam
- **URL**: https://app.notion.com/p/3ea721d30f8581ed8398e34aff4f0f55
- **Estado**: Siguiente (verified 2026-10-01 via Notion fetch)
- **Prioridad**: P0 · **Bloque**: NSGA-III · **Release**: V0.1 Core + NSGA-III
- **Risk**: L3 · **MATH_VERIFY**: ON · **Orden**: 107
- **Dependencias**: HARD MATH-ARC-002 (Estado=Done, verified); HARD MATH-CON-001 (Estado=Done, verified)
- **Objective**: Determine exactly which hydro-optimizer code is generic math and which remains Hydraulics-owned before moving anything.
- **Scope**: reference points, nondominated sort, normalization, niching, operators, RNG, encoding, constraints, objectives, penalties, metrics, public API and current parity fixtures.
- **Non-goals**: No code move/refactor.
- **Definition of Done**:
  - KEEP-IN-HYDRAULICS vs EXTRACT vs REWRITE-USING-LIBRARY matrix complete.
  - Current SHA and tests/oracles frozen.
  - Known HV/IGD+ gap documented.
  - Rollback plan defined.
- **Validation / evidence**: Code map with exact files/symbols. Current Hydraulics baseline test counts + seed fixtures captured.
- **Architecture guardrail**: No domain physics/normative logic enters MAMUT Math; public API remains MAMUT-owned.
- **Acceptance (Notion)**: Extraction seam proven from current code; no hydraulic objective/constraint semantics misclassified as generic.

## Execution sandbox
- Worktree: `C:\Users\Jonnathan\Mamut system\mamut-math-nsga-001`
- Branch: `ticket/MATH-NSGA-001-extraction-seam`
- BASE_SHA (WAVE_BASE_SHA): `dc7dcdfd7633baa9e239a6634bff0e263e72b27c` (verified == HEAD, tree clean at start)

## Frozen source SHAs (provenance-critical)
1. **Instructed "CURRENT HYDRAULICS SHA"** — `git -C "C:\Users\Jonnathan\Mamut system\mamut-hydraulics" rev-parse HEAD`
   = `2a74e9dda021bd6bd575dfb3a1a25b800767f26a` (branch main).
   - **CAVEAT A (dirty)**: repo has one modified tracked file `AGENTS.md` (pre-existing local change; untouched by this ticket, recorded only).
   - **CAVEAT B (scaffold-only)**: this repo contains ONLY `AGENTS.md`, `README.md`, `.agents/` (single commit "chore: initialize project scaffold"). It contains **no hydro-optimizer code**. Its own README states: *"Known AS-IS (Notion, code lives outside this folder): `Mamut1195/optimizador-hidraulico-rust` (8 productive crates + oracle, 6 design solvers, own NSGA-III, 15 norm profiles, 523 tests declared, JSON CLI, MAMUT adapter)"*.
2. **AS-IS optimizer repository actually characterized** (the only real location of the current Hydraulics optimizer, per the anchor above):
   `C:\Users\Jonnathan\Mamut system\mamut-harness\motores\optimizador-hidraulico-rust`
   - Remote: `https://github.com/Mamut1195/optimizador-hidraulico-rust.git`, branch `main`
   - **Frozen SHA: `6db4862d7a4035de8d494cdc62607b12b4edb103`**
   - **CAVEAT C (dirty)**: one modified tracked file `mamut.engine.json` (pre-existing local change; untouched by this ticket, recorded only). No source-code files are dirty, so all file/symbol evidence below is provable against the frozen SHA via `git ls-files` / `git show HEAD:<path>`.
   - **DISCLOSURE**: the coordinator's STEP 2 command freezes SHA (1). SHA (1) cannot satisfy the Notion DoD ("Code map with exact files/symbols" of the current hydro-optimizer) because it holds no code. Evidence gate for every claimed file/symbol is therefore bound to SHA (2). This deviation is surfaced for verifier/reviewer validation and is flagged in the final report as a finding requiring coordinator/human awareness.

## Scope surfaces (per Notion) — initial file anchors verified at SHA (2)
- reference points → `hydro-optimizer/src/nsga3/reference_points.rs`
- nondominated sort → `hydro-optimizer/src/nsga3/nondom_sort.rs`
- normalization → `hydro-optimizer/src/nsga3/normalize.rs`
- niching → `hydro-optimizer/src/nsga3/niching.rs`
- (selection) → `hydro-optimizer/src/nsga3/selection.rs`
- operators → `hydro-optimizer/src/operators.rs`
- RNG → `hydro-optimizer/src/rng.rs`
- encoding → `hydro-optimizer/src/encoding.rs`
- constraints → `hydro-optimizer/src/constraints.rs`
- objectives → `hydro-optimizer/src/objective.rs`
- penalties → located inside constraints/objective surfaces (verified in code map)
- metrics → `hydro-optimizer/src/metrics.rs`
- public API → `hydro-optimizer/src/lib.rs` (+ config/results/errors/optimizer surfaces)
- parity fixtures → `hydro-optimizer/tests/*`, workspace `tests/`, `benchmarks/`, Python oracle `motor-optimizacion-hidraulico` (oracle/predecessor only, offline)

## Mode flags
- **MATH_VERIFY = ON** — trigger: any claim about algorithm semantics (NSGA-III components, operators, RNG determinism, metric definitions HV/IGD+) and evidence integrity (every file/symbol verified to exist at frozen SHA). Independent verifier (GLM-5.3 Flash) required; reviewer (GPT-6 Luna) mandatory (Risk L3).
- **METHOD_REGISTRY = NONE** — characterization-only ticket; no method is implemented, registered, or moved in MAMUT Math by this ticket.

## Governing sources
- Notion ticket page (canonical, copied above) — PROVENANCE VERIFIED (fetched 2026-10-01).
- MATH-ARC-001 Architecture Freeze v1; MATH-ARC-002 Crate Boundaries & API Isolation Freeze v1 (Done).
- MATH-CON-001 core contracts merged at main `dc7dcdfd7633baa9e239a6634bff0e263e72b27c` (this worktree base).

## Hard constraints
- Documentation-only deliverables inside this math worktree (`docs/characterization/`). NO code move, NO copying of Hydraulics code into the library, NO refactor of Hydraulics.
- Subagents never commit/push/merge; subagents never modify the Hydraulics repos (read-only inspection only; allowed writes limited to `target/` build artifacts if test enumeration requires it).
- Do not read sibling mamut-math ticket worktrees.
- No Notion writes (coordinator owns governance).
- Fix cycle: max 1 fix + 1 re-verify/re-review, then `HUMAN_DECISION_REQUIRED`.
- Provenance discipline: claims marked VERIFIED only when confirmed at SHA (2) via git; docs-only claims = PROVISIONAL; uninspectable = BLOCKED.
