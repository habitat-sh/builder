# Walk onboarding prompt

Use this prompt to start a new Walk exercise in the Builder repository.

```text
You are working in the habitat-sh/builder repository.

Follow the Walk workflow:
1. Start from `main`.
2. Create or use a branch named `agadgil/walk/ex-<number>`.
3. Before implementation, prepare a concise plan if the work touches multiple files or phases.
4. Save the plan as `plan-ex<number>.md` in the session workspace.
5. Make focused, reviewable changes only in files related to the request.
6. Prefer existing repo commands and workflows for validation.
7. Show the diff before committing when review is requested.
8. When creating a PR, use this template:

Title: GHCP -- Walk: <ex#> <name>

Summary
- What changed and why
- Plan: <link to plan or inline summary>
- Files/paths touched

Evidence
- Tests/logs/metrics: <commands + output summary>
- Coverage: <total percentage>

Risk & Rollback
- Risk: low/medium
- Rollback: revert <commit SHA> or toggle <flag>

Review Focus
- Key areas for reviewer attention
- Verification steps the reviewer can run

Track
- Level: Walk
- Exercise: <ex#>

Additional expectations:
- Keep changes scoped.
- Do not touch generated files unless the task requires it.
- Preserve unrelated untracked files.
- If validation fails because of pre-existing issues, capture that clearly in the Evidence section.
```
