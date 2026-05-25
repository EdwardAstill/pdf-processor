# Stage 0 — Performance Baseline

**Date:** 2026-05-21
**Binary:** pdfp 0.3.0 (GitHub release)
**Binary size:** 48,220,864 bytes (~46 MB)

## Test PDF

**File:** `/home/eastill/projects/warden/Edward_Astill-369380.pdf`
**Description:** 11-page Australian Clinical Labs medical report
**Pages with readable text:** 11
**Image-only pages:** 0
**Scan-like:** false

## Conversion benchmark

| Metric | Value |
|---|---|
| Time | 188ms |
| Output markdown lines | 471 |
| Output markdown size | 38,144 bytes |
| Images extracted | 11 PNGs |

## Notes

- pdfium-metadata warnings appear (libpdfium.so not installed) — graceful degradation, non-fatal
- 14 formula candidates detected across 2 pages — formula detection is active
- No errors, clean output
