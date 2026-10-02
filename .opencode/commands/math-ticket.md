---
description: Execute exactly one MAMUT Math ticket end-to-end via mamut-math.
agent: mamut-math
---

Target ticket: $ARGUMENTS

Execute exactly this one ticket end-to-end. Never start a second ticket; this is not a wave.

1. Query the canonical base "MAMUT Math · Tickets" and fetch the ticket page. Fail with a clear report if the ticket does not exist.
2. Verify gates: every HARD dependency must be "Done". If "Estado != Siguiente", do NOT start it and return exactly: TICKET_NOT_STARTABLE. Stop.
3. If "Estado = Siguiente", freeze the immutable execution snapshot exactly as defined in AGENTS.md and the primary prompt (goal, acceptance criteria, DoD, risk level, dependencies, frozen contracts, MATH_VERIFY, METHOD_REGISTRY, provenance, independent validation with tolerance, references, assumptions). Do not invent or modify scope.
4. If repo-mutating: run git fetch origin; freeze the base SHA; create exactly one branch ticket/<TICKET-ID>-<slug> and one isolated worktree from that base SHA. Never work directly on main. Never reuse another worktree or branch.
5. Execute the ticket using the mamut-math subagents per mandatory routing with the identical snapshot: implementer always; repo-engineer only for cross-crate/architecture work; verifier when MATH_VERIFY = ON or numerical semantics are involved; reviewer for L3 or material MATH_VERIFY = ON.
6. Run deterministic gates first (format, lint, type, build, unit/integration, oracle/fixture comparisons as the snapshot requires). An LLM PASS never substitutes required tests or oracles.
7. Respect the bounded fix cycle: at most one fix plus one re-verification/re-review; if blockers remain, return STATUS: HUMAN_DECISION_REQUIRED. Never loop again.
8. If everything is green: commit the ticket in its worktree; push the ticket branch; merge sequentially into main; rerun the gates on main; push main; update the Notion ticket to Done with evidence and SHAs.
9. Query NEXT_READY_SET for the report only. STOP. Do NOT start the next ticket.
