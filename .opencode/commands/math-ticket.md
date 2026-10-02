---
description: Prepare exactly one MAMUT Math ticket from Notion without executing it.
agent: mamut-math
---

Target ticket: $ARGUMENTS

Perform ticket preparation only, in this order:
1. Query the canonical base "MAMUT Math · Tickets" and fetch the ticket page. Fail with a clear report if the ticket does not exist.
2. Validate gates: only "Estado = Siguiente" may proceed, and every HARD dependency must be "Done". Otherwise stop and report BLOCKED with the unmet dependency list.
3. Freeze the immutable execution snapshot exactly as defined in AGENTS.md sections 2 and the primary prompt (goal, acceptance criteria, DoD, risk level, dependencies, frozen contracts, MATH_VERIFY, METHOD_REGISTRY, provenance, independent validation with tolerance, references, assumptions). Do not invent or modify scope.
4. Verify the base SHA on main (git log/status). Create exactly one worktree and one branch ticket/<TICKET-ID>-<slug> from that verified base SHA. Never reuse another worktree or branch.
5. STOP before any implementation. Report: snapshot, base SHA, worktree path, branch, and the exact next step for the user to authorize execution.

Do not launch subagents in this command. Do not write implementation code. Do not update Notion state.
