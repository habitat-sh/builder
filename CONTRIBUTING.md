# Contributing to a Progress Chef Habitat Project

Thank you for your interest in contributing to this project! It is part of the larger Progress Chef Habitat project. Contribution guidelines can be found at [Contributing to Progress Chef Habitat](https://chef.github.io/chef-oss-practices/projects/habitat/contributing/).

## Walk workflow

This repository also uses a lightweight **Walk** workflow for scoped, reviewable exercises and
small documentation or code changes.

### Branches and plans

1. Start from `main`.
2. Create a branch named `agadgil/walk/ex-<number>`.
3. Save exercise plans in the session workspace as `plan-ex<number>.md`.

### Expected sequence

1. Analyze the request and identify the files that will change.
2. Write a short plan before implementation when the work spans multiple steps or files.
3. Make the smallest complete change that satisfies the exercise.
4. Validate the result with the closest existing command or workflow.
5. Review the diff before committing.

### PR format

Use the Walk PR structure below when preparing a pull request:

**Title**

`GHCP -- Walk: <ex#> <name>`

**Body**

```text
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
```

### Onboarding prompt

For a reusable starting prompt, see [docs/walk-onboarding-prompt.md](docs/walk-onboarding-prompt.md).