# Contradictions And Resolutions

date: 2026-05-05

## Embedded Images vs Figure Snapshots

There is no real contradiction, but there is an important distinction:

- Embedded image extraction is useful when users want the raw raster objects stored inside the PDF.
- Figure snapshot extraction is useful when users want the visual figure as it appears on the page.

The research paper and existing local code both show why these differ. A visible figure may include raster images, vector drawing commands, text labels, legends, masks, and panel letters. Raw extraction can preserve original embedded pixels, but it cannot guarantee the complete visual figure.

Resolution: keep embedded extraction and add snapshot extraction as a separate figure mode.

## Pure Rust Renderer vs External Sidecar

Official MuPDF/PyMuPDF docs support clipped pixmap rendering. The local Rust `mupdf` crate exposes page rendering, display lists, devices, pixmaps, and clip-aware devices, but its high-level `Page::to_pixmap` wrapper does not appear to expose a direct `clip` parameter.

Resolution: first prototype a pure-Rust region renderer using display lists and clip-aware draw devices. Keep Poppler or `mutool draw` as a contingency only if the Rust binding blocks clean clipped rendering.

## Default Behavior

Changing the default image behavior could surprise current users and alter output size/performance.

Resolution: make figure snapshots opt-in at first via `--figures snapshot` or `--figures both`. Leave current behavior equivalent to embedded extraction until the test corpus shows stable quality and acceptable performance.
