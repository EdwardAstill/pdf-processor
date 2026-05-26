//! Page media selection and repeated edge-text suppression.

use crate::document::types::{Block, BlockKind, Document, Page};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(crate) struct RenderContext {
    repeated_edge_text_first_seen: HashMap<String, usize>,
    repeated_media_fingerprints: HashSet<MediaFingerprint>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct MediaFingerprint {
    center_x_bin: u8,
    center_y_bin: u8,
    width_bin: u8,
    height_bin: u8,
    edge_band: u8,
}

#[derive(Default)]
pub(crate) struct PageMediaPlan {
    pub(crate) kept_block_ids: HashSet<usize>,
}

pub(crate) fn build_render_context(doc: &Document) -> RenderContext {
    let mut repeated_text_pages: HashMap<String, HashSet<usize>> = HashMap::new();
    let mut first_seen_text_page: HashMap<String, usize> = HashMap::new();
    let mut repeated_media_pages: HashMap<MediaFingerprint, HashSet<usize>> = HashMap::new();

    for page in &doc.pages {
        for block in &page.blocks {
            if let Some(key) = normalized_repeated_text_key(block, page) {
                repeated_text_pages
                    .entry(key.clone())
                    .or_default()
                    .insert(page.page_num);
                first_seen_text_page
                    .entry(key)
                    .and_modify(|first_seen| *first_seen = (*first_seen).min(page.page_num))
                    .or_insert(page.page_num);
            }

            if is_media_block(block) {
                repeated_media_pages
                    .entry(media_fingerprint(block, page))
                    .or_default()
                    .insert(page.page_num);
            }
        }
    }

    RenderContext {
        repeated_edge_text_first_seen: repeated_text_pages
            .into_iter()
            .filter_map(|(key, pages)| {
                (pages.len() >= 3).then_some((key.clone(), *first_seen_text_page.get(&key)?))
            })
            .collect(),
        repeated_media_fingerprints: repeated_media_pages
            .into_iter()
            .filter_map(|(fingerprint, pages)| (pages.len() >= 2).then_some(fingerprint))
            .collect(),
    }
}

pub(crate) fn build_page_media_plan(
    page: &Page,
    blocks: &[&Block],
    render_ctx: &RenderContext,
    scholarly_front_page: bool,
) -> PageMediaPlan {
    let media_blocks: Vec<&Block> = blocks
        .iter()
        .copied()
        .filter(|block| is_media_block(block))
        .collect();
    if media_blocks.is_empty() {
        return PageMediaPlan::default();
    }

    let content_block_count = blocks
        .iter()
        .copied()
        .filter(|block| {
            !is_media_block(block) && !should_suppress_repeated_text_block(block, page, render_ctx)
        })
        .count();
    let short_heading_count = blocks
        .iter()
        .copied()
        .filter(|block| {
            matches!(block.kind, BlockKind::Heading { .. })
                && block.text.split_whitespace().count() <= 12
        })
        .count();
    let layout_heavy = media_blocks.len() >= 6
        || (media_blocks.len() >= 4 && content_block_count <= 10)
        || short_heading_count >= 4;

    let mut cap = if scholarly_front_page || media_blocks.len() >= 10 {
        1usize
    } else if layout_heavy {
        2usize
    } else if media_blocks.len() >= 4 {
        3usize
    } else {
        media_blocks.len()
    };

    #[derive(Clone, Copy)]
    struct Candidate {
        block_id: usize,
        score: i32,
        hard_drop: bool,
        keep_even_if_capped: bool,
    }

    let mut candidates: Vec<Candidate> = media_blocks
        .iter()
        .map(|block| {
            let has_caption = media_has_caption(block, blocks, page);
            let bbox = block.bbox;
            let page_area = (page.width.max(1.0) * page.height.max(1.0)).max(1.0);
            let area_ratio = (bbox.area() / page_area).clamp(0.0, 1.0);
            let width_ratio = (bbox.width() / page.width.max(1.0)).clamp(0.0, 1.0);
            let height_ratio = (bbox.height() / page.height.max(1.0)).clamp(0.0, 1.0);
            let aspect_ratio = bbox.width().max(1.0) / bbox.height().max(1.0);
            let top_band = bbox.y1 <= page.height * 0.18;
            let bottom_band = bbox.y0 >= page.height * 0.84;
            let edge_band = top_band
                || bottom_band
                || bbox.x0 <= page.width * 0.06
                || bbox.x1 >= page.width * 0.94;
            let repeated = render_ctx
                .repeated_media_fingerprints
                .contains(&media_fingerprint(block, page));
            let tiny = area_ratio < 0.015 || (width_ratio < 0.12 && height_ratio < 0.12);
            let extreme_aspect = aspect_ratio >= 5.5 || aspect_ratio <= 0.22;
            let top_half = bbox.center_y() <= page.height * 0.55;
            let centered_body = bbox.center_y() > page.height * 0.18
                && bbox.center_y() < page.height * 0.82
                && bbox.center_x() > page.width * 0.12
                && bbox.center_x() < page.width * 0.88;

            let mut score = (area_ratio * 100.0).round() as i32;
            if matches!(block.kind, BlockKind::Figure { .. }) {
                score += 15;
            }
            if has_caption {
                score += 55;
            }
            if area_ratio >= 0.12 {
                score += 20;
            }
            if centered_body {
                score += 8;
            }
            if tiny {
                score -= 30;
            }
            if edge_band {
                score -= 20;
            }
            if repeated {
                score -= 30;
            }
            if extreme_aspect && area_ratio < 0.08 {
                score -= 20;
            }
            if layout_heavy && !has_caption {
                score -= 12;
            }
            if scholarly_front_page && top_half && !has_caption {
                score -= 55;
            }

            let hard_drop = (!has_caption && tiny && edge_band)
                || (!has_caption && repeated && (edge_band || scholarly_front_page))
                || (scholarly_front_page && top_half && !has_caption && area_ratio < 0.16)
                || (layout_heavy && !has_caption && repeated && area_ratio < 0.06);

            Candidate {
                block_id: block.id,
                score,
                hard_drop,
                keep_even_if_capped: has_caption && !bottom_band,
            }
        })
        .collect();

    let guaranteed = candidates
        .iter()
        .filter(|candidate| candidate.keep_even_if_capped && !candidate.hard_drop)
        .count();
    cap = cap.max(guaranteed);

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.block_id.cmp(&right.block_id))
    });

    PageMediaPlan {
        kept_block_ids: candidates
            .into_iter()
            .filter(|candidate| !candidate.hard_drop)
            .take(cap)
            .map(|candidate| candidate.block_id)
            .collect(),
    }
}

