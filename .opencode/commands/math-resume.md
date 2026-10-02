---
description: Resume exactly one in-flight MAMUT Math ticket from its preserved worktree and state.
agent: mamut-math
---

Target ticket: $ARGUMENTS

Resume the existing ticket execution, continuing exactly from preserved state:
1. Locate the worktree and branch ticket/<TICKET-ID>-<slug> via git worktree list. If none exists, stop and report NO_EXISTING_WORKTREE; use /math-ticket instead. Never create a second worktree, never change branch, never reset or discard files.
2. Require git status in the worktree; preserve all pre-existing changes and report them.
3. Rehydrate the frozen execution snapshot from the Notion ticket page (Estado, Risk, MATH_VERIFY, dependencies, execution record). The snapshot is not renegotiated on resume.
4. Determine the exact resume point from the ticket execution record and current evidence: last completed routing step, open verifier/reviewer findings, fix_round and review_round counters.
5. Continue the mandatory routing for the ticket level with the identical snapshot, respecting the bounded fix cycle (at most one fix plus one re-verification/re-review; then STATUS: HUMAN_DECISION_REQUIRED).
6. Report the resume point, actions taken, and remaining gate chain.

Do not repeat reconnaissance or completed work. Do not touch other tickets or worktrees.
