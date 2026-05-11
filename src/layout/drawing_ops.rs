//! Extract horizontal and vertical line geometry from rendered PDF pages.
//!
//! The `mupdf` crate exposes rendering and display lists, but not a stable,
//! high-level path iterator. This module uses a deterministic raster fallback:
//! render the page in grayscale, scan for long dark runs, then map detected
//! runs back into PDF point coordinates.

use anyhow::Context;
use mupdf::{Colorspace, Matrix, Page};

const DETECT_DPI: f32 = 150.0;
const SCALE: f32 = DETECT_DPI / 72.0;
const DARK_THRESHOLD: u8 = 80;
const MIN_H_RUN_PX: usize = 50;
const MIN_V_RUN_PX: usize = 42;
const MERGE_TOLERANCE_PT: f32 = 1.5;

/// A horizontal line segment detected on the page, in PDF point coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct HLine {
    pub x0: f32,
    pub x1: f32,
    pub y: f32,
    pub thickness: f32,
}

impl HLine {
    pub fn length(&self) -> f32 {
        (self.x1 - self.x0).abs()
    }

    pub fn is_significant(&self) -> bool {
        self.length() >= 100.0
    }
}

/// A vertical line segment detected on the page, in PDF point coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct VLine {
    pub x: f32,
    pub y0: f32,
    pub y1: f32,
    pub thickness: f32,
}

impl VLine {
    pub fn length(&self) -> f32 {
        (self.y1 - self.y0).abs()
    }

    pub fn is_significant(&self) -> bool {
        self.length() >= 20.0
    }
}

/// Extract horizontal and vertical line segments from a rendered page.
pub fn extract_lines(
    page: &Page,
    page_width: f32,
    page_height: f32,
) -> anyhow::Result<(Vec<HLine>, Vec<VLine>)> {
    let pixmap = page
        .to_pixmap(
            &Matrix::new_scale(SCALE, SCALE),
            &Colorspace::device_gray(),
            false,
            false,
        )
        .context("rendering page for line extraction failed")?;

    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    let stride = pixmap.stride().max(0) as usize;
    let components = pixmap.n().max(1) as usize;
    let samples = pixmap.samples();

    if width == 0 || height == 0 || samples.is_empty() || stride == 0 {
        return Ok((Vec::new(), Vec::new()));
    }

    let mut hlines = Vec::new();
    for row in 0..height {
        let mut run_start = None;
        for col in 0..width {
            if dark_pixel(samples, stride, components, row, col) {
                run_start.get_or_insert(col);
            } else if let Some(start) = run_start.take() {
                push_hline_run(&mut hlines, start, col, row, page_width, page_height);
            }
        }
        if let Some(start) = run_start {
            push_hline_run(&mut hlines, start, width, row, page_width, page_height);
        }
    }

    let mut vlines = Vec::new();
    for col in 0..width {
        let mut run_start = None;
        for row in 0..height {
            if dark_pixel(samples, stride, components, row, col) {
                run_start.get_or_insert(row);
            } else if let Some(start) = run_start.take() {
                push_vline_run(&mut vlines, start, row, col, page_width, page_height);
            }
        }
        if let Some(start) = run_start {
            push_vline_run(&mut vlines, start, height, col, page_width, page_height);
        }
    }

    Ok((merge_hlines(hlines), merge_vlines(vlines)))
}

fn dark_pixel(samples: &[u8], stride: usize, components: usize, row: usize, col: usize) -> bool {
    let idx = row
        .saturating_mul(stride)
        .saturating_add(col.saturating_mul(components));
    samples
        .get(idx)
        .is_some_and(|sample| *sample <= DARK_THRESHOLD)
}

fn push_hline_run(
    hlines: &mut Vec<HLine>,
    start: usize,
    end: usize,
    row: usize,
    page_width: f32,
    page_height: f32,
) {
    if end.saturating_sub(start) < MIN_H_RUN_PX {
        return;
    }
    let x0 = (start as f32 / SCALE).clamp(0.0, page_width);
    let x1 = (end as f32 / SCALE).clamp(0.0, page_width);
    let y = (row as f32 / SCALE).clamp(0.0, page_height);
    if x1 > x0 {
        hlines.push(HLine {
            x0,
            x1,
            y,
            thickness: 1.0 / SCALE,
        });
    }
}

fn push_vline_run(
    vlines: &mut Vec<VLine>,
    start: usize,
    end: usize,
    col: usize,
    page_width: f32,
    page_height: f32,
) {
    if end.saturating_sub(start) < MIN_V_RUN_PX {
        return;
    }
    let x = (col as f32 / SCALE).clamp(0.0, page_width);
    let y0 = (start as f32 / SCALE).clamp(0.0, page_height);
    let y1 = (end as f32 / SCALE).clamp(0.0, page_height);
    if y1 > y0 {
        vlines.push(VLine {
            x,
            y0,
            y1,
            thickness: 1.0 / SCALE,
        });
    }
}

fn merge_hlines(mut lines: Vec<HLine>) -> Vec<HLine> {
    lines.sort_by(|left, right| {
        left.y
            .partial_cmp(&right.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.x0
                    .partial_cmp(&right.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut merged: Vec<HLine> = Vec::new();
    for line in lines {
        if let Some(last) = merged.last_mut() {
            let same_band = (last.y - line.y).abs() <= MERGE_TOLERANCE_PT;
            let touches = line.x0 <= last.x1 + 2.0;
            if same_band && touches {
                let total_thickness = last.thickness + line.thickness;
                last.y = (last.y * last.thickness + line.y * line.thickness) / total_thickness;
                last.x0 = last.x0.min(line.x0);
                last.x1 = last.x1.max(line.x1);
                last.thickness = total_thickness;
                continue;
            }
        }
        merged.push(line);
    }
    merged
}

fn merge_vlines(mut lines: Vec<VLine>) -> Vec<VLine> {
    lines.sort_by(|left, right| {
        left.x
            .partial_cmp(&right.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.y0
                    .partial_cmp(&right.y0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut merged: Vec<VLine> = Vec::new();
    for line in lines {
        if let Some(last) = merged.last_mut() {
            let same_band = (last.x - line.x).abs() <= MERGE_TOLERANCE_PT;
            let touches = line.y0 <= last.y1 + 2.0;
            if same_band && touches {
                let total_thickness = last.thickness + line.thickness;
                last.x = (last.x * last.thickness + line.x * line.thickness) / total_thickness;
                last.y0 = last.y0.min(line.y0);
                last.y1 = last.y1.max(line.y1);
                last.thickness = total_thickness;
                continue;
            }
        }
        merged.push(line);
    }
    merged
}
