---
name: squad
description: Fan findings, tickets or tasks out to subagents, an implementer and a reviewer per item that iterate until the reviewer approves and lands one commit on main, with proof and ticket evidence; the lead spot-checks what landed. Use when asked to use subagents, a squad, or to triage and merge a batch of items.
---

# Squad

Fan a list of items out to subagents and land the approved ones on main. Items
can be findings, tickets or tasks. You are the lead: you split, brief and
spot-check. You do not write the fix yourself.

## Steps

1. **Split.** Make one work item per finding or ticket. Merge duplicates.
2. **Brief the implementer.** Each one gets:
   - the item, and what you already verified about its cause;
   - the files involved, with line anchors;
   - the repo's check and test commands;
   - "do not commit, do not touch devices".
   Use the model the user names for every subagent. Give each implementer its
   own worktree (`isolation: "worktree"`), unless items touch disjoint files —
   then a shared tree is fine, with every commit path-staged.
3. **Verdict.** The implementer first checks that the item is still real against
   current main, then returns one of:
   - `FIX`: the diff, plus proof. Proof is a repro before and after, a test that
     fails without the change, or a screenshot pair.
   - `SKIP`: the evidence that the item is invalid, a duplicate or already fixed.
   - `DECIDE`: a product or scope call only the user can make, stated in one line.
4. **Review with a reviewer subagent.** As each implementer finishes, start a
   reviewer for that item. It:
   - verifies the implementer's claims independently (contracts, callers,
     schema), not from the report;
   - rejects missing proof, scope creep, broken conventions, files that belong
     to another item;
   - sends change requests to the implementer with `SendMessage` and re-reviews,
     or fixes small things itself; loops until it would approve;
   - on approval, lands one commit per item on main — path-staged, own files
     only, following the repo's convention. Pushes only where the user allows it.
5. **Evidence.** Shared devices get ONE evidence agent for all items that need
   them, so nobody fights over a device. Items from a ticket close with the
   evidence on the ticket, in the format the project's skill defines (e.g.
   `linear-flow`). Never change ticket states.
6. **Spot-check.** Read every landed commit yourself. Re-run the tests on any
   commit a reviewer rebased before pushing.
7. **Report.** Give a table: item, verdict, commit hash or reason. List the
   `DECIDE` lines and out-of-scope findings last.

## Rules

- Do not wait for approval between items. Approval was given by starting the squad.
- A `SKIP` with evidence is a good outcome. Never fix something to justify a run.
- Kill a subagent that loops or digs past its item. Tokens are the budget.
- Devices: the ones the project names (rose emulators), never nyx or the
  personal phone.
