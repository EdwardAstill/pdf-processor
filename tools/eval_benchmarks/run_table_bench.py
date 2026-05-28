#!/usr/bin/env python3
"""Run standard table extraction benchmarks against pdfp output.

Usage:
    python tools/eval_benchmarks/run_table_bench.py \
        --pdfp path/to/pdfp \
        --benchmark rd-tablebench \
        --output results.json

Supported benchmarks:
    - rd-tablebench: Reducto's RD-TableBench

This helper records reproducible pdfp table counts and shapes for a local
RD-TableBench checkout. It does not download datasets or claim full GriTS/TEDS
scoring.
"""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


def run_pdfp(pdfp_bin: str, pdf_path: Path, output_dir: Path) -> Path:
    """Run pdfp convert and return path to the output markdown file."""
    output_dir.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [pdfp_bin, "convert", str(pdf_path), "-o", str(output_dir)],
        check=True,
        capture_output=True,
        text=True,
    )
    # pdfp produces {output_dir}/{pdf_stem}/{pdf_stem}.md
    stem = pdf_path.stem
    md_path = output_dir / stem / f"{stem}.md"
    if not md_path.exists():
        # Try without subdirectory
        candidates = list(output_dir.glob("**/*.md"))
        if candidates:
            return candidates[0]
        raise FileNotFoundError(f"No markdown output found in {output_dir}")
    return md_path


def extract_tables_from_markdown(md_path: Path) -> list[list[list[str]]]:
    """Extract GFM tables from pdfp markdown output.

    Returns list of tables, each table is list of rows, each row is list of cells.
    """
    tables = []
    current_table = []
    in_table = False

    with open(md_path) as f:
        for line in f:
            line = line.rstrip()
            if line.startswith("|") and line.endswith("|"):
                # Skip separator rows (|---|---|)
                if all(c in "|-: " for c in line.strip("| ")):
                    continue
                cells = [c.strip() for c in line.strip("|").split("|")]
                current_table.append(cells)
                in_table = True
            else:
                if in_table and current_table:
                    tables.append(current_table)
                    current_table = []
                in_table = False

    if in_table and current_table:
        tables.append(current_table)

    return tables


def run_rd_tablebench(pdfp_bin: str, output_path: Path) -> dict:
    """Return a skipped result when no RD-TableBench path is provided."""
    results = {
        "benchmark": "rd-tablebench",
        "status": "not_downloaded",
        "message": (
            "RD-TableBench dataset must be downloaded from "
            "https://github.com/reductoai/rd-tablebench. "
            "Set --rd-tablebench-path to the dataset directory."
        ),
    }
    return results


def main():
    parser = argparse.ArgumentParser(description="Run table extraction benchmarks on pdfp")
    parser.add_argument("--pdfp", default="pdfp", help="Path to pdfp binary")
    parser.add_argument(
        "--benchmark",
        choices=["rd-tablebench"],
        default="rd-tablebench",
    )
    parser.add_argument("--output", default="table_bench_results.json", help="Output JSON path")
    parser.add_argument("--rd-tablebench-path", help="Path to RD-TableBench dataset")
    args = parser.parse_args()

    results = {}

    if args.benchmark == "rd-tablebench":
        if not args.rd_tablebench_path:
            results = run_rd_tablebench(args.pdfp, Path(args.output))
        else:
            dataset = Path(args.rd_tablebench_path)
            pdfs = list(dataset.glob("**/*.pdf"))
            if not pdfs:
                print(f"No PDFs found in {dataset}", file=sys.stderr)
                sys.exit(1)

            with tempfile.TemporaryDirectory() as tmpdir:
                tmpdir = Path(tmpdir)
                table_results = []
                for pdf_path in pdfs:
                    try:
                        md_path = run_pdfp(args.pdfp, pdf_path, tmpdir / pdf_path.stem)
                        tables = extract_tables_from_markdown(md_path)
                        table_results.append({
                            "file": str(pdf_path.name),
                            "tables_found": len(tables),
                            "table_shapes": [
                                {"rows": len(t), "cols": max((len(r) for r in t), default=0)}
                                for t in tables
                            ],
                        })
                    except Exception as e:
                        table_results.append({
                            "file": str(pdf_path.name),
                            "error": str(e),
                        })

                results = {
                    "benchmark": "rd-tablebench",
                    "pdfp_binary": args.pdfp,
                    "files_processed": len(pdfs),
                    "results": table_results,
                }

    with open(args.output, "w") as f:
        json.dump(results, f, indent=2)

    print(f"Results written to {args.output}")


if __name__ == "__main__":
    main()
