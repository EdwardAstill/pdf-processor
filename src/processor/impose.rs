use std::{fs, path::Path};

use anyhow::Context;
use mupdf::{Document, DocumentWriter, Matrix, Rect};

use crate::cli::{ImposeCommand, ImposeSubcommand};

pub fn run(args: &ImposeCommand) -> anyhow::Result<()> {
    match &args.command {
        ImposeSubcommand::TwoUp(args) => two_up(&args.input, &args.output),
        ImposeSubcommand::Booklet(args) => booklet(&args.input, &args.output),
    }
}

fn two_up(input: &Path, output: &Path) -> anyhow::Result<()> {
    let doc = open_doc(input)?;
    let page_count = doc.page_count()? as usize;
    let page_order: Vec<Option<usize>> = (0..page_count).map(Some).collect();
    write_two_up(&doc, &page_order, output)
}

fn booklet(input: &Path, output: &Path) -> anyhow::Result<()> {
    let doc = open_doc(input)?;
    let page_count = doc.page_count()? as usize;
    let padded = page_count.next_multiple_of(4);
    let mut order = Vec::with_capacity(padded);

    for sheet_start in (0..padded).step_by(4) {
        let left_front = padded - sheet_start - 1;
        let right_front = sheet_start;
        let left_back = sheet_start + 1;
        let right_back = padded - sheet_start - 2;
        order.push(page_if_real(left_front, page_count));
        order.push(page_if_real(right_front, page_count));
        order.push(page_if_real(left_back, page_count));
        order.push(page_if_real(right_back, page_count));
    }

    write_two_up(&doc, &order, output)
}

fn page_if_real(page: usize, page_count: usize) -> Option<usize> {
    (page < page_count).then_some(page)
}

fn write_two_up(doc: &Document, page_order: &[Option<usize>], output: &Path) -> anyhow::Result<()> {
    let first_page_idx = page_order
        .iter()
        .flatten()
        .next()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("cannot impose an empty PDF"))?;
    let first_bounds = doc.load_page(first_page_idx as i32)?.bounds()?;
    let cell_width = first_bounds.width();
    let cell_height = first_bounds.height();
    let media_box = Rect::new(0.0, 0.0, cell_width * 2.0, cell_height);

    ensure_parent_dir(output)?;
    let output_str = output.to_string_lossy();
    let mut writer = DocumentWriter::new(output_str.as_ref(), "pdf", "")
        .with_context(|| format!("failed to create {}", output.display()))?;

    for spread in page_order.chunks(2) {
        let device = writer.begin_page(media_box)?;
        for (slot, page_idx) in spread.iter().enumerate() {
            if let Some(page_idx) = page_idx {
                let page = doc.load_page(*page_idx as i32)?;
                let bounds = page.bounds()?;
                let scale = (cell_width / bounds.width()).min(cell_height / bounds.height());
                let x = slot as f32 * cell_width + (cell_width - bounds.width() * scale) / 2.0;
                let y = (cell_height - bounds.height() * scale) / 2.0;
                let matrix = Matrix::new(
                    scale,
                    0.0,
                    0.0,
                    scale,
                    x - bounds.x0 * scale,
                    y - bounds.y0 * scale,
                );
                page.run(&device, &matrix)
                    .with_context(|| format!("failed to render page {}", page_idx + 1))?;
            }
        }
        writer.end_page(device)?;
    }

    Ok(())
}

fn open_doc(input: &Path) -> anyhow::Result<Document> {
    let input_str = input.to_string_lossy();
    Document::open(input_str.as_ref())
        .with_context(|| format!("failed to open {}", input.display()))
}

fn ensure_parent_dir(output: &Path) -> anyhow::Result<()> {
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn booklet_order_pads_to_multiple_of_four() {
        let page_count: usize = 5;
        let padded = page_count.next_multiple_of(4);
        let mut order = Vec::new();
        for sheet_start in (0..padded).step_by(4) {
            order.push(page_if_real(padded - sheet_start - 1, page_count));
            order.push(page_if_real(sheet_start, page_count));
            order.push(page_if_real(sheet_start + 1, page_count));
            order.push(page_if_real(padded - sheet_start - 2, page_count));
        }
        assert_eq!(order.len(), 8);
        assert_eq!(order[0], None);
        assert_eq!(order[1], Some(0));
    }
}
