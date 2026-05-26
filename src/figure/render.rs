use std::fs;
use std::path::Path;

use anyhow::Context;
use mupdf::{Colorspace, Device, Document as MuDocument, IRect, ImageFormat, Matrix, Pixmap};

use crate::document::types::{Bbox, Block, BlockKind};
use crate::figure::FigureCandidate;

#[derive(Clone, Debug)]
pub struct RenderedFigure {
    pub block: Block,
}

pub fn render_figure_snapshots(
    pdf_path: &Path,
    page_num: usize,
    candidates: &[FigureCandidate],
    images_dir: &Path,
    dpi: u32,
) -> anyhow::Result<Vec<RenderedFigure>> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    fs::create_dir_all(images_dir)
        .with_context(|| format!("Failed to create images dir {}", images_dir.display()))?;

    let document = MuDocument::open(pdf_path)
        .with_context(|| format!("Failed to open {} for figure rendering", pdf_path.display()))?;
    let page = document
        .load_page(page_num as i32)
        .with_context(|| format!("Failed to load page {} for figure rendering", page_num + 1))?;

    let mut rendered = Vec::new();
    for candidate in candidates {
        let filename = format!("page{}_fig{}.png", page_num + 1, rendered.len() + 1);
        let abs_path = images_dir.join(&filename);
        let Some(bytes) = render_bbox_png(&page, candidate.bbox, dpi)
            .with_context(|| format!("Failed to render figure snapshot {filename}"))?
        else {
            continue;
        };
        fs::write(&abs_path, bytes)
            .with_context(|| format!("Failed to write figure snapshot {}", abs_path.display()))?;

        rendered.push(RenderedFigure {
            block: Block {
                override_markdown: None,
                id: 2_000_000 + rendered.len(),
                bbox: candidate.bbox,
                text: String::new(),
                kind: BlockKind::Figure {
                    path: Some(format!("images/{filename}")),
                    caption: candidate.caption_text.clone(),
                },
                font_size: 0.0,
                font_name: "figure-snapshot".to_string(),
                page_num,
                reading_order: 0,
                bold: false,
                italic: false,
            },
        });
    }

    Ok(rendered)
}

pub(crate) fn render_bbox_png(
    page: &mupdf::Page,
    bbox: Bbox,
    dpi: u32,
) -> anyhow::Result<Option<Vec<u8>>> {
    let scale = (dpi.max(1) as f32) / 72.0;
    let width = ((bbox.width() * scale).ceil() as i32).max(1);
    let height = ((bbox.height() * scale).ceil() as i32).max(1);
    let mut pixmap = Pixmap::new(&Colorspace::device_rgb(), 0, 0, width, height, false)
        .context("failed to allocate figure pixmap")?;
    pixmap
        .clear_with(255)
        .context("failed to clear figure pixmap")?;

    {
        let device = Device::from_pixmap_with_clip(&pixmap, IRect::new(0, 0, width, height))
            .context("failed to create clipped draw device")?;
        let mut ctm = Matrix::new_scale(scale, scale);
        ctm.pre_translate(-bbox.x0, -bbox.y0);
        page.run(&device, &ctm)
            .context("failed to draw page into figure pixmap")?;
    }

    if pixmap_is_blank(&pixmap) {
        return Ok(None);
    }

    let mut bytes = Vec::new();
    pixmap
        .write_to(&mut bytes, ImageFormat::PNG)
        .context("failed to encode figure PNG")?;
    Ok(Some(bytes))
}

fn pixmap_is_blank(pixmap: &Pixmap) -> bool {
    pixmap.samples().iter().all(|sample| *sample >= 248)
}
