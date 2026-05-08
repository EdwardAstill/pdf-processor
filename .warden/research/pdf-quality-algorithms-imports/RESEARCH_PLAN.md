# Research Plan: PDF Quality Algorithms And Import Candidates

**Started:** 2026-05-08
**Depth:** deep
**Deliverable:** recommendation report

## Question

Which external algorithms, tools, GitHub projects, and research-backed processes should `pdfp` adapt, call as optional sidecars, or explicitly avoid to improve table extraction, formulas, images/figures, scans/OCR, and quality evaluation?

## Checklist

- [x] Identify table extraction algorithms and repos that map to `pdfp`'s coordinate-based Rust path.
- [x] Identify formula recognition/enrichment tools suitable for optional page/crop sidecars.
- [x] Identify scan/OCR and full-document fallback engines worth testing against the current examples.
- [x] Identify figure/image extraction techniques that improve over embedded raster extraction.
- [x] Identify datasets, metrics, and repeatable processes for deciding whether a change really improved quality.
- [x] Separate candidates into native-port, optional sidecar, benchmark-only, and avoid-for-now categories.

## Source Families

| Family | Tool | Target count | Reason |
| --- | --- | ---: | --- |
| GitHub repos | `github-repo-researcher`, web, GitHub pages | 8-12 | Find importable implementations and maintenance risk. |
| Official docs | web | 8-12 | Confirm current capabilities and integration shape. |
| Papers | web | 4-8 | Understand algorithms, datasets, and evaluation metrics. |
| Existing local research | local files | 3-5 | Avoid repeating prior repo-specific findings. |

## Stop Conditions

- Every checklist item has at least one primary source.
- Each recommendation is tied to a concrete `pdfp` integration path.
- Known contradictions and use-case differences are recorded.
- No candidate is recommended only because it exists; each has a fit/risk call.

## Acceptance Notes

- Primary sources were checked from GitHub repos, official docs, and papers.
- Existing local research was used to avoid repeating prior repo-specific findings.
- Licensing was treated as a first-class filter because this repo is MIT licensed.
