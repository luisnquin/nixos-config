---
name: dev-flow
description: Default loop for a bug or change. Scope to what was asked, reproduce on the real target, fix the cause, instrument instead of guessing after two failed attempts, prove with before/after evidence, then deliver only as far as allowed. Use for any bug fix or feature implementation.
---

# Dev flow

The default loop for a bug or a change: scope, reproduce, fix, prove, deliver.
Do not skip a step. Do not take one the user did not allow.

## 1. Scope

- Restate what was asked in one line. Do only that. A related problem you notice
  is a line in the report, not an edit.
- A question ("wdyt", "what's next", "¿tiene sentido?") gets an answer, not an
  implementation.
- Before writing new code, find the repo's existing pattern, component, hook or
  helper that does the same thing, and follow it.
- Note what is allowed for this task: build, eval, tests, dev server, device,
  deploy, commit. "Don't build" and "no tests" hold until the user lifts them.

## 2. Reproduce

- Trigger the bug on the real target before touching code: a test, the CLI, or
  a device through `phone`. Write down the exact steps and the observed output.
- If it does not reproduce, say so and stop. Never fix blind.

## 3. Fix

- Make the smallest change that removes the cause, not the symptom.
- No hardcoding to make one case pass. No new abstraction unless asked.
- **Failed twice** ("still the same", "sigue igual", "nada"): stop guessing.
  1. Add instrumentation (logs, a debug flag, assertions) at the boundaries of
     the suspected path.
  2. Rerun the reproduction and read the output.
  3. Fix from evidence, then remove the instrumentation.

## 4. Prove

- Rerun the exact reproduction from step 2 and show it now passes.
- Prefer a regression test that fails without the fix, if tests are allowed.
- For UI, show a before and after screenshot from the same device.
- Run the repo's check command (lint and typecheck).
- Say "passes locally", not "passes".

## 5. Deliver

- Commit with the `commit` skill. Commit only what this task touched. Make one
  commit per concern.
- For a host or app change, deliver only as far as allowed: rebuild the host,
  install on a device, or deploy. Then check that the live binary or app runs
  the new code.
- Work from a ticket closes with the before/after proof posted on the ticket,
  in the format the project's skill defines. Do not change its state.
- Report:
  - what changed;
  - the proof;
  - anything left behind, recorded with techdebt-mcp if it is debt;
  - anything noticed out of scope.
