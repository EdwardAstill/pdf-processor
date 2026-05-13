//! DNV-ST-N001 formula detection regression tests.
//! These tests require the DNV PDF at DNV_PDF and are skipped in CI.
//! Run locally with: cargo test --test dnv_formula_regression -- --ignored

const DNV_PDF: &str = "/home/eastill/projects/literature/standards/pdfs/\
    marine-operations-lifting-transport/\
    DNV-ST-N001_2018 - Marine operations and marine warranty.pdf";

#[cfg(test)]
mod dnv_formula_regression {
    use super::*;
    use pdf_processor::formula::detect::detect_formula_candidates;
    use pdf_processor::pdf::extractor::PdfExtractor;

    fn dnv_raw_page(page_num: usize) -> pdf_processor::document::types::RawPage {
        let pages = PdfExtractor::extract_pages(std::path::Path::new(DNV_PDF))
            .expect("failed to extract DNV PDF pages");
        pages
            .into_iter()
            .find(|p| p.page_num == page_num)
            .unwrap_or_else(|| panic!("page {page_num} not found in DNV PDF"))
    }

    #[test]
    #[ignore = "requires DNV PDF"]
    fn page_597_has_zero_formula_candidates() {
        let page = dnv_raw_page(596); // 0-indexed
        let candidates = detect_formula_candidates(&page, &[]);
        assert!(
            candidates.is_empty(),
            "page 597 is a references section — expected 0 candidates, got {}: {:#?}",
            candidates.len(),
            candidates
                .iter()
                .map(|c| &c.source_text)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    #[ignore = "requires DNV PDF"]
    fn page_130_has_formula_candidates() {
        let page = dnv_raw_page(129); // 0-indexed
        let candidates = detect_formula_candidates(&page, &[]);
        assert!(
            !candidates.is_empty(),
            "page 130 contains weight formulas — expected ≥1 candidate"
        );
    }
}
