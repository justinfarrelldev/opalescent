# Task 8 pre-fix executable-stress RED feasibility note

## Why pre-fix executable-stress RED was not feasible
By the time Task 8 executable-stress verification was added and finalized, the in-scope leak fixes from Tasks 4 and 7 were already present in the working tree. That meant the ignored executable stress command was no longer available on a pre-fix tree state inside this remediation branch, so a faithful pre-fix `game_of_life_full_memory_stress` RED capture was not feasible without rewinding implementation work outside the allowed F1/F4 remediation scope.

## What served as the real RED drivers instead
The plan's deterministic RED drivers for the two confirmed leak classes were the targeted regressions introduced earlier in the wave:

- RC store leak RED: `.sisyphus/evidence/task-2-rc-store-red.txt`
- Call-temp direct interpolation RED: `.sisyphus/evidence/task-6-direct-interpolation-red.txt`
- Call-temp propagate-path RED: `.sisyphus/evidence/task-6-propagate-red.txt`

Those targeted regressions are the evidence that the leak classes were actually failing before the fixes. Task 8's executable stress test is the bounded end-to-end GREEN guard for the real Game of Life executable after the in-scope fixes, not the primary RED signal.

## Current GREEN references
- Timeout-bounded explicit stress verification: `.sisyphus/evidence/task-8-stress-timeout.txt`
- Passing explicit stress rerun: `.sisyphus/evidence/task-8-stress-green.txt`

## Remediation note
This file exists to satisfy the Task 8 acceptance wording in the plan and the final-wave F1 blocker: when pre-fix executable-stress RED is not feasible in the current tree, the evidence must explicitly state that fact and point to the targeted RED regressions that were the actual RED drivers.
