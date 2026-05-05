use std::collections::BTreeSet;

use anyhow::{bail, Context};

pub fn parse_page_selection(spec: &str, page_count: usize) -> anyhow::Result<Vec<usize>> {
    if page_count == 0 {
        bail!("cannot select pages from an empty PDF");
    }

    let spec = spec.trim();
    if spec.is_empty() {
        bail!("page selection cannot be empty");
    }

    let mut pages = Vec::new();
    let mut seen = BTreeSet::new();
    for token in spec
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        match token {
            "all" => push_range(&mut pages, &mut seen, 1, page_count, page_count)?,
            "odd" => {
                for page in (1..=page_count).step_by(2) {
                    push_page(&mut pages, &mut seen, page, page_count)?;
                }
            }
            "even" => {
                for page in (2..=page_count).step_by(2) {
                    push_page(&mut pages, &mut seen, page, page_count)?;
                }
            }
            _ if token.contains('-') => {
                let (start, end) = token
                    .split_once('-')
                    .ok_or_else(|| anyhow::anyhow!("invalid page range `{token}`"))?;
                let start = parse_one_indexed(start, page_count)
                    .with_context(|| format!("invalid range start in `{token}`"))?;
                let end = parse_one_indexed(end, page_count)
                    .with_context(|| format!("invalid range end in `{token}`"))?;
                push_range(&mut pages, &mut seen, start, end, page_count)?;
            }
            _ => {
                let page = parse_one_indexed(token, page_count)
                    .with_context(|| format!("invalid page `{token}`"))?;
                push_page(&mut pages, &mut seen, page, page_count)?;
            }
        }
    }

    if pages.is_empty() {
        bail!("page selection `{spec}` did not select any pages");
    }
    Ok(pages.into_iter().map(|page| page - 1).collect())
}

fn parse_one_indexed(input: &str, page_count: usize) -> anyhow::Result<usize> {
    let page: usize = input
        .trim()
        .parse()
        .with_context(|| format!("`{}` is not a positive page number", input.trim()))?;
    if page == 0 || page > page_count {
        bail!("page {page} is out of range 1-{page_count}");
    }
    Ok(page)
}

fn push_range(
    pages: &mut Vec<usize>,
    seen: &mut BTreeSet<usize>,
    start: usize,
    end: usize,
    page_count: usize,
) -> anyhow::Result<()> {
    if start > end {
        bail!("page range {start}-{end} is descending; use reorder for custom order");
    }
    for page in start..=end {
        push_page(pages, seen, page, page_count)?;
    }
    Ok(())
}

fn push_page(
    pages: &mut Vec<usize>,
    seen: &mut BTreeSet<usize>,
    page: usize,
    page_count: usize,
) -> anyhow::Result<()> {
    if page == 0 || page > page_count {
        bail!("page {page} is out of range 1-{page_count}");
    }
    if seen.insert(page) {
        pages.push(page);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_pages_and_ranges() {
        assert_eq!(
            parse_page_selection("1,3,5-7", 10).unwrap(),
            vec![0, 2, 4, 5, 6]
        );
    }

    #[test]
    fn parses_all_odd_and_even() {
        assert_eq!(parse_page_selection("odd", 5).unwrap(), vec![0, 2, 4]);
        assert_eq!(parse_page_selection("even", 5).unwrap(), vec![1, 3]);
        assert_eq!(parse_page_selection("all", 3).unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn rejects_out_of_range_pages() {
        assert!(parse_page_selection("0", 3).is_err());
        assert!(parse_page_selection("4", 3).is_err());
    }

    #[test]
    fn rejects_descending_ranges() {
        assert!(parse_page_selection("3-1", 3).is_err());
    }
}
