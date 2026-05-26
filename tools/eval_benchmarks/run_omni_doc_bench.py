#!/usr/bin/env python3
"""Run OmniDocBench evaluation against pdfp output.

Usage:
    python tools/eval_benchmarks/run_omni_doc_bench.py \
        --pdfp path/to/pdfp \
        --omnidocbench path/to/OmniDocBench/dataset \
        --output results.json

OmniDocBench dataset: https://huggingface.co/datasets/opendatalab/OmniDocBench
"""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


BLOCK_CATEGORIES = {
    "heading": {"H1", "H2", "H3", "H4", "H5", "Title"},
    "paragraph": {"Paragraph", "P"},
    "table": {"Table", "TableCell"},
    "figure": {"Figure", "Image"},
    "formula": {"Formula"},
    "list": {"ListItem", "List"},
    "caption": {"Caption"},
    "footer": {"Footer", "PageNumber"},
}


def run_pdfp(pdfp_bin: str, pdf_path: Path, output_dir: Path) -> Path:
    """Run pdfp convert and return path to the output markdown file."""
    output_dir.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [pdfp_bin, "convert", str(pdf_path), "-o", str(output_dir)],
        check=True,
        capture_output=True,
        text=True,
    )
    stem = pdf_path.stem
    md_path = output_dir / stem / f"{stem}.md"
    if not md_path.exists():
        candidates = list(output_dir.glob("**/*.md"))
        if candidates:
            return candidates[0]
        raise FileNotFoundError(f"No markdown output found in {output_dir}")
    return md_path


def count_blocks_in_markdown(md_path: Path) -> dict[str, int]:
    """Count structured elements in pdfp markdown output.

    Detects: headings (#), tables (|...|), formulas ($$...$$), images (![...]),
    lists (- / 1.), code blocks (```), paragraphs (text between blank lines).
    """
    counts = {
        "heading": 0,
        "table": 0,
        "formula": 0,
        "figure": 0,
        "list": 0,
        "paragraph": 0,
        "code": 0,
    }

    with open(md_path) as f:
        content = f.read()

    lines = content.split("\n")
    in_table = False
    in_code = False
    in_formula = False
    in_list = False
    paragraph_lines = 0

    for line in lines:
        stripped = line.strip()

        # Headings
        if stripped.startswith("#") and not stripped.startswith("<!--"):
            counts["heading"] += 1
            continue

        # Formulas
        if stripped.startswith("$$"):
            if in_formula:
                counts["formula"] += 1
                in_formula = False
            else:
                in_formula = True
            continue
        if stripped.startswith("$") and not stripped.startswith("$$"):
            counts["formula"] += 1
            continue

        # Code blocks
        if stripped.startswith("```"):
            if in_code:
                in_code = False
            else:
                counts["code"] += 1
                in_code = True
            continue

        # Tables
        if stripped.startswith("|") and stripped.endswith("|"):
            if not in_table:
                counts["table"] += 1
                in_table = True
            continue
        else:
            in_table = False

        # Images
        if stripped.startswith("![") and "](" in stripped:
            counts["figure"] += 1
            continue

        # Lists
        if stripped.startswith(("- ", "* ", "+ ")) or (
            len(stripped) > 2 and stripped[0].isdigit() and stripped[1:].startswith(". ")
        ):
            if not in_list:
                counts["list"] += 1
                in_list = True
            continue
        else:
            in_list = False

        # Paragraphs (non-empty, non-comment, non-special lines)
        if stripped and not stripped.startswith("<!--"):
            paragraph_lines += 1
        elif paragraph_lines > 0:
            counts["paragraph"] += 1
            paragraph_lines = 0

    if paragraph_lines > 0:
        counts["paragraph"] += 1

    return counts


