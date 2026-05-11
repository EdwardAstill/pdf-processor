//! Cross-page furniture detection for repeated margin text.
//!
//! Running headers, footers, and boilerplate watermarks usually repeat in
//! consistent page bands. This pass builds page-local exclusion boxes before
//! formula detection and Markdown rendering.

use std::collections::{HashMap, HashSet};

use crate::document::types::{Bbox, RawPage, RawWord};

const MARGIN_FRACTION: f32 = 0.08;
const SIDE_FRACTION: f32 = 0.10;
const REPEAT_THRESHOLD: f32 = 0.5;

/// Returns page number to repeated margin-region boxes.
pub fn detect_furniture_bboxes(pages: &[RawPage]) -> HashMap<usize, Vec<Bbox>> {
    if pages.len() < 2 {
        return HashMap::new();
    }

    let margin_lines: Vec<Vec<FurnitureLine>> = pages
        .iter()
        .map(|page| {
            let mut candidates = collect_margin_lines(page);
            candidates.extend(collect_side_words(page));
            candidates
        })
        .collect();
    let mut frequency: HashMap<String, usize> = HashMap::new();

    for lines in &margin_lines {
        let mut seen = HashSet::new();
        for line in lines {
            if seen.insert(line.signature.clone()) {
                *frequency.entry(line.signature.clone()).or_insert(0) += 1;
            }
        }
    }

    let min_count = ((pages.len() as f32 * REPEAT_THRESHOLD).ceil() as usize).max(2);
    let mut result = HashMap::new();

    for (page, lines) in pages.iter().zip(margin_lines.iter()) {
        for line in lines {
            if frequency.get(&line.signature).copied().unwrap_or(0) >= min_count {
                result
                    .entry(page.page_num)
                    .or_insert_with(Vec::new)
                    .push(line.bbox);
            }
        }
    }

    result
}

#[derive(Clone, Debug)]
struct FurnitureLine {
    signature: String,
    bbox: Bbox,
}

fn collect_margin_lines(page: &RawPage) -> Vec<FurnitureLine> {
    let top_limit = page.height * MARGIN_FRACTION;
    let bottom_limit = page.height * (1.0 - MARGIN_FRACTION);
    let mut words: Vec<&RawWord> = page
        .words
        .iter()
        .filter(|word| word.bbox.y0 < top_limit || word.bbox.y1 > bottom_limit)
        .filter(|word| !normalise_token(&word.text).is_empty())
        .collect();

    words.sort_by(|a, b| {
        a.baseline_y
            .partial_cmp(&b.baseline_y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.bbox
                    .x0
                    .partial_cmp(&b.bbox.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut groups: Vec<Vec<&RawWord>> = Vec::new();
    for word in words {
        let tolerance = (word.font_size * 0.8).clamp(3.0, 10.0);
        if let Some(group) = groups.last_mut() {
            let baseline = average_baseline(group);
            if (baseline - word.baseline_y).abs() <= tolerance {
                group.push(word);
                continue;
            }
        }
        groups.push(vec![word]);
    }

    groups
        .into_iter()
        .filter_map(|mut group| {
            group.sort_by(|a, b| {
                a.bbox
                    .x0
                    .partial_cmp(&b.bbox.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let first = group.first()?;
            let mut bbox = first.bbox;
            let mut tokens = Vec::new();
            for word in group {
                bbox = bbox.union(&word.bbox);
                tokens.push(normalise_token(&word.text));
            }
            let signature = tokens.join(" ");
            if signature.is_empty() {
                None
            } else {
                Some(FurnitureLine { signature, bbox })
            }
        })
        .collect()
}

fn collect_side_words(page: &RawPage) -> Vec<FurnitureLine> {
    let left_limit = page.width * SIDE_FRACTION;
    let right_limit = page.width * (1.0 - SIDE_FRACTION);

    page.words
        .iter()
        .filter(|word| word.bbox.x1 <= left_limit || word.bbox.x0 >= right_limit)
        .filter_map(|word| {
            let signature = normalise_token(&word.text);
            if signature.len() < 4 {
                None
            } else {
                Some(FurnitureLine {
                    signature,
                    bbox: word.bbox,
                })
            }
        })
        .collect()
}

fn average_baseline(words: &[&RawWord]) -> f32 {
    words.iter().map(|word| word.baseline_y).sum::<f32>() / words.len().max(1) as f32
}

fn normalise_token(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
