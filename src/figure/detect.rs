use serde::Serialize;

use crate::document::types::{Bbox, Block, BlockKind, ImageRef, RawPage};

#[derive(Clone, Copy, Debug)]
pub struct FigureDetectionOptions {
    pub padding: f32,
    pub min_width_ratio: f32,
    pub min_height_ratio: f32,
}

impl Default for FigureDetectionOptions {
    fn default() -> Self {
        Self {
            padding: 8.0,
            min_width_ratio: 0.10,
            min_height_ratio: 0.06,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FigureCandidate {
    pub page_num: usize,
    pub figure_index: usize,
    pub bbox: Bbox,
    pub caption_bbox: Option<Bbox>,
    pub caption_text: Option<String>,
    pub seed_image_indices: Vec<usize>,
    pub confidence: u8,
    pub reason: String,
}

pub fn detect_figure_candidates(
    raw_page: &RawPage,
    blocks: &[Block],
    options: FigureDetectionOptions,
) -> Vec<FigureCandidate> {
    let page_bbox = raw_page.bbox();
    let visual_seeds = significant_images(raw_page, &options);
    let captions = caption_blocks(blocks);
    let mut candidates = Vec::new();
    let mut consumed_images = vec![false; raw_page.image_refs.len()];

    for caption in captions {
        let nearby = nearby_images_for_caption(caption.bbox, &visual_seeds, raw_page.height);
        if nearby.is_empty() {
            if let Some((bbox, confidence, reason)) =
                estimate_caption_only_region(caption.bbox, raw_page.width, raw_page.height, blocks)
            {
                let mut padded = pad_and_clamp(bbox, options.padding, page_bbox);
                if padded.overlaps(&caption.bbox) {
                    if bbox.center_y() < caption.bbox.center_y() {
                        padded.y1 = padded.y1.min(caption.bbox.y0 - 1.0).max(padded.y0);
                    } else {
                        padded.y0 = padded.y0.max(caption.bbox.y1 + 1.0).min(padded.y1);
                    }
                }
                candidates.push(FigureCandidate {
                    page_num: raw_page.page_num,
                    figure_index: candidates.len(),
                    bbox: padded,
                    caption_bbox: Some(caption.bbox),
                    caption_text: Some(caption.text.trim().to_string()),
                    seed_image_indices: Vec::new(),
                    confidence,
                    reason,
                });
            }
            continue;
        }

        let mut bbox = nearby[0].bbox;
        let mut seed_image_indices = Vec::new();
        for image in nearby {
            bbox = bbox.union(&image.bbox);
            seed_image_indices.push(image.image_index);
            if image.image_index < consumed_images.len() {
                consumed_images[image.image_index] = true;
            }
        }

        candidates.push(FigureCandidate {
            page_num: raw_page.page_num,
            figure_index: candidates.len(),
            bbox: pad_and_clamp(bbox, options.padding, page_bbox),
            caption_bbox: Some(caption.bbox),
            caption_text: Some(caption.text.trim().to_string()),
            seed_image_indices,
            confidence: 82,
            reason: "caption-near-images".to_string(),
        });
    }

    for group in image_only_groups(
        &visual_seeds,
        &consumed_images,
        raw_page.width,
        raw_page.height,
    ) {
        let mut bbox = group[0].bbox;
        let mut seed_image_indices = Vec::new();
        for image in group {
            bbox = bbox.union(&image.bbox);
            seed_image_indices.push(image.image_index);
        }

        candidates.push(FigureCandidate {
            page_num: raw_page.page_num,
            figure_index: candidates.len(),
            bbox: pad_and_clamp(bbox, options.padding, page_bbox),
            caption_bbox: None,
            caption_text: None,
            seed_image_indices,
            confidence: 52,
            reason: "image-group".to_string(),
        });
    }

    dedupe_candidates(candidates)
        .into_iter()
        .enumerate()
        .map(|(figure_index, mut candidate)| {
            candidate.figure_index = figure_index;
            candidate
        })
        .collect()
}

fn significant_images<'a>(
    raw_page: &'a RawPage,
    options: &FigureDetectionOptions,
) -> Vec<&'a ImageRef> {
    raw_page
        .image_refs
        .iter()
        .filter(|image| {
            let width_ratio = image.bbox.width() / raw_page.width.max(1.0);
            let height_ratio = image.bbox.height() / raw_page.height.max(1.0);
            let area_ratio =
                image.bbox.area() / (raw_page.width.max(1.0) * raw_page.height.max(1.0));
            let large_enough =
                width_ratio >= options.min_width_ratio || height_ratio >= options.min_height_ratio;
            large_enough
                && area_ratio >= 0.004
                && image.bbox.width() > 8.0
                && image.bbox.height() > 8.0
        })
        .collect()
}

fn caption_blocks(blocks: &[Block]) -> Vec<&Block> {
    blocks
        .iter()
        .filter(|block| matches!(block.kind, BlockKind::Caption) || looks_like_caption(&block.text))
        .collect()
}

fn looks_like_caption(text: &str) -> bool {
    let trimmed = text.trim_start().to_ascii_lowercase();
    ["figure", "fig.", "fig ", "table", "exhibit", "plate"]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

fn nearby_images_for_caption<'a>(
    caption_bbox: Bbox,
    images: &[&'a ImageRef],
    page_height: f32,
) -> Vec<&'a ImageRef> {
    let max_gap = (page_height * 0.28).max(120.0);
    let mut nearby: Vec<&ImageRef> = images
        .iter()
        .copied()
        .filter(|image| {
            let vertical_gap = if image.bbox.y1 <= caption_bbox.y0 {
                caption_bbox.y0 - image.bbox.y1
            } else if caption_bbox.y1 <= image.bbox.y0 {
                image.bbox.y0 - caption_bbox.y1
            } else {
                0.0
            };
            vertical_gap <= max_gap && horizontal_related(image.bbox, caption_bbox)
        })
        .collect();
    nearby.sort_by(|left, right| {
        left.bbox
            .y0
            .partial_cmp(&right.bbox.y0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    nearby
}

fn horizontal_related(a: Bbox, b: Bbox) -> bool {
    let overlap = a.x0 < b.x1 && a.x1 > b.x0;
    let center_distance = (a.center_x() - b.center_x()).abs();
    overlap || center_distance <= a.width().max(b.width()) * 0.75
}

fn estimate_caption_only_region(
    caption: Bbox,
    page_width: f32,
    page_height: f32,
    blocks: &[Block],
) -> Option<(Bbox, u8, String)> {
    let estimated_height = (page_height * 0.24).clamp(90.0, 220.0);
    let gap = 10.0;
    let above = Bbox::new(
        page_width * 0.08,
        (caption.y0 - estimated_height - gap).max(0.0),
        page_width * 0.92,
        (caption.y0 - gap).max(0.0),
    );
    let below = Bbox::new(
        page_width * 0.08,
        (caption.y1 + gap).min(page_height),
        page_width * 0.92,
        (caption.y1 + gap + estimated_height).min(page_height),
    );

    [("above", above), ("below", below)]
        .into_iter()
        .filter(|(_, bbox)| bbox.height() > page_height * 0.04)
        .map(|(direction, bbox)| {
            let contamination = body_text_contamination(bbox, caption, blocks);
            let edge_penalty = if bbox.y0 <= 1.0 || bbox.y1 >= page_height - 1.0 {
                8.0
            } else {
                0.0
            };
            let score = (46.0 - contamination * 85.0 - edge_penalty).clamp(8.0, 58.0);
            (direction, bbox, score)
        })
        .max_by(|left, right| {
            left.2
                .partial_cmp(&right.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    if left.0 == "above" && right.0 == "below" {
                        std::cmp::Ordering::Greater
                    } else if left.0 == "below" && right.0 == "above" {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
        })
        .map(|(direction, bbox, score)| {
            let confidence = score.round() as u8;
            let reason = if direction == "above" {
                "caption-only-estimate".to_string()
            } else {
                "caption-only-proposal-below".to_string()
            };
            (bbox, confidence, reason)
        })
}

fn body_text_contamination(region: Bbox, caption: Bbox, blocks: &[Block]) -> f32 {
    let overlap_area: f32 = blocks
        .iter()
        .filter(|block| {
            !matches!(block.kind, BlockKind::Caption) && !looks_like_caption(&block.text)
        })
        .filter(|block| !same_bbox(block.bbox, caption))
        .map(|block| intersection_area(region, block.bbox))
        .sum();
    (overlap_area / region.area().max(1.0)).clamp(0.0, 1.0)
}

fn same_bbox(a: Bbox, b: Bbox) -> bool {
    (a.x0 - b.x0).abs() < 0.1
        && (a.y0 - b.y0).abs() < 0.1
        && (a.x1 - b.x1).abs() < 0.1
        && (a.y1 - b.y1).abs() < 0.1
}

fn intersection_area(a: Bbox, b: Bbox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);
    ((x1 - x0).max(0.0)) * ((y1 - y0).max(0.0))
}

fn image_only_groups<'a>(
    images: &[&'a ImageRef],
    consumed: &[bool],
    page_width: f32,
    page_height: f32,
) -> Vec<Vec<&'a ImageRef>> {
    let mut unused: Vec<&ImageRef> = images
        .iter()
        .copied()
        .filter(|image| !consumed.get(image.image_index).copied().unwrap_or(false))
        .collect();
    unused.sort_by(|left, right| {
        left.bbox
            .y0
            .partial_cmp(&right.bbox.y0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.bbox
                    .x0
                    .partial_cmp(&right.bbox.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    let mut groups: Vec<Vec<&ImageRef>> = Vec::new();
    for image in unused {
        let page_area = page_width.max(1.0) * page_height.max(1.0);
        let area_ratio = image.bbox.area() / page_area;
        let centered = image.bbox.center_x() > page_width * 0.10
            && image.bbox.center_x() < page_width * 0.90
            && image.bbox.center_y() > page_height * 0.12
            && image.bbox.center_y() < page_height * 0.88;
        if area_ratio < 0.012 && !centered {
            continue;
        }

        if let Some(group) = groups.iter_mut().find(|group| {
            group
                .iter()
                .any(|other| images_close_enough(other.bbox, image.bbox, page_width, page_height))
        }) {
            group.push(image);
        } else {
            groups.push(vec![image]);
        }
    }
    merge_image_groups_by_band(groups, page_width)
}

fn merge_image_groups_by_band(groups: Vec<Vec<&ImageRef>>, page_width: f32) -> Vec<Vec<&ImageRef>> {
    let mut merged: Vec<Vec<&ImageRef>> = Vec::new();

    for group in groups {
        let bbox = group_bbox(&group);
        if let Some(existing) = merged.iter_mut().find(|existing| {
            let existing_bbox = group_bbox(existing);
            let horizontal_gap = horizontal_bbox_gap(existing_bbox, bbox);
            vertical_overlap_ratio(existing_bbox, bbox) >= 0.65
                && horizontal_gap <= page_width * 0.35
        }) {
            existing.extend(group);
        } else {
            merged.push(group);
        }
    }

    merged
}

fn group_bbox(group: &[&ImageRef]) -> Bbox {
    group
        .iter()
        .skip(1)
        .fold(group[0].bbox, |bbox, image| bbox.union(&image.bbox))
}

fn horizontal_bbox_gap(a: Bbox, b: Bbox) -> f32 {
    if a.x1 <= b.x0 {
        b.x0 - a.x1
    } else if b.x1 <= a.x0 {
        a.x0 - b.x1
    } else {
        0.0
    }
}

fn vertical_overlap_ratio(a: Bbox, b: Bbox) -> f32 {
    let overlap = a.y1.min(b.y1) - a.y0.max(b.y0);
    if overlap <= 0.0 {
        return 0.0;
    }
    overlap / a.height().min(b.height()).max(1.0)
}

fn images_close_enough(a: Bbox, b: Bbox, page_width: f32, page_height: f32) -> bool {
    let vertical_gap = if a.y1 <= b.y0 {
        b.y0 - a.y1
    } else if b.y1 <= a.y0 {
        a.y0 - b.y1
    } else {
        0.0
    };
    let horizontal_gap = if a.x1 <= b.x0 {
        b.x0 - a.x1
    } else if b.x1 <= a.x0 {
        a.x0 - b.x1
    } else {
        0.0
    };
    vertical_gap <= page_height * 0.08 && horizontal_gap <= page_width * 0.08
}

fn dedupe_candidates(mut candidates: Vec<FigureCandidate>) -> Vec<FigureCandidate> {
    candidates.sort_by(|left, right| {
        right.confidence.cmp(&left.confidence).then_with(|| {
            left.bbox
                .y0
                .partial_cmp(&right.bbox.y0)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    let mut kept: Vec<FigureCandidate> = Vec::new();
    for candidate in candidates {
        if kept
            .iter()
            .any(|existing| overlap_ratio(existing.bbox, candidate.bbox) > 0.72)
        {
            continue;
        }
        kept.push(candidate);
    }
    kept.sort_by(|left, right| {
        left.bbox
            .y0
            .partial_cmp(&right.bbox.y0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                left.bbox
                    .x0
                    .partial_cmp(&right.bbox.x0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    kept
}

fn overlap_ratio(a: Bbox, b: Bbox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);
    if x1 <= x0 || y1 <= y0 {
        return 0.0;
    }
    let intersection = (x1 - x0) * (y1 - y0);
    intersection / a.area().min(b.area()).max(1.0)
}

fn pad_and_clamp(bbox: Bbox, padding: f32, page: Bbox) -> Bbox {
    Bbox::new(
        (bbox.x0 - padding).max(page.x0),
        (bbox.y0 - padding).max(page.y0),
        (bbox.x1 + padding).min(page.x1),
        (bbox.y1 + padding).min(page.y1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_page(images: Vec<Bbox>) -> RawPage {
        RawPage {
            page_num: 0,
            width: 600.0,
            height: 800.0,
            blocks: Vec::new(),
            words: Vec::new(),
            image_refs: images
                .into_iter()
                .enumerate()
                .map(|(image_index, bbox)| ImageRef {
                    page_num: 0,
                    bbox,
                    image_index,
                    bytes: vec![1, 2, 3],
                    format: "png".to_string(),
                })
                .collect(),
        }
    }

    fn caption(y0: f32, text: &str) -> Block {
        Block {
            id: 1,
            bbox: Bbox::new(90.0, y0, 510.0, y0 + 18.0),
            text: text.to_string(),
            kind: BlockKind::Caption,
            font_size: 9.0,
            font_name: "test".to_string(),
            page_num: 0,
            reading_order: 0,
            bold: false,
            italic: false,
        }
    }

    fn paragraph(id: usize, bbox: Bbox, text: &str) -> Block {
        Block {
            id,
            bbox,
            text: text.to_string(),
            kind: BlockKind::Paragraph,
            font_size: 10.0,
            font_name: "test".to_string(),
            page_num: 0,
            reading_order: id,
            bold: false,
            italic: false,
        }
    }

    #[test]
    fn groups_multiple_panels_with_caption_below() {
        let raw = raw_page(vec![
            Bbox::new(100.0, 120.0, 260.0, 260.0),
            Bbox::new(280.0, 122.0, 440.0, 262.0),
        ]);
        let candidates = detect_figure_candidates(
            &raw,
            &[caption(285.0, "Figure 1: two panels")],
            FigureDetectionOptions::default(),
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].seed_image_indices, vec![0, 1]);
        assert!(candidates[0].bbox.x0 < 100.0);
        assert!(candidates[0].bbox.x1 > 440.0);
    }

    #[test]
    fn estimates_caption_only_region_above_caption() {
        let raw = raw_page(Vec::new());
        let candidates = detect_figure_candidates(
            &raw,
            &[caption(360.0, "Fig. 2. Vector diagram")],
            FigureDetectionOptions::default(),
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].reason, "caption-only-estimate");
        assert!(candidates[0].bbox.y1 < 360.0);
    }

    #[test]
    fn caption_only_region_scores_against_body_text() {
        let raw = raw_page(Vec::new());
        let caption = caption(360.0, "Figure 3: vector-only chart");
        let body = paragraph(
            2,
            Bbox::new(70.0, 130.0, 530.0, 330.0),
            "Dense body text above the caption should not become the figure crop.",
        );

        let candidates =
            detect_figure_candidates(&raw, &[body, caption], FigureDetectionOptions::default());

        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0].bbox.y0 >= 378.0,
            "expected the candidate below the caption, got {:?}",
            candidates[0].bbox
        );
        assert!(candidates[0].reason.contains("caption-only-proposal"));
    }

    #[test]
    fn caption_only_region_can_choose_region_below_caption() {
        let raw = raw_page(Vec::new());
        let caption = caption(180.0, "Fig. 4. Diagram below the caption");
        let header_text = paragraph(2, Bbox::new(75.0, 60.0, 520.0, 165.0), "Introductory prose");

        let candidates = detect_figure_candidates(
            &raw,
            &[header_text, caption],
            FigureDetectionOptions::default(),
        );

        assert_eq!(candidates.len(), 1);
        assert!(
            candidates[0].bbox.y0 > 198.0,
            "expected region below the caption, got {:?}",
            candidates[0].bbox
        );
    }

    #[test]
    fn rejects_tiny_decorative_image_without_caption() {
        let raw = raw_page(vec![Bbox::new(10.0, 10.0, 24.0, 24.0)]);
        let candidates = detect_figure_candidates(&raw, &[], FigureDetectionOptions::default());
        assert!(candidates.is_empty());
    }

    #[test]
    fn clamps_padding_to_page_bounds() {
        let raw = raw_page(vec![Bbox::new(4.0, 6.0, 260.0, 180.0)]);
        let candidates = detect_figure_candidates(
            &raw,
            &[caption(200.0, "Figure 1: near edge")],
            FigureDetectionOptions {
                padding: 20.0,
                ..Default::default()
            },
        );

        assert_eq!(candidates[0].bbox.x0, 0.0);
        assert_eq!(candidates[0].bbox.y0, 0.0);
    }

    #[test]
    fn merges_separated_panels_on_same_vertical_band() {
        let raw = raw_page(vec![
            Bbox::new(60.0, 70.0, 130.0, 400.0),
            Bbox::new(176.0, 70.0, 245.0, 400.0),
            Bbox::new(291.0, 70.0, 361.0, 400.0),
            Bbox::new(407.0, 70.0, 476.0, 400.0),
        ]);
        let candidates = detect_figure_candidates(&raw, &[], FigureDetectionOptions::default());

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].seed_image_indices, vec![0, 1, 2, 3]);
        assert!(candidates[0].bbox.x0 < 60.0);
        assert!(candidates[0].bbox.x1 > 476.0);
    }
}
