# Preparación para ejecución con subagentes

Consulta canónica del **2026-11-02**, sin iniciar tickets ni cambiar Notion. [DAG completo](DAG.md), [vista interactiva](DAG.html), [propiedades capturadas](tickets-2026-11-02.json), [grafo procesable](graph.json) y [plan de esta preparación](PREPARATION.md). La vista HTML es un artefacto autocontenido de este snapshot; no consulta estados en vivo.

## Resultado y punto de entrada

**59 tickets: 11 Done, 1 Siguiente, 47 Planeado.** El grafo derivado tiene 113 HARD, 3 SOFT internas y 3 notas SOFT externas, sin ciclos ni referencias faltantes. Se consultaron todas las filas, se recuperaron individualmente los 59 tickets y se contrastaron sus diez propiedades operativas con la consulta; también se recuperaron 13 páginas de proyecto/governance. Es una ventana de lectura, no una transacción atómica. No incluye comentarios, adjuntos ni exploración recursiva de fuentes externas.

**Sólo MATH-E2E-001 cumple Estado=Siguiente y HARD Done.** Su cuerpo termina con STOP después del checkpoint M2: el adapter/pin siguiente requiere nueva autorización humana acotada. Esta solicitud prepara el proyecto; no inicia C1/C2/C3. [Ticket canónico](https://app.notion.com/p/3ea721d30f8581b092e8dd433014a7f9).

ARC-003, VV-004 y NSGA-005 tienen HARD satisfechas, pero siguen Planeado y sin release. Son candidatos para decisión de promoción, no una onda ejecutable. Ninguno se convierte en nuevo prerequisito del snapshot congelado E2E-001. [ADR-010](https://app.notion.com/p/40c721d30f8581138283c0a5c6b5e7c6).

## Arquitectura observada

Baseline de main al inspeccionar: `19a316d251da60f9f3398c34e1855481cf15199b`.

| Área | Implementado en el baseline | Límite |
| --- | --- | --- |
| `mamut-math-core` | Variables/candidatos/evaluaciones, errores, codec binario versionado, RNG y manifests de evidencia | `f64`, tipos MAMUT; sin dependencias externas de producción |
| `mamut-nsga3` | Ordenamiento, normalización, niching, selección, variación, métricas HV/IGD+ y `Nsga3Session` ask/tell | Sólo depende de core; física y normativa fuera |
| Validación | Goldens, fixtures independientes, suites rápidas/lentas, harness de calidad y oráculos offline | El harness histórico no acredita la sesión pública; VV-005 posee ese gate futuro |
| Sensibilidad/linalg/gradiente/híbrido/WASM | Tickets y arquitectura objetivo | No son crates implementados en este checkout |

CodeGraph se usó para relaciones y superficies; Cargo/manifests para dependencias reales. Las propuestas antiguas de crates y dependencias no se toman como implementación. El nombre real es `mamut-nsga3`; dominancia/métricas viven allí y el codec actual es binario, diferencias ya reconocidas por ADR-010 y pendientes de ARC-003. No se aplica ese ticket mediante esta documentación.

## Git y checkpoint E2E

| Referencia al comenzar | Estado observado |
| --- | --- |
| Remoto | `https://github.com/Mamut1195/mamut-math.git`, privado, rama por defecto main |
| main local / origin/main / main remoto | Mismo SHA `19a316d251da60f9f3398c34e1855481cf15199b` después de fetch y consulta remota |
| Checkout del usuario | `docs/local-ticket-snapshot`, mismo baseline, con trabajo previo |
| `ticket/MATH-E2E-001-hydraulics-consumer` | Candidato local `b0f4fcd2740109a1e52f9f7fc699588e90d379fd`, tres commits por encima del baseline; 18 rutas modificadas; sin rama remota equivalente |
| Worktrees registrados antes de preparar | Sólo el checkout principal; los paths históricos E2E no estaban registrados |

Los tres commits pendientes son T2a `ac4225d`, M1 `ab4d8ed` y M2 `b0f4fcd`. Su presencia local y su ancestry se comprobaron; sus pruebas/reviews en Notion son evidencia histórica, no ejecución actual. No se integran ni se publican con esta preparación.

El consumidor histórico real es `mamut-harness/motores/optimizador-hidraulico-rust`, no el scaffold `mamut-hydraulics`. Los SHAs, estado del consumidor, archivos de evidencia externos y paths históricos deben revalidarse antes de retomar: no se inspeccionó ni modificó ese repo en esta preparación. El snapshot menciona una base del consumidor distinta de su remoto; no extender la sincronización Math a ese historial automáticamente.

Se preservan los cambios anteriores en `.gitattributes`, `.codegraph/`, `docs/tickets/` y `scripts/tickets/`. No forman parte del commit de preparación. El nuevo DAG es independiente del espejo no versionado. Sin reset, clean, stash, rebase ni force-push.

## Asignación de roles disponible

Catálogo vivo consultado en T3. Los IDs siguientes aparecieron en el catálogo; **ningún modelo fue invocado** y no se verificaron cuota, credenciales efectivas ni funcionamiento del child. El catálogo no acredita un review.

| Rol de AGENTS.md | Provider instance | Model ID propuesto |
| --- | --- | --- |
| mamut-math-implementer | `opencode` | `openrouter/deepseek/deepseek-v4.1-flash` |
| mamut-math-repo-engineer | `opencode` | `openrouter/meta/muse-spark-1.3` |
| mamut-math-verifier | `opencode` | `openrouter/z-ai/glm-5.3-flash` |
| mamut-math-reviewer | `codex` | `gpt-6-luna` |

Revalidar catálogo y perfiles globales al lanzamiento, sin cambiar identidades/permisos para acomodar el plan. La ruta Pi del snapshot histórico no estaba habilitada en el catálogo actual; no afirmar que continúa disponible ni sustituirla silenciosamente si el gate exige esa ruta concreta. Elegir transporte T3 `delegate_task` para modelos fuera del catálogo nativo. `isolation:"worktree"` parte del commit del parent y no copia edits sin commit: verificar el base SHA exacto, en especial para recuperar E2E desde b0f4fcd. No crear conversaciones top-level para simular subagentes.

## Secuencia recomendada

Es una propuesta de scheduling; **no modifica Dependencias ni releases**. Cada fila futura exige promoción canónica, release/alcance resueltos cuando corresponda, snapshot y preflight. El máximo de cuatro lanes de los comandos existentes es un límite, no una obligación de llenarlas.

| Etapa | Trabajo y paralelismo posible | Gate de salida |
| --- | --- | --- |
| 0 · recuperar E2E-001 | Un executor propietario de la integración Math/consumer. Repo-engineer delimita el adapter; implementer escribe después. Mantener congelados T2a/M1/M2 y separar cualquier bookkeeping pendiente | Autorización acotada del siguiente tramo; SHAs, snapshots, permisos y evidencia histórica recuperados |
| 1 · cerrar V0.1 | Adapter/pin/cache exactos → sustitución activa → paridad/rollback. Después, verifier independiente y reviewer L3 sobre candidato inmóvil; primary integra | Gates completos del snapshot, SHAs integrados y reruns; sólo entonces Done y evaluación NEXT_READY_SET |
| 2 · seguimientos, si se promueven | ARC-003 governance-only puede convivir con VV-004 sobre selectores/regenerador. NSGA-005 tiene lane propia; serializar si su lista exacta de rutas/corpus intersecta VV-004 o E2E | ADR, verificación inmutable y sesión transaccional/progreso, cada uno con su evidencia |
| 3 · evidencia y baseline | Tras VV-004: EVD-002 y PERF-002 son candidatos paralelos sólo si reporter/schema/harness no se solapan. Congelar la versión del corpus antes de medir | Reports honestos y baseline reproducible con ruido/allocations/memoria |
| 4 · composición | VV-005 espera NSGA-005+EVD-002+NSGA-004. NSGA-006 espera ARC-003+VV-005+PERF-002. SAFE-001 puede tener lane aparte sólo si sus kernels/contratos no chocan con NSGA-006 | Calidad por sesión pública, defaults compatibles, sustitución demostrada, límites numéricos |
| 5 · automatización y ejemplo | CI-001 espera VV-004+EVD-002+DEP-001; DOC-002 espera NSGA-006+VV-005 | Ejemplo ejecutable y gates automáticos reales. La ejecución de CI alojado necesita autorización de cuota |
| V0.2 | E2E-001 → SENS-001; luego FD-001 y LA-001 pueden separarse; LA-002 y GCHK-001 avanzan por sus propios HARD; SENS-002 y SCALE-001 convergen con LA-002 en VV-002 → E2E-002 | Gate real del backend; un mock de LA-001 no acredita faer |
| V0.3 | Desde E2E-002, cadena ADJ-001/002 y cadena OPT-001/002; OPT-002 permite analytical/FD sin esperar adjoint. CONSTR-001+STOP-001+ADJ-002 convergen en VV-003 → E2E-003 | Validación integrada de gradientes, stops y dominio |
| V0.4 | HYB, PAR y WASM tienen rutas separables después de E2E-003; CHK lateral depende de STOP/EVD. PERF-001 reúne PAR+WASM-002+HYB-002+PERF-002; SNAP+PERF convergen en E2E-004 | Paridad nativo/WASM y beneficio híbrido medido, sin forzar promoción |
| V0.5 / research | Adapters de dominio en sus repos; HAR/DOC convergen en E2E-005. GEO tiene HARD temprano VV-002 pero conserva su release V0.5. GPU/AD/UQ/OPT-EXT sólo Post-V0.5 | Dos dominios sobre mismos contratos; research no equivale a producción |

## Conflictos de escritura que el DAG no expresa

| Superficie compartida | Tickets afectados | Scheduling recomendado |
| --- | --- | --- |
| RNG/variación/selección/raw-vs-checked | E2E-001, NSGA-005/006, SAFE-001 | Congelar/cerrar E2E primero; un escritor por contrato y rerun de compatibilidad |
| Fixtures, selectores, harness, thresholds y reports | VV-004, EVD-002, VV-005, PERF-002, CI-001 | Separar regeneración de verificación; congelar corpus/schema antes de lectores; serializar cambios comunes |
| API pública / `lib.rs` / codec / errores | ARC-003, SENS-001, LA-001, NSGA-006, SAFE-001, STOP-001 | Diseño previo, contratos congelados; primary integra exports y cambios compatibles secuencialmente |
| Workspace manifests, Cargo.lock y registros | Nuevos crates/backends/CI | Un propietario de integración; lanes proponen cambios y no actualizan lockfile simultáneamente |
| Benchmark y mediciones | PERF-002, PERF-001, cambios en kernels | No medir candidatos que cambian ni competir por CPU con otra lane; entorno y SHA fijos |
| Notion, promoción, commits y publicación | Todos | Sólo primary; subagentes entregan propuestas, diff y evidencia |

Estas restricciones son de scheduling, no aristas HARD inventadas. Se calculan de nuevo con la lista de rutas real en cada preflight; esta inspección no puede garantizar de antemano que dos futuros diffs no se solapen.

## Contrato mínimo de cada despacho futuro

1. Ticket/URL, Estado y HARD consultados en vivo; release y STOP vigentes revisados. Congelar scope/DoD/acceptance/risk/MATH_VERIFY, contratos, tolerancias, seeds, referencias y assumptions. En E2E conservar el snapshot existente y registrar sólo el tramo autorizado.
2. Base SHA exacta, branch/worktree exclusivos y lista de rutas permitidas. Un ticket por executor/worktree; en integración de dos repos, un único propietario y un par explícito de SHAs. No compartir el checkout del usuario.
3. Separar propuesta arquitectónica, implementación, verificación numérica independiente y review L3. Verifier/reviewer trabajan con candidato inmóvil. Un PASS LLM no sustituye oracle/tests.
4. Devolver diff/manifest, comandos y exits reales, hashes/artefactos, limitaciones y estado VERIFIED/FAILED/NOT RUN/UNAVAILABLE. Implementers no hacen commit/push/merge ni escriben Notion.
5. Máximo 1 fix + 1 re-review; fallos posteriores → HUMAN_DECISION_REQUIRED. El presupuesto ya consumido de E2E no se reinicia al recuperar el trabajo ni al cambiar executor.
6. Primary integra por ticket, valida el SHA integrado, publica sólo dentro de autorización y actualiza Notion con evidencia. NEXT_READY_SET es informativo hasta promoción; detenerse al terminar el alcance autorizado.

Los comandos `.opencode/commands/math-wave-launch.md` y `math-wave-collect.md` ya describen preflight y recolección; no se ejecutaron. Esta preparación no crea otro orquestador ni instala un sincronizador.

## MIT y validación de esta preparación

El usuario solicitó MIT. Se añade el texto raíz y se cambia `workspace.package.license` de `MIT OR Apache-2.0` a `MIT`; los dos packages heredan esa propiedad. Es una modificación administrativa explícita; no reescribe el freeze histórico DEP-001 ni relicencia materiales de terceros. La futura matriz de release debe registrar la decisión actual y sus propios avisos de distribución.

Ver [recibo de validación](VALIDATION.md) para comandos realmente ejecutados y límites. No se acredita MATH_VERIFY ni V0.1 con este DAG.
