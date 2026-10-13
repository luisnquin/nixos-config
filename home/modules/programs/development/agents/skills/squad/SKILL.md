---
name: squad
description: Fan findings, tickets or tasks out to subagents in separate worktrees, each re-validating its item and returning FIX/SKIP/DECIDE with proof; the lead reviews and lands approved work on main, one commit per item. Use when asked to use subagents, a squad, or to triage and merge a batch of items.
---

# Squad

Fan a list of items out to subagents and land the approved ones on main. Items
can be findings, tickets or tasks. You are the lead: you brief, review and
merge. You do not write the fix yourself.

## Steps

1. **Split.** Make one work item per finding or ticket. Merge duplicates.
2. **Brief.** Each subagent gets:
   - the item;
   - the files involved;
   - the device or host it may use (rose emulators, never nyx or the personal phone);
   - the repo's check command.
   Each subagent gets its own worktree (`isolation: "worktree"`).
3. **Run.** At most two subagents at once unless the user says otherwise.
4. **Verdict.** Each subagent first checks that the item is still real against
   current main, then returns one of:
   - `FIX`: the diff, plus proof. Proof is a repro before and after, a test that
     fails without the change, or a screenshot pair.
   - `SKIP`: the evidence that the item is invalid, a duplicate or already fixed.
   - `DECIDE`: a product or scope call only the user can make, stated in one line.
5. **Review.** Read every `FIX` diff yourself. Reject it when:
   - the proof is missing;
   - the change goes beyond the item;
   - it breaks repo conventions;
   - the diff touches files that belong to another item.
   Send a rejected fix back once with the reason. Drop it on the second failure.
6. **Land.** Rebase approved work onto main one item at a time. Run the check
   command after each. Make one commit per item, and squash any review fixes
   into it. Do not push. Remove the worktree.
7. **Report.** Give a table: item, verdict, commit hash or reason. List the
   `DECIDE` lines last.

## Rules

- Do not wait for approval between items. Approval was given by starting the squad.
- A `SKIP` with evidence is a good outcome. Never fix something to justify a run.
- Kill a subagent that loops or digs past its item. Tokens are the budget.
