PATH: .github/instructions/exclude-submodules.instructions.md
applyTo:
  - "**/*"
exclude:
  - "vendor/**"
  - "third_party/**"
  - "**/.git/modules/**"
  - paths listed in .gitmodules
rules:
  - "Do not propose or apply edits in excluded paths; treat them as read-only references."
