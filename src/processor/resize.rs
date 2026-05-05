use std::fs;

use anyhow::{bail, Context};
use mupdf::{Document, DocumentWriter, Matrix, Rect};

use crate::cli::{PageCommand, PageSubcommand, ResizeArgs};

pub fn run(args: &PageCommand) -> anyhow::Result<()> {
    match &args.command {
        PageSubcommand::Resize(args) => resize(args),
    }
}

fn resize(args: &ResizeArgs) -> anyhow::Result<()> {
    let (target_width, target_height) = paper_size(&args.paper)?;
    let fit = FitMode::parse(&args.fit)?;
    let media_box = Rect::new(0.0, 0.0, target_width, target_height);

    let input_str = args.input.to_string_lossy();
    let doc = Document::open(input_str.as_ref())
        .with_context(|| format!("failed to open {}", args.input.display()))?;
    if let Some(parent) = args
        .output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let output_str = args.output.to_string_lossy();
    let mut writer = DocumentWriter::new(output_str.as_ref(), "pdf", "")
        .with_context(|| format!("failed to create {}", args.output.display()))?;

    for page_idx in 0..doc.page_count()? {
        let page = doc.load_page(page_idx)?;
        let bounds = page.bounds()?;
        let (sx, sy) = match fit {
            FitMode::Stretch => (
                target_width / bounds.width(),
                target_height / bounds.height(),
            ),
            FitMode::Contain => {
                let scale = (target_width / bounds.width()).min(target_height / bounds.height());
                (scale, scale)
            }
            FitMode::Cover => {
                let scale = (target_width / bounds.width()).max(target_height / bounds.height());
                (scale, scale)
            }
        };
        let rendered_width = bounds.width() * sx;
        let rendered_height = bounds.height() * sy;
        let x = (target_width - rendered_width) / 2.0;
        let y = (target_height - rendered_height) / 2.0;
        let matrix = Matrix::new(sx, 0.0, 0.0, sy, x - bounds.x0 * sx, y - bounds.y0 * sy);

        let device = writer.begin_page(media_box)?;
        page.run(&device, &matrix)
            .with_context(|| format!("failed to render page {}", page_idx + 1))?;
        writer.end_page(device)?;
    }

    Ok(())
}

fn paper_size(paper: &str) -> anyhow::Result<(f32, f32)> {
    match paper.to_ascii_lowercase().as_str() {
        "a4" => Ok((595.0, 842.0)),
        "letter" => Ok((612.0, 792.0)),
        other => bail!("unsupported paper size `{other}`; expected `a4` or `letter`"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FitMode {
    Contain,
    Cover,
    Stretch,
}

impl FitMode {
    fn parse(input: &str) -> anyhow::Result<Self> {
        match input.to_ascii_lowercase().as_str() {
            "contain" => Ok(Self::Contain),
            "cover" => Ok(Self::Cover),
            "stretch" => Ok(Self::Stretch),
            other => bail!("unsupported fit mode `{other}`; expected contain, cover, or stretch"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fit_modes() {
        assert_eq!(FitMode::parse("contain").unwrap(), FitMode::Contain);
        assert_eq!(FitMode::parse("cover").unwrap(), FitMode::Cover);
        assert_eq!(FitMode::parse("stretch").unwrap(), FitMode::Stretch);
        assert!(FitMode::parse("bad").is_err());
    }

    #[test]
    fn knows_a4_dimensions() {
        assert_eq!(paper_size("a4").unwrap(), (595.0, 842.0));
    }
}
