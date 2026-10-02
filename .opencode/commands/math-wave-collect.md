---
description: Collect and gate the evidence of a launched MAMUT Math wave, ticket by ticket.
agent: mamut-math
---

Wave collection scope: $ARGUMENTS (empty = all currently active wave tickets)

Collect the wave results and drive them through the gates without re-deciding frozen scope:
1. Enumerate active wave worktrees (git worktree list) and match them to tickets. Report the state of each lane: clean/dirty, commits ahead of base, and last known executor status.
2. For each lane, gather deterministic evidence first (cargo fmt/check/clippy/test, ticket-specific gates, oracle/fixture comparisons, MATH_VERIFY requirements from the frozen snapshot). An LLM PASS never substitutes required tests or oracles.
3. Run the mandatory routing chain per ticket level (implementer/repo-engineer, verifier, reviewer for L3 or material MATH_VERIFY=ON), passing the identical snapshot. Respect the bounded fix cycle; if a lane exceeds it, mark that lane STATUS: HUMAN_DECISION_REQUIRED and continue collecting the others.
4. Produce a wave report table: ticket, lane verdict, evidence links/artifact hashes, integration readiness (Yes/No/Blocked), and remaining blockers.
5. Integration, merges, pushes, Notion closure, and worktree removal stay with the primary under project policy and require explicit human direction; this command only prepares and reports the collected gate evidence.

Do not modify code in this command. Do not remove any worktree here.
