#!/usr/bin/env python3
"""Compare two saved pdfp evaluation result JSON files.

The comparator is intentionally schema-light: it walks both JSON documents,
finds numeric leaf values, and reports changed metrics by JSON path. This works
for `scripts/quality-report.sh`, `run_omni_doc_bench.py`, and
`run_table_bench.py` outputs without making their schemas depend on each other.

Usage:
    python tools/eval_benchmarks/compare.py old.json new.json
    python tools/eval_benchmarks/compare.py old.json new.json --json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


Number = int | float


def numeric_leaves(value: Any, prefix: str = "") -> dict[str, Number]:
    """Return JSON paths for numeric scalar leaves.

    Booleans are excluded even though `bool` is an `int` subclass in Python.
    Lists are indexed with `[N]` so per-file metrics can still be compared when
    result ordering is stable.
    """
    leaves: dict[str, Number] = {}
    if isinstance(value, dict):
        for key, child in value.items():
            child_prefix = f"{prefix}.{key}" if prefix else str(key)
            leaves.update(numeric_leaves(child, child_prefix))
    elif isinstance(value, list):
        for idx, child in enumerate(value):
            leaves.update(numeric_leaves(child, f"{prefix}[{idx}]"))
    elif isinstance(value, (int, float)) and not isinstance(value, bool):
        leaves[prefix] = value
    return leaves


def compare(old: dict[str, Any], new: dict[str, Any]) -> list[dict[str, Any]]:
    old_metrics = numeric_leaves(old)
    new_metrics = numeric_leaves(new)
    rows = []
    for path in sorted(set(old_metrics) | set(new_metrics)):
        old_value = old_metrics.get(path)
        new_value = new_metrics.get(path)
        if old_value == new_value:
            continue
        delta = None
        percent = None
        if old_value is not None and new_value is not None:
            delta = new_value - old_value
            if old_value != 0:
                percent = delta / old_value * 100.0
        rows.append(
            {
                "path": path,
                "old": old_value,
                "new": new_value,
                "delta": delta,
                "percent": percent,
            }
        )
    return rows


def load_json(path: Path) -> dict[str, Any]:
    with path.open() as f:
        value = json.load(f)
    if not isinstance(value, dict):
        raise SystemExit(f"{path} must contain a top-level JSON object")
    return value


def print_table(rows: list[dict[str, Any]]) -> None:
    if not rows:
        print("No numeric metric changes.")
        return
    print("metric\told\tnew\tdelta\tpercent")
    for row in rows:
        percent = "" if row["percent"] is None else f"{row['percent']:.2f}%"
        delta = "" if row["delta"] is None else row["delta"]
        print(f"{row['path']}\t{row['old']}\t{row['new']}\t{delta}\t{percent}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare saved pdfp evaluation JSON results")
    parser.add_argument("old", type=Path, help="Baseline JSON result")
    parser.add_argument("new", type=Path, help="New JSON result")
    parser.add_argument("--json", action="store_true", help="Emit machine-readable comparison JSON")
    args = parser.parse_args()

    rows = compare(load_json(args.old), load_json(args.new))
    if args.json:
        print(json.dumps({"changes": rows, "change_count": len(rows)}, indent=2))
    else:
        print_table(rows)


if __name__ == "__main__":
    main()
