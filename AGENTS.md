# AGENTS.md — MAMUT Math (constitución operativa del repo)

> La configuración global (`~/.config/opencode/`) define quiénes son los agentes,
> modelos, providers, MCPs y permisos. Este archivo define únicamente las reglas
> operativas específicas de MAMUT Math dentro de este proyecto. En caso de
> conflicto, la configuración global es fuente de verdad para identidad/permisos.

## 1. Purpose

MAMUT Math es una librería Rust compartida y domain-agnostic para capacidades
matemáticas reutilizables por productos MAMUT de ingeniería:

- optimización;
- Pareto / NSGA-III;
- sensibilidad;
- gradientes;
- optimización local;
- composición híbrida;
- contratos reproducibles de evidencia.

No es una librería científica general.

## 2. Canonical governance

- Notion **MAMUT Math — Optimization & Sensitivity Library** y su base
  **MAMUT Math · Tickets** son canónicos para scope, DAG, estado, dependencias,
  release y acceptance.
- Sólo `Estado = Siguiente` puede iniciarse.
- Dependencias HARD deben estar `Done`.
- El execution snapshot de un ticket queda congelado al iniciarse.
- No inventar tickets ni adelantar releases.

## 3. Ownership

MAMUT Math PUEDE poseer:

- mathematical contracts genéricos;
- optimization abstractions;
- Pareto / NSGA-III genérico;
- sensitivity;
- gradient/local optimization;
- linear algebra ports;
- hybrid composition;
- reproducibility/evidence contracts.

MAMUT Math NO posee:

- hidráulica, estructuras, geotecnia;
- ecuaciones específicas de dominio;
- NEC/ACI/normativa;
- política de unidades de cada dominio;
- EPANET/OpenSees/Robot/CalculiX semantics;
- UI, billing, Harness orchestration.

## 4. Architecture invariants

- API pública MAMUT-owned.
- Tipos de terceros no pueden filtrarse a contratos públicos estables.
- V0.x usa `f64` canónico.
- Backends reemplazables mediante traits/ports.
- Sin Python runtime en producción.
- Python/JAX/DEAP/pymoo pueden utilizarse como oráculos offline/research.
- Gradientes no sustituyen Pareto.
- Domain adapters deben mantenerse finos.
- No big-bang migration.
- El optimizador Hydraulics existente sigue siendo autoridad hasta que el gate
  E2E correspondiente apruebe su sustitución.

## 5. Execution / Git

- Ticket que modifica repo/código → branch/worktree aislado obligatorio.
- Ticket governance/research/architecture-only que explícitamente prohíba
  Git writes → NO crear repo, branch ni worktree artificialmente; puede
  ejecutarse sin repo Git inicializado.
- Un ticket por executor/worktree.
- Implementers no hacen commit/push/merge.
- El primary controla integración y cierre.
- Máximo 1 fix + 1 re-review; después `HUMAN_DECISION_REQUIRED`.

## 6. Model routing

- `mamut-math-implementer` / DeepSeek V4.1 Flash: implementación Rust,
  debugging, tests y refactors normales.
- `mamut-math-repo-engineer` / Muse Spark 1.3: arquitectura repo-wide,
  crates, traits, API pública, SDK/CLI/WASM y refactors cross-crate.
- `mamut-math-verifier` / GLM-5.3 Flash: verificación matemática/numérica
  independiente.
- `mamut-math-reviewer` / GPT-6 Luna: L3, gates importantes `MATH_VERIFY`,
  algoritmos, API pública, release gates y conflictos no resueltos.
- No llamar modelos adicionales cuando una herramienta determinista pueda
  resolver la cuestión.

## 7. Evidence hierarchy

Preferir, en orden:

1. fuente matemática/ingenieril verificable;
2. oracle independiente;
3. tests/evidencia de máquina;
4. review LLM;
5. opinión del implementer.

- Un PASS de un LLM jamás sustituye tests/oráculos exigidos.
- `MATH_VERIFY=ON` requiere verificación numérica independiente.
- Para algoritmos estocásticos usar seeds, corpus y métricas
  estadísticas/de calidad; no aceptación visual o subjetiva.

## 8. Tools

- CodeGraph primero cuando se necesiten relaciones del repo.
- Wolfram es herramienta de cálculo, no autoridad normativa.
- `mamut-math` primary es el único escritor de governance/Notion.
- Los subagentes devuelven propuestas/evidencia y no alteran Notion.
- Engram sólo para decisiones durables; no ruido transitorio.