pub(crate) fn normalized_repeated_text_key(block: &Block, page: &Page) -> Option<String> {
    if !matches!(block.kind, BlockKind::Heading { .. } | BlockKind::Paragraph) {
        return None;
    }
    if block.bbox.y0 > page.height * 0.18 && block.bbox.y1 < page.height * 0.84 {
        return None;
    }

    let normalized = block
        .text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase();
    let alnum_count = normalized.chars().filter(|ch| ch.is_alphanumeric()).count();
    if normalized.is_empty() || normalized.len() > 120 || alnum_count < 4 {
        return None;
    }
    Some(normalized)
}

pub(crate) fn should_suppress_repeated_text_block(
    block: &Block,
    page: &Page,
    render_ctx: &RenderContext,
) -> bool {
    normalized_repeated_text_key(block, page)
        .and_then(|key| {
            render_ctx
                .repeated_edge_text_first_seen
                .get(&key)
                .copied()
                .map(|first_seen| page.page_num > first_seen)
        })
        .unwrap_or(false)
}

pub(crate) fn is_media_block(block: &Block) -> bool {
    matches!(
        block.kind,
        BlockKind::Image { .. } | BlockKind::Figure { .. }
    )
}

pub(crate) fn media_has_caption(block: &Block, blocks: &[&Block], page: &Page) -> bool {
    if matches!(
        block.kind,
        BlockKind::Figure {
            caption: Some(ref caption),
            ..
        } if !caption.trim().is_empty()
    ) {
        return true;
    }

    blocks.iter().copied().any(|other| {
        matches!(other.kind, BlockKind::Caption)
            && other.bbox.x0 < block.bbox.x1
            && other.bbox.x1 > block.bbox.x0
            && {
                let gap = if other.bbox.y0 >= block.bbox.y1 {
                    other.bbox.y0 - block.bbox.y1
                } else if block.bbox.y0 >= other.bbox.y1 {
                    block.bbox.y0 - other.bbox.y1
                } else {
                    0.0
                };
                gap <= page.height * 0.06
            }
    })
}

fn media_fingerprint(block: &Block, page: &Page) -> MediaFingerprint {
    fn quantize(value: f32) -> u8 {
        (value.clamp(0.0, 1.0) * 10.0).round() as u8
    }

    let width = page.width.max(1.0);
    let height = page.height.max(1.0);
    let bbox = block.bbox;
    let edge_band = if bbox.y1 <= page.height * 0.18 {
        0
    } else if bbox.y0 >= page.height * 0.84 {
        1
    } else {
        2
    };

    MediaFingerprint {
        center_x_bin: quantize(bbox.center_x() / width),
        center_y_bin: quantize(bbox.center_y() / height),
        width_bin: quantize(bbox.width() / width),
        height_bin: quantize(bbox.height() / height),
        edge_band,
    }
}
