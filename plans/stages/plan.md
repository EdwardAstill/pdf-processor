# pdfp Improvement Plan

**Date:** 2026-05-21
**From:** `.pied/research/pdf-processor-landscape/FINDINGS.md`

## Goal

Systematically improve pdfp across four stages: consolidate → quick wins → medium features → advanced. Performance is benchmarked at each stage boundary to prevent regressions.

## Stage summary

| Stage | Focus | Timeline | New features? |
|---|---|---|---|
| 0 | Performance baseline | Now | No — measure only |
| 1 | Consolidation & refactoring | ~1 week | No — code quality only |
| 2 | Quick wins (11 features from research) | ~1 month | Yes — all ~30-100 LOC each |
| 3 | Medium features (forms, PDF/A, CJK, links) | ~3 months | Yes |
| 4 | Advanced (round-trip, signatures, repair) | ~6+ months | Yes |

## Performance gate

Each stage must pass:
1. `cargo clippy -- -D warnings` — clean
2. `cargo test` — all passing
3. Conversion benchmark on a standard test PDF — time and output must not regress
4. `cargo build --release` — succeeds
5. Eval fixtures — scores must not regress

---

## Stage 0: Performance Baseline

**Status:** ✅ Complete

Baseline recorded at `plans/stages/baseline.md`. Test PDF: 11-page medical report, 168ms conversion, 471 lines output, 11 images.

## Stage 1: Consolidation & Refactoring

**Status:** ✅ Complete

No new features. Code quality and documentation only.

### Tasks

- [x] 1.1 Rename `VtvError` → `PdfpError`, `VtvResult` → `PdfpResult` (52 references across 7 files)
- [x] 1.2 Audit dead code (17 sites — mostly intentional: feature-gated, reserved, test-only; added doc comments)
- [x] 1.3 Write `ARCHITECTURE.md` documenting pdfp's moat, module map, and design decisions
- [x] 1.4 Update `README.md` with competitive positioning from research
- [x] 1.5 `pdfp --version` already handled by clap
- [x] 1.6 Dependencies reviewed — mupdf 0.6, lopdf 0.40 are current
- [x] 1.7 `cli.rs` structure is fine — each command delegates to `processor/`
- [x] 1.8 Error messages are clear and actionable
- [x] 1.9 Add performance benchmark script at `tools/bench.sh`
- [x] 1.10 Run performance gate (clippy, test, benchmark, eval) — all passed

### Acceptance
- [x] Clippy clean
- [x] All tests pass (6 unit + 5 integration)
- [x] `VtvError`/`VtvResult` fully renamed
- [x] `ARCHITECTURE.md` exists
- [x] README updated with moat
- [x] Performance metrics recorded, no regression (168ms on 11-page PDF)

## Stage 2: Quick Wins

**Status:** ⬜ Not started

All features use existing mupdf `pdf` module APIs — zero new dependencies.

### Tasks

- [ ] 2.1 Outlines/Bookmarks → Markdown TOC (~50 LOC)
- [ ] 2.2 Password unlock (~30 LOC) — `--password` flag
- [ ] 2.3 Rotate pages (~40 LOC) — `pdfp pages rotate`
- [ ] 2.4 Annotations extract (~110 LOC) — `pdfp inspect --annotations`
- [ ] 2.5 Compress/optimize PDF (~50 LOC) — `pdfp optimize` command
- [ ] 2.6 Crop box operations (~40 LOC) — `pdfp page crop`
- [ ] 2.7 Watermarks (~80 LOC) — `pdfp watermark` command
- [ ] 2.8 Page links in Markdown (~40 LOC)
- [ ] 2.9 Save with encryption (~60 LOC) — `pdfp encrypt` / `--encrypt` flag on save commands
- [ ] 2.10 Internal link → heading reference conversion (~60 LOC)
- [ ] 2.11 Media box reading (~20 LOC)
- [ ] 2.12 Performance gate after EACH feature

## Stage 3: Medium Features

**Status:** ⬜ Not started

### Tasks

- [ ] 3.1 Form filling — read/write AcroForm widget values via lopdf
- [ ] 3.2 PDF/A validation — check compliance, report violations
- [ ] 3.3 CJK/RTL text ordering improvements
- [ ] 3.4 Form flattening
- [ ] 3.5 N-up imposition (4-up, 6-up, etc.)
- [ ] 3.6 Overlay/underlay pages
- [ ] 3.7 Performance gate

## Stage 4: Advanced

**Status:** ⬜ Not started

### Tasks

- [ ] 4.1 Markdown→PDF round-trip via Typst compiler integration
- [ ] 4.2 Digital signatures (PKCS#7/PAdES) — requires new crate or FFI
- [ ] 4.3 PDF repair/recovery
- [ ] 4.4 OmniDocBench alignment for eval framework
- [ ] 4.5 PDF/UA accessibility generation
- [ ] 4.6 Performance gate
