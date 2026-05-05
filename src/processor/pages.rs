use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use mupdf::pdf::PdfDocument;

use crate::cli::{MergeArgs, PageSelectionArgs, PagesCommand, PagesSubcommand, SplitArgs};
use crate::processor::page_range::parse_page_selection;

pub fn run(args: &PagesCommand) -> anyhow::Result<()> {
    match &args.command {
        PagesSubcommand::Extract(args) => extract(args),
        PagesSubcommand::Delete(args) => delete(args),
        PagesSubcommand::Split(args) => split(args),
        PagesSubcommand::Reorder(args) => reorder(args),
        PagesSubcommand::Merge(args) => merge(args),
    }
}

fn extract(args: &PageSelectionArgs) -> anyhow::Result<()> {
    let page_count = page_count(&args.input)?;
    let keep_pages = parse_page_selection(&args.pages, page_count)?;
    write_selected_pages(&args.input, &args.output, &keep_pages)?;
    eprintln!("wrote {}", args.output.display());
    Ok(())
}

fn delete(args: &PageSelectionArgs) -> anyhow::Result<()> {
    let page_count = page_count(&args.input)?;
    let delete_pages: BTreeSet<usize> = parse_page_selection(&args.pages, page_count)?
        .into_iter()
        .collect();
    let keep_pages: Vec<usize> = (0..page_count)
        .filter(|page| !delete_pages.contains(page))
        .collect();
    if keep_pages.is_empty() {
        bail!("refusing to write an empty PDF after deleting every page");
    }
    write_selected_pages(&args.input, &args.output, &keep_pages)?;
    eprintln!("wrote {}", args.output.display());
    Ok(())
}

fn split(args: &SplitArgs) -> anyhow::Result<()> {
    if args.every == 0 {
        bail!("--every must be greater than 0");
    }

    let page_count = page_count(&args.input)?;
    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("failed to create {}", args.output.display()))?;

    let stem = args
        .input
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "chunk".to_string());

    for (chunk_idx, start) in (0..page_count).step_by(args.every).enumerate() {
        let end = (start + args.every).min(page_count);
        let keep_pages: Vec<usize> = (start..end).collect();
        let output = args
            .output
            .join(format!("{stem}-part{:03}.pdf", chunk_idx + 1));
        write_selected_pages(&args.input, &output, &keep_pages)?;
        eprintln!("wrote {}", output.display());
    }

    Ok(())
}

fn reorder(args: &PageSelectionArgs) -> anyhow::Result<()> {
    let page_count = page_count(&args.input)?;
    let pages = parse_page_selection(&args.pages, page_count)?;
    write_copied_pages(&[args.input.as_path()], &args.output, &[pages])?;
    eprintln!("wrote {}", args.output.display());
    Ok(())
}

fn merge(args: &MergeArgs) -> anyhow::Result<()> {
    if args.inputs.is_empty() {
        bail!("merge requires at least one input PDF");
    }

    let mut selections = Vec::with_capacity(args.inputs.len());
    for input in &args.inputs {
        let count = page_count(input)?;
        selections.push((0..count).collect::<Vec<_>>());
    }

    let inputs: Vec<&Path> = args.inputs.iter().map(PathBuf::as_path).collect();
    write_copied_pages(&inputs, &args.output, &selections)?;
    eprintln!("wrote {}", args.output.display());
    Ok(())
}

fn page_count(path: &Path) -> anyhow::Result<usize> {
    let path_str = path.to_string_lossy();
    let doc = PdfDocument::open(path_str.as_ref())
        .with_context(|| format!("failed to open {}", path.display()))?;
    Ok(doc
        .page_count()
        .with_context(|| format!("failed to count pages in {}", path.display()))? as usize)
}

fn write_selected_pages(input: &Path, output: &Path, keep_pages: &[usize]) -> anyhow::Result<()> {
    ensure_output_is_not_input(input, output)?;
    if keep_pages.is_empty() {
        bail!("page selection produced no output pages");
    }

    let page_count = page_count(input)?;
    let keep: BTreeSet<usize> = keep_pages.iter().copied().collect();
    let path_str = input.to_string_lossy();
    let mut pdf = PdfDocument::open(path_str.as_ref())
        .with_context(|| format!("failed to open {}", input.display()))?;

    for page in (0..page_count).rev() {
        if !keep.contains(&page) {
            pdf.delete_page(page as i32)
                .with_context(|| format!("failed to delete page {}", page + 1))?;
        }
    }

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let output_str = output.to_string_lossy();
    pdf.save(output_str.as_ref())
        .with_context(|| format!("failed to save {}", output.display()))?;
    Ok(())
}

fn ensure_output_is_not_input(input: &Path, output: &Path) -> anyhow::Result<()> {
    let input_abs = absolutize(input);
    let output_abs = absolutize(output);
    if input_abs == output_abs {
        bail!(
            "refusing to overwrite input PDF {}; choose a different -o path",
            input.display()
        );
    }
    Ok(())
}

fn write_copied_pages(
    inputs: &[&Path],
    output: &Path,
    selections: &[Vec<usize>],
) -> anyhow::Result<()> {
    if inputs.len() != selections.len() {
        bail!("internal error: input and selection counts differ");
    }
    for input in inputs {
        ensure_output_is_not_input(input, output)?;
    }

    let mut out = PdfDocument::new();
    let mut output_page_count = 0i32;
    for (input, pages) in inputs.iter().zip(selections) {
        let path_str = input.to_string_lossy();
        let source = PdfDocument::open(path_str.as_ref())
            .with_context(|| format!("failed to open {}", input.display()))?;
        let page_count = source
            .page_count()
            .with_context(|| format!("failed to count pages in {}", input.display()))?
            as usize;

        for page in pages {
            if *page >= page_count {
                bail!(
                    "page {} is out of range 1-{page_count} for {}",
                    page + 1,
                    input.display()
                );
            }
            let page_obj = source
                .find_page(*page as i32)
                .with_context(|| format!("failed to find page {}", page + 1))?;
            let copied = out
                .graft_object(&page_obj)
                .with_context(|| format!("failed to copy page {}", page + 1))?;
            out.insert_page(output_page_count, &copied)
                .with_context(|| format!("failed to append page {}", page + 1))?;
            output_page_count += 1;
        }
    }

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let output_str = output.to_string_lossy();
    out.save(output_str.as_ref())
        .with_context(|| format!("failed to save {}", output.display()))?;
    Ok(())
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_same_input_and_output_path() {
        let path = Path::new("same.pdf");
        assert!(ensure_output_is_not_input(path, path).is_err());
    }
}
