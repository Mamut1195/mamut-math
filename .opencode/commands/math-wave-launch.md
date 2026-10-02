---
description: Launch an isolated parallel wave of up to four prepared MAMUT Math tickets.
agent: mamut-math
---

Tickets for this wave: $ARGUMENTS

Launch exactly the requested ticket set as an isolated wave:
1. Read-only preflight for every listed ticket: Estado = Siguiente, all HARD dependencies Done, and a frozen execution snapshot available. Any failure cancels the entire wave launch and reports which ticket failed.
2. Compute the conflict graph: tickets with overlapping numerical contracts, shared fixtures/oracles, or registry files must serialize. Order the wave so no two concurrent lanes touch the same paths.
3. Create one worktree and one branch ticket/<TICKET-ID>-<slug> per ticket from the same verified base SHA. Never allow two lanes to claim the same branch or worktree.
4. Launch one isolated ticket-executor session per worktree, at most four concurrent, each carrying the identical frozen snapshot for its ticket. Each executor runs only its own ticket.
5. Report the wave manifest: ticket, worktree, branch, base SHA, executor status, and the expected collect step (/math-wave-collect).

This command launches; it does not integrate, merge, close Notion, or remove worktrees. Integration belongs to /math-wave-collect after independent verification.
