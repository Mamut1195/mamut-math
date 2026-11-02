# Preparation plan · 2026-11-02

## 1. Goal

Preparar el DAG y las condiciones de asignación de subagentes; aplicar MIT y sincronizar la preparación con GitHub.

## 2. Context

Baseline `19a316d251da60f9f3398c34e1855481cf15199b`, igual a main local/remoto al comenzar. Dos crates implementados. Notion es canónico; 59 tickets recuperados individualmente y consulta completa de propiedades. E2E-001 es el único Siguiente; su candidato local `b0f4fcd` conserva gates pendientes.

## 3. Constraints

No iniciar tickets, promover estados o asignar releases. No integrar/publicar el candidato E2E. Preservar `.gitattributes`, `.codegraph/`, `docs/tickets/` y `scripts/tickets/` previos. Un escritor por contrato; snapshots históricos intactos.

## 4. Affected areas

`LICENSE`, metadata license del workspace, `README.md`, `docs/planning/` y `scripts/planning/`. Preparación administrativa solicitada, sin atribuirla a un ticket Planeado ni al ARC-003 que prohíbe Git writes.

## 5. Implementation steps

1. Contrastar documentación, propiedades y cuerpos canónicos, ramas y arquitectura mediante CodeGraph.
2. Guardar propiedades y evidencia de cobertura; derivar HARD/SOFT, orden topológico y estados de entrada; documentar conflictos de escritura aparte.
3. Crear MIT y ajustar metadata heredada en worktree aislado.
4. Validar grafo, igualdad de aristas, enlaces locales, licencia heredada y alcance Git; publicar sólo la preparación y hacer fast-forward de las referencias locales pertinentes.

## 6. Required skills

task-planning, safe-git-workflow, notion-research-documentation y verify-before-complete. No se requiere ejecución numérica: no cambia Rust ni se certifica un algoritmo.

## 7. Validation

Generación determinista y modo `--check`; verificación independiente contra el DAG local previo; identidad de 59 filas, referencias/ciclos y gates. `cargo metadata --locked --offline --no-deps`, `cargo check --workspace --locked --offline`, `git diff --check`; hashes de trabajo previo antes/después y recibo SHA local/remoto.

## 8. Risks

Una capa topológica no autoriza ejecución ni garantiza ausencia de conflictos. El catálogo de modelos no prueba llamadas operativas. Los resultados históricos de Notion no son tests actuales. No ejecutar el selector lento amplio que también selecciona el regenerador de fixtures; VV-004 posee su reparación futura. CI remoto y gates E2E quedan NOT RUN.
