# Crawl Notes

Use crawl work to support **small, chained PRs** instead of one large batch. When PRs depend on each other, make that explicit in the description and link the parent/child sequence so reviewers can land them in order.

Every PR should carry **evidence** for the change: commands run, outputs that matter, screenshots when UI changes are involved, and file or code references that justify the update. Keep the evidence brief but concrete enough that another reviewer can trace the claim.

For **prompt usage**, keep prompts scoped and operational: state the goal, branch or PR context, constraints, affected paths, and the evidence you expect back. Prefer prompts that ask for one reviewable step at a time over broad "fix everything" requests.
