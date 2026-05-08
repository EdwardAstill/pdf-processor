# Contradictions - Formula Extraction Tools

## No True Contradictions

The sources broadly agree that formulas require dedicated detection and recognition, not generic OCR.

## Different Use Cases

- **Whole-document converters**: Docling, MinerU, Marker, PaddleOCR-VL/PP-Structure can produce document-level Markdown/JSON. These are easier to run as a fallback backend but can replace more of `pdfp`'s local pipeline than desired.
- **Component recognizers**: PDF-Extract-Kit and UniMERNet are better for `pdfp` if we want to keep the Rust layout pipeline and only patch formula gaps by sending cropped equation regions to a sidecar.
- **Hosted high-accuracy APIs**: Mathpix and Azure reduce local setup but conflict with a local-first/offline default and raise privacy/cost issues for standards.

## Licensing/Packaging Caveats

- Docling and PaddleOCR are permissive enough to call as optional sidecars/services.
- UniMERNet code is Apache-2.0, but model download size is material.
- Surya/Marker are technically attractive, but Surya's GPL code and model terms make bundling inside a permissive one-command `pdfp` release less straightforward.
- Mathpix/Azure are API integrations, not bundled local tools.
