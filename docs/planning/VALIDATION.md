# Validación de la preparación · 2026-11-02

Ámbito: licencia, documentación y generador del DAG. Baseline `19a316d251da60f9f3398c34e1855481cf15199b`; ejecución en worktree aislado `mamut-math-preparation-20261102`. Este recibo no cierra tickets ni acredita matemática o release.

| Check | Estado | Evidencia observada |
| --- | --- | --- |
| Consulta canónica y fetch individual | VERIFIED | 59 filas, has_more=false, 59 cuerpos recuperados; diez propiedades contrastadas por ticket; 13 referencias recuperadas |
| Estados / releases | VERIFIED | 11 Done, 1 Siguiente (E2E-001), 47 Planeado; diez seguimientos conservan Release=null; ninguna escritura en Notion |
| `python scripts/planning/build_dag.py` | VERIFIED | 59 nodos, 113 HARD, 3 SOFT internas, 3 externas; identidades/órdenes únicos, referencias válidas y HARD acíclico |
| `python scripts/planning/build_dag.py --check` | VERIFIED | Outputs iguales a regeneración; no escribe en este modo |
| Casos adversariales del generador | VERIFIED | Rechazó ciclo HARD, referencia faltante HARD/SOFT, orden duplicado y consulta incompleta |
| Comparación independiente desde filas vivas | VERIFIED | Aristas generadas = Dependencias canónicas = diagrama detallado 08; macro 05 tiene 77 HARD y conserva exactamente la clausura transitiva; SOFT iguales |
| Contraste con espejo local previo | VERIFIED | 59 marcas de última edición iguales a los fetches vivos; 113 HARD y 3 SOFT del DAG previo iguales al generado |
| Preservación de trabajo previo | VERIFIED | 84 archivos registrados con SHA-256 antes de editar; sin cambios al contrastar después de generar; `.codegraph` incluido en ese inventario |
| Cargo metadata | VERIFIED | `cargo metadata --no-deps --locked --offline --format-version 1`; ambos packages heredan MIT y MSRV declarado 1.86.0 |
| Compilación de desarrollo | VERIFIED | `cargo check --workspace --locked --offline`, exit 0; rustc 1.95.0 / cargo 1.95.0, Windows |
| Alcance Rust | VERIFIED | Sin cambios en `.rs`, lockfile, fixtures ni algoritmos; sólo metadata de licencia en Cargo.toml |
| `git diff --check` | VERIFIED | Sin errores de whitespace en el diff observado |
| Enlaces locales Markdown | VERIFIED | Todos los destinos locales de README y docs/planning existen tras completar este recibo |
| Vista interactiva HTML | VERIFIED | T3 html_preview a 728 y 360 px; inspección visual y console sin errores; selección, 3 prerequisitos de NSGA-006, guard de Planeado, navegación por arista y filtro de 10 seguimientos probados en navegador |
| Rust tests, fmt/clippy, doc/release, MSRV/32-bit/Linux/WASM | NOT RUN | No hay cambios Rust; el check ejecutado no prueba esos targets/profiles |
| Oráculos / corpus estocástico / MATH_VERIFY / benchmarks | NOT RUN | No se certifica resultado numérico ni rendimiento; los resultados de tickets son históricos |
| Regeneración de fixtures / selector slow amplio | NOT RUN | No se ejecuta el regenerador ni el comando que también lo selecciona |
| CI remoto y llamadas a subagentes | NOT RUN | No se habilitaron workflows ni se gastó cuota de ejecución alojada/modelos hijos |
| Gates E2E / consumidor externo | NOT RUN | Checkpoint b0f4fcd preservado; no integración, publicación o Done del ticket |

La primera comparación auxiliar de fechas usó un literal acentuado por stdin de PowerShell y falló por encoding del literal; se repitió extrayendo fechas ISO y pasó para 59/59. La primera revisión de enlaces detectó este recibo aún pendiente de crear; se completa antes del cierre. Estos resultados auxiliares no se confunden con fallos matemáticos ni con el presupuesto de corrección del ticket E2E, que no se ejecutó.

La sincronización final y su SHA exacto se reportan en la entrega, fuera del commit para evitar un recibo que se auto-referencia. Se conserva el trabajo previo sin incluirlo en el commit.
