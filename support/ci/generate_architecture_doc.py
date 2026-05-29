#!/usr/bin/env python3

from __future__ import annotations

import json
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
MODEL_PATH = REPO_ROOT / "support/ci/architecture-model.json"
PREVIOUS_PATH = REPO_ROOT / "support/ci/architecture-previous.json"
OUTPUT_PATH = REPO_ROOT / "docs/architecture.md"


def load_json(path: Path) -> dict:
    with path.open() as fh:
        return json.load(fh)


def current_component_paths() -> list[str]:
    components_dir = REPO_ROOT / "components"
    return sorted(
        str(path.relative_to(REPO_ROOT))
        for path in components_dir.iterdir()
        if path.is_dir()
    )


def render_mermaid(model: dict) -> list[str]:
    style_map = {"solid": "-->", "dashed": "-.->"}
    group_order = [group["id"] for group in model["groups"]]
    groups = {group["id"]: group["label"] for group in model["groups"]}
    components = {component["id"]: component for component in model["components"]}

    lines = ["```mermaid", "flowchart LR"]
    for group_id in group_order:
        lines.append(f'    subgraph {group_id}["{groups[group_id]}"]')
        for component in sorted(
            (component for component in model["components"] if component["group"] == group_id),
            key=lambda item: item["path"],
        ):
            lines.append(
                f'        {component["id"]}["{component["path"]}/<br/>{component["label"]}"]'
            )
        lines.append("    end")
        lines.append("")

    for edge in model["edges"]:
        connector = style_map[edge["style"]]
        lines.append(
            f'    {edge["from"]} {connector}|"'
            f'{edge["label"]}"| {edge["to"]}'
        )

    lines.append("```")
    return lines


def current_edge_snapshot(model: dict) -> list[tuple[str, str, str, str]]:
    component_paths = {component["id"]: component["path"] for component in model["components"]}
    return sorted(
        (
            component_paths[edge["from"]],
            component_paths[edge["to"]],
            edge["label"],
            edge["style"],
        )
        for edge in model["edges"]
    )


def render_change_summary(model: dict, previous: dict) -> list[str]:
    current_components = {component["path"] for component in model["components"]}
    previous_components = set(previous["components"])
    added_components = sorted(current_components - previous_components)
    removed_components = sorted(previous_components - current_components)

    current_edges = set(current_edge_snapshot(model))
    previous_edges = {
        (edge["from"], edge["to"], edge["label"], edge["style"]) for edge in previous["edges"]
    }
    added_edges = sorted(current_edges - previous_edges)
    removed_edges = sorted(previous_edges - current_edges)

    lines = [
        "## Change summary since the last documented snapshot",
        "",
        "- Automation: this document is now generated from "
        "`support/ci/architecture-model.json` by "
        "`support/ci/generate_architecture_doc.py` and freshness-checked in CI.",
    ]

    if added_components:
        lines.append(
            "- Added component coverage: "
            + ", ".join(f"`{path}/`" for path in added_components)
            + "."
        )
    if removed_components:
        lines.append(
            "- Removed component coverage: "
            + ", ".join(f"`{path}/`" for path in removed_components)
            + "."
        )
    if added_edges:
        edge_descriptions = [
            f"`{source}/` -> `{target}/` ({label})" for source, target, label, _ in added_edges
        ]
        lines.append("- Added flow coverage: " + "; ".join(edge_descriptions) + ".")
    if removed_edges:
        edge_descriptions = [
            f"`{source}/` -> `{target}/` ({label})" for source, target, label, _ in removed_edges
        ]
        lines.append("- Removed flow coverage: " + "; ".join(edge_descriptions) + ".")

    if not any([added_components, removed_components, added_edges, removed_edges]):
        lines.append(
            "- No component-path or flow shifts were detected relative to the stored snapshot."
        )

    return lines


def render_doc(model: dict, previous: dict) -> str:
    repo_components = current_component_paths()
    modeled_components = sorted(component["path"] for component in model["components"])
    missing_components = sorted(set(repo_components) - set(modeled_components))
    if missing_components:
        missing = ", ".join(missing_components)
        raise SystemExit(f"architecture model is missing components: {missing}")

    lines = [
        f'# {model["title"]}',
        "",
        model["intro"],
        "",
    ]
    lines.extend(render_change_summary(model, previous))
    lines.extend(["", "## Diagram", ""])
    lines.extend(render_mermaid(model))
    lines.extend(["", "## Data flows", ""])

    for index, flow in enumerate(model["data_flows"], start=1):
        lines.append(f"{index}. {flow}")

    lines.extend(["", "## Notes", ""])
    for note in model["notes"]:
        lines.append(f"- {note}")

    lines.append("")
    return "\n".join(lines)


def main() -> None:
    model = load_json(MODEL_PATH)
    previous = load_json(PREVIOUS_PATH)
    OUTPUT_PATH.write_text(render_doc(model, previous))


if __name__ == "__main__":
    main()
