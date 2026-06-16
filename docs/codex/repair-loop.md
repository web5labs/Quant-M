# Repair Loop

Use this only when the current slice fails validation or review.

## Loop

1. Read the failing diff or failing scope only.
2. Read test failures and reviewer feedback.
3. Patch only the failing scope.
4. Add or update regression coverage when practical.
5. Re-run the relevant validation commands.
6. Stop when the threshold is met or document the blocker.
