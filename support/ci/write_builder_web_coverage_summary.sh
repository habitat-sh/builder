#!/usr/bin/env bash

set -euo pipefail

summary_file="${1:-}"
log_file="${2:-}"
summary_target="${GITHUB_STEP_SUMMARY:-/dev/stdout}"

{
  echo "## Builder-web coverage evidence"
  echo
  echo "> Informational only: this workflow does not block merges."
  echo

  if [[ -n "$summary_file" && -f "$summary_file" ]]; then
    node - "$summary_file" <<'NODE'
const fs = require("fs");

const summaryPath = process.argv[2];
const summary = JSON.parse(fs.readFileSync(summaryPath, "utf8")).total;
const metrics = [
  ["Lines", "lines"],
  ["Statements", "statements"],
  ["Branches", "branches"],
  ["Functions", "functions"]
];

function formatPct(value) {
  const pct = Number(value);
  return Number.isFinite(pct) ? `${pct.toFixed(1)}%` : String(value);
}

console.log("| Metric | Coverage | Covered/Total |");
console.log("| --- | ---: | ---: |");

for (const [label, key] of metrics) {
  const metric = summary[key];
  console.log(`| ${label} | ${formatPct(metric.pct)} | ${metric.covered}/${metric.total} |`);
}
NODE
    echo
    echo "- Coverage summary source: \`$summary_file\`"
  else
    echo "Coverage summary was not produced."
  fi

  if [[ -n "$log_file" && -f "$log_file" ]]; then
    echo
    echo "<details><summary>Coverage command log tail</summary>"
    echo
    echo '```text'
    tail -n 40 "$log_file"
    echo '```'
    echo "</details>"
  fi
} >> "$summary_target"