def compare_block_counts(pdfp_counts: dict[str, int], gt_counts: dict[str, int]) -> dict:
    """Compare pdfp block counts against ground truth."""
    comparison = {}
    all_categories = set(pdfp_counts.keys()) | set(gt_counts.keys())
    for cat in sorted(all_categories):
        pred = pdfp_counts.get(cat, 0)
        gt = gt_counts.get(cat, 0)
        recall = min(pred, gt) / max(gt, 1)
        precision = min(pred, gt) / max(pred, 1)
        comparison[cat] = {
            "pdfp_count": pred,
            "gt_count": gt,
            "recall": round(recall, 4),
            "precision": round(precision, 4),
        }
    return comparison


def load_omnidocbench_ground_truth(dataset_path: Path) -> dict:
    """Load ground truth block counts from OmniDocBench annotations.

    OmniDocBench provides JSON annotations per page with block-level categories.
    """
    gt_counts = {}
    json_files = list(dataset_path.glob("**/*.json"))
    if not json_files:
        print(f"No annotation JSON files found in {dataset_path}", file=sys.stderr)
        return gt_counts

    for json_path in json_files:
        try:
            with open(json_path) as f:
                data = json.load(f)
            # OmniDocBench format: each file has page-level annotations
            pdf_name = data.get("file_name", json_path.stem)
            blocks = data.get("blocks", data.get("annotations", []))
            counts = {}
            for block in blocks:
                category = block.get("category", block.get("type", "unknown"))
                counts[category] = counts.get(category, 0) + 1
            gt_counts[pdf_name] = counts
        except (json.JSONDecodeError, KeyError):
            continue

    return gt_counts


def main():
    parser = argparse.ArgumentParser(description="Run OmniDocBench evaluation on pdfp")
    parser.add_argument("--pdfp", default="pdfp", help="Path to pdfp binary")
    parser.add_argument("--omnidocbench", required=True, help="Path to OmniDocBench dataset")
    parser.add_argument("--output", default="omnidocbench_results.json", help="Output JSON path")
    parser.add_argument("--limit", type=int, default=0, help="Limit number of PDFs (0 = all)")
    args = parser.parse_args()

    dataset = Path(args.omnidocbench)
    pdfs = list(dataset.glob("**/*.pdf"))
    if not pdfs:
        print(f"No PDFs found in {dataset}", file=sys.stderr)
        sys.exit(1)

    if args.limit > 0:
        pdfs = pdfs[: args.limit]

    gt_counts_all = load_omnidocbench_ground_truth(dataset)

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        per_file_results = []

        for i, pdf_path in enumerate(pdfs):
            print(f"[{i+1}/{len(pdfs)}] {pdf_path.name}")
            try:
                md_path = run_pdfp(args.pdfp, pdf_path, tmpdir / pdf_path.stem)
                block_counts = count_blocks_in_markdown(md_path)

                pdf_key = pdf_path.name
                gt = gt_counts_all.get(pdf_key, {})
                comparison = compare_block_counts(block_counts, gt) if gt else {}

                per_file_results.append({
                    "file": str(pdf_path.name),
                    "block_counts": block_counts,
                    "ground_truth": gt,
                    "comparison": comparison,
                })
            except Exception as e:
                per_file_results.append({
                    "file": str(pdf_path.name),
                    "error": str(e),
                })

    # Aggregate
    total_counts = {}
    for r in per_file_results:
        if "block_counts" in r:
            for cat, count in r["block_counts"].items():
                total_counts[cat] = total_counts.get(cat, 0) + count

    results = {
        "benchmark": "omnidocbench",
        "pdfp_binary": args.pdfp,
        "files_processed": len(pdfs),
        "aggregate_counts": total_counts,
        "per_file": per_file_results,
    }

    with open(args.output, "w") as f:
        json.dump(results, f, indent=2)

    print(f"\nAggregate block counts:")
    for cat, count in sorted(total_counts.items()):
        print(f"  {cat}: {count}")
    print(f"\nResults written to {args.output}")


if __name__ == "__main__":
    main()
