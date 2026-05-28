//! Geometric LaTeX recovery from per-word baseline and font-size data.
//!
//! The core insight: `RawWord` carries `baseline_y`, `font_size`, and `bbox`
//! per word. When words in a formula region sit on different baselines or use
//! different font sizes, we can infer subscripts, superscripts, fractions, and
//! operator limits — structure that plain `unicode_to_latex` cannot recover
//! from a flat concatenated string.

use crate::document::types::Bbox;
use crate::formula::detect::FormulaWord;

const SUBSCRIPT_BASELINE_RATIO: f32 = 0.30;
const SUPERSCRIPT_BASELINE_RATIO: f32 = 0.30;
const SCRIPT_SIZE_RATIO: f32 = 0.80;
const FRACTION_OVERLAP: f32 = 0.50;
const FRACTION_GAP_RATIO: f32 = 2.0;

/// Produce LaTeX from formula words using baseline and font-size geometry.
/// Falls back to the provided `fallback` closure when geometry doesn't suggest
/// structure.
pub fn geometric_latex(
    words: &[FormulaWord],
    source_text: &str,
    fallback: impl Fn(&str) -> String,
) -> String {
    if words.len() < 2 || !is_formula_content(words, source_text) {
        return fallback(source_text);
    }

    // Filter out prose noise before baseline grouping.
    let filtered: Vec<FormulaWord> = words
        .iter()
        .filter(|w| !is_formula_noise_word(&w.text))
        .cloned()
        .collect();
    if filtered.len() < 2 {
        return fallback(source_text);
    }

    let median_baseline = median(filtered.iter().map(|w| w.baseline_y));
    let median_font = median(filtered.iter().map(|w| w.font_size));

    // Group filtered words into baseline rows.
    let mut rows = group_by_baseline(&filtered, median_font);

    // Merge isolated script-sized rows back into their nearest neighbor.
    merge_script_rows(&mut rows, median_font);

    // Detect fraction: two vertically stacked rows with horizontal overlap.
    if let Some(frac) = try_fraction(&rows, words, median_font) {
        return frac;
    }

    // Detect large operator with limits.
    if let Some(op) = try_operator_limits(&rows, median_baseline, median_font, &fallback) {
        return op;
    }

    // Single-row case: detect subscripts/superscripts within the row.
    if rows.len() == 1 {
        return render_row_with_scripts(&rows[0], median_baseline, median_font, &fallback);
    }

    // Multi-row with no detected structure: render rows separated by newlines.
    let mut parts = Vec::new();
    for row in &rows {
        parts.push(render_row_with_scripts(
            row,
            median_baseline,
            median_font,
            &fallback,
        ));
    }
    parts.join(" \\\\ ")
}

/// A group of words on approximately the same baseline, sorted left-to-right.
#[derive(Debug, Clone)]
struct BaselineRow {
    words: Vec<FormulaWord>,
    baseline_y: f32,
    bbox: Bbox,
}

fn group_by_baseline(words: &[FormulaWord], median_font: f32) -> Vec<BaselineRow> {
    let mut sorted: Vec<&FormulaWord> = words.iter().collect();
    sorted.sort_by(|a, b| {
        a.baseline_y
            .partial_cmp(&b.baseline_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let tolerance = (median_font * 0.40).clamp(2.0, 8.0);
    let mut rows: Vec<BaselineRow> = Vec::new();
    for word in sorted {
        let placed = rows.iter_mut().any(|row| {
            if (row.baseline_y - word.baseline_y).abs() <= tolerance {
                row.words.push(word.clone());
                row.words.sort_by(|a, b| {
                    a.bbox
                        .x0
                        .partial_cmp(&b.bbox.x0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                row.bbox = row.bbox.union(&word.bbox);
                row.baseline_y = average_baseline(&row.words);
                true
            } else {
                false
            }
        });

        if !placed {
            rows.push(BaselineRow {
                words: vec![word.clone()],
                baseline_y: word.baseline_y,
                bbox: word.bbox,
            });
        }
    }

    rows.sort_by(|a, b| {
        a.baseline_y
            .partial_cmp(&b.baseline_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

/// Merge a row into a neighbor if it's vertically close and the row's words
/// all sit entirely within the neighbor's horizontal span (or vice versa).
fn merge_script_rows(rows: &mut Vec<BaselineRow>, median_font: f32) {
    let merge_gap = median_font * 1.3;
    loop {
        let mut merged = false;
        for i in (0..rows.len()).rev() {
            let mut best_j = None;
            let mut best_dist = f32::MAX;
            for j in 0..rows.len() {
                if i == j {
                    continue;
                }
                let gap = (rows[j].baseline_y - rows[i].baseline_y).abs();
                if gap < merge_gap && gap < best_dist {
                    best_dist = gap;
                    best_j = Some(j);
                }
            }
            if let Some(j) = best_j {
                let removed = rows.remove(i);
                rows[j].words.extend(removed.words);
                rows[j].words.sort_by(|a, b| {
                    a.bbox
                        .x0
                        .partial_cmp(&b.bbox.x0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                rows[j].bbox = rows[j].bbox.union(&removed.bbox);
                rows[j].baseline_y = average_baseline(&rows[j].words);
                merged = true;
            }
        }
        if !merged {
            break;
        }
    }
    rows.sort_by(|a, b| {
        a.baseline_y
            .partial_cmp(&b.baseline_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn try_fraction(
    rows: &[BaselineRow],
    _all_words: &[FormulaWord],
    median_font: f32,
) -> Option<String> {
    if rows.len() != 2 {
        return None;
    }

    let top = &rows[0];
    let bottom = &rows[1];
    let overlap = horizontal_overlap(top.bbox, bottom.bbox);
    if overlap < FRACTION_OVERLAP {
        return None;
    }

    let gap = bottom.baseline_y - top.baseline_y;
    if gap > median_font * FRACTION_GAP_RATIO || gap < median_font * 0.8 {
        return None;
    }

    let top_text: String = top
        .words
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let bottom_text: String = bottom
        .words
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    Some(format!(
        "\\frac{{{}}}{{{}}}",
        unicode_to_latex(&top_text),
        unicode_to_latex(&bottom_text)
    ))
}

fn try_operator_limits(
    rows: &[BaselineRow],
    median_baseline: f32,
    median_font: f32,
    fallback: impl Fn(&str) -> String,
) -> Option<String> {
    // Find row with large operator (∑, ∏, ∫)
    let op_row_idx = rows.iter().position(|row| {
        row.words.iter().any(|w| {
            let t = w.text.trim();
            t == "∑" || t == "∏" || t == "∫"
        })
    })?;

    let op_row = &rows[op_row_idx];
    let op_word_idx = op_row
        .words
        .iter()
        .position(|w| {
            let t = w.text.trim();
            t == "∑" || t == "∏" || t == "∫"
        })
        .unwrap();

    let op_word = &op_row.words[op_word_idx];
    let op_latex = unicode_to_latex(&op_word.text);

    // Words above the operator (higher baseline = lower Y)
    let above: Vec<_> = rows
        .iter()
        .filter(|row| row.baseline_y < op_row.baseline_y - median_font * 0.3)
        .flat_map(|row| row.words.iter())
        .map(|w| unicode_to_latex(&w.text))
        .collect();

    // Words below the operator
    let below: Vec<_> = rows
        .iter()
        .filter(|row| row.baseline_y > op_row.baseline_y + median_font * 0.3)
        .flat_map(|row| row.words.iter())
        .map(|w| unicode_to_latex(&w.text))
        .collect();

    // Left-of-operator words (same row, before op)
    let left: String = op_row.words[..op_word_idx]
        .iter()
        .map(|w| unicode_to_latex(&w.text))
        .collect::<Vec<_>>()
        .join(" ");

    // Right-of-operator words (same row, after op)
    let right: String = op_row.words[op_word_idx + 1..]
        .iter()
        .map(|w| unicode_to_latex(&w.text))
        .collect::<Vec<_>>()
        .join(" ");

    let mut result = String::new();
    if !left.is_empty() {
        result.push_str(&left);
        result.push(' ');
    }
    result.push_str(&op_latex);

    if !below.is_empty() || !above.is_empty() {
        result.push('_');
        result.push('{');
        result.push_str(&below.join(" "));
        result.push('}');
        if !above.is_empty() {
            result.push('^');
            result.push('{');
            result.push_str(&above.join(" "));
            result.push('}');
        }
    }

    if !right.is_empty() {
        result.push(' ');
        result.push_str(&right);
    }

    // Render remaining rows that aren't part of the operator.
    let other_rows: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(idx, _)| *idx != op_row_idx)
        .filter(|(_, row)| {
            // Skip rows we already handled as above/below limits
            let is_above = above.iter().any(|_| {
                row.words.iter().any(|w| {
                    unicode_to_latex(&w.text) == *above.first().unwrap_or(&String::new()).as_str()
                })
            });
            !is_above
        })
        .map(|(_, row)| render_row_with_scripts(row, median_baseline, median_font, &fallback))
        .collect::<Vec<_>>();

    if !other_rows.is_empty() {
        result.push_str(" \\\\ ");
        result.push_str(&other_rows.join(" \\\\ "));
    }

    Some(result)
}

fn render_row_with_scripts(
    row: &BaselineRow,
    _median_baseline: f32,
    median_font: f32,
    _fallback: impl Fn(&str) -> String,
) -> String {
    if row.words.len() == 1 {
        return unicode_to_latex(&row.words[0].text);
    }

    // Use the baseline of the largest-font word(s) as reference.
    let script_threshold = median_font * SCRIPT_SIZE_RATIO;
    let main_baseline = {
        let max_font = row.words.iter().map(|w| w.font_size).fold(0.0f32, f32::max);
        let main_values: Vec<f32> = row
            .words
            .iter()
            .filter(|w| w.font_size >= max_font * 0.9)
            .map(|w| w.baseline_y)
            .collect();
        if main_values.is_empty() {
            average_baseline(&row.words)
        } else {
            let mut sorted = main_values;
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            sorted[(sorted.len() - 1) / 2]
        }
    };

    let mut parts = Vec::new();
    for word in &row.words {
        let is_sub = (word.baseline_y - main_baseline) > median_font * SUBSCRIPT_BASELINE_RATIO;
        let is_super = (main_baseline - word.baseline_y) > median_font * SUPERSCRIPT_BASELINE_RATIO;
        let is_script_size = word.font_size < script_threshold;

        let text = unicode_to_latex(&word.text);
        if is_sub || (is_script_size && word.baseline_y > main_baseline) {
            parts.push(format!("_{{{}}}", text));
        } else if is_super || (is_script_size && word.baseline_y < main_baseline) {
            parts.push(format!("^{{{}}}", text));
        } else {
            parts.push(text);
        }
    }

    parts.join(" ")
}

fn horizontal_overlap(a: Bbox, b: Bbox) -> f32 {
    let inter_x0 = a.x0.max(b.x0);
    let inter_x1 = a.x1.min(b.x1);
    let inter_width = (inter_x1 - inter_x0).max(0.0);
    let min_width = a.width().min(b.width());
    if min_width <= 0.0 {
        return 0.0;
    }
    inter_width / min_width
}

/// Only allow geometric recovery on genuine formula content, not prose.
fn is_formula_content(words: &[FormulaWord], source_text: &str) -> bool {
    // Unicode math operators that indicate real formulas.
    for word in words {
        let t = word.text.trim();
        if matches!(t, "∑" | "∏" | "∫" | "√" | "∂" | "∞") {
            return true;
        }
        // Unicode sub/superscript digits (genuine formula indicators)
        if t.contains('₁')
            || t.contains('₂')
            || t.contains('₃')
            || t.contains('ⁱ')
            || t.contains('ⁿ')
        {
            return true;
        }
        // Standalone math operator/relation words — not embedded in prose.
        if matches!(
            t,
            "=" | "+" | "−" | "≤" | "≥" | "≠" | "≈" | "→" | "⇒" | "×" | "÷"
        ) {
            return true;
        }
    }
    // Explicit sub/superscript underscores/carets in source (e.g. a_n, x^2).
    if source_text.contains('_')
        || source_text.contains('^')
        || source_text.contains("\\frac")
        || source_text.contains("\\sum")
    {
        return true;
    }
    false
}

/// Words that are prose noise, not formula content.
fn is_formula_noise_word(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    // Never filter math operators.
    if matches!(
        trimmed,
        "+" | "−" | "=" | "×" | "÷" | "≤" | "≥" | "∑" | "∏" | "∫" | "√"
    ) {
        return false;
    }
    // Short parenthesized numbers: (1), (12), (3)
    if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() <= 6 {
        return true;
    }
    // URLs and domain-like text
    if trimmed.contains("http")
        || trimmed.contains("www")
        || trimmed.contains("://")
        || trimmed.contains(".com")
        || trimmed.contains(".org")
    {
        return true;
    }
    // Bullet characters and punctuation-only
    if trimmed == "•"
        || trimmed == "·"
        || trimmed == "●"
        || trimmed == "◦"
        || trimmed.chars().all(|c| c.is_ascii_punctuation())
    {
        return true;
    }
    false
}

fn median(values: impl Iterator<Item = f32>) -> f32 {
    let mut v: Vec<f32> = values.collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = v.len() / 2;
    if v.len().is_multiple_of(2) && mid > 0 {
        (v[mid - 1] + v[mid]) / 2.0
    } else {
        v[mid]
    }
}

fn average_baseline(words: &[FormulaWord]) -> f32 {
    if words.is_empty() {
        return 0.0;
    }
    words.iter().map(|w| w.baseline_y).sum::<f32>() / words.len() as f32
}

fn unicode_to_latex(s: &str) -> String {
    s.chars()
        .fold(String::with_capacity(s.len() + 16), |mut out, c| {
            match c {
                'α' => out.push_str("\\alpha "),
                'β' => out.push_str("\\beta "),
                'γ' => out.push_str("\\gamma "),
                'δ' => out.push_str("\\delta "),
                'ε' => out.push_str("\\varepsilon "),
                'ζ' => out.push_str("\\zeta "),
                'η' => out.push_str("\\eta "),
                'θ' => out.push_str("\\theta "),
                'λ' => out.push_str("\\lambda "),
                'μ' => out.push_str("\\mu "),
                'ν' => out.push_str("\\nu "),
                'ξ' => out.push_str("\\xi "),
                'π' => out.push_str("\\pi "),
                'ρ' => out.push_str("\\rho "),
                'σ' => out.push_str("\\sigma "),
                'τ' => out.push_str("\\tau "),
                'φ' => out.push_str("\\phi "),
                'χ' => out.push_str("\\chi "),
                'ψ' => out.push_str("\\psi "),
                'ω' => out.push_str("\\omega "),
                'Γ' => out.push_str("\\Gamma "),
                'Δ' | '∆' => out.push_str("\\Delta "),
                'Θ' => out.push_str("\\Theta "),
                'Λ' => out.push_str("\\Lambda "),
                'Π' => out.push_str("\\Pi "),
                'Σ' => out.push_str("\\Sigma "),
                'Φ' => out.push_str("\\Phi "),
                'Ψ' => out.push_str("\\Psi "),
                'Ω' => out.push_str("\\Omega "),
                '∑' => out.push_str("\\sum "),
                '∏' => out.push_str("\\prod "),
                '∫' => out.push_str("\\int "),
                '∂' => out.push_str("\\partial "),
                '∞' => out.push_str("\\infty "),
                '√' => out.push_str("\\sqrt{} "),
                '±' => out.push_str("\\pm "),
                '∓' => out.push_str("\\mp "),
                '×' => out.push_str("\\times "),
                '÷' => out.push_str("\\div "),
                '≤' => out.push_str("\\leq "),
                '≥' => out.push_str("\\geq "),
                '≠' => out.push_str("\\neq "),
                '≈' => out.push_str("\\approx "),
                '∝' => out.push_str("\\propto "),
                '∈' => out.push_str("\\in "),
                '∉' => out.push_str("\\notin "),
                '⊂' => out.push_str("\\subset "),
                '∪' => out.push_str("\\cup "),
                '∩' => out.push_str("\\cap "),
                '−' => out.push('-'),
                _ => out.push(c),
            }
            out
        })
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(x: f32, y: f32, size: f32, text: &str) -> FormulaWord {
        FormulaWord {
            text: text.to_string(),
            bbox: Bbox::new(x, y - size, x + size * text.len() as f32, y),
            baseline_y: y,
            font_size: size,
        }
    }

    fn render(words: &[FormulaWord], source: &str) -> String {
        geometric_latex(words, source, |s| unicode_to_latex(s))
    }

    #[test]
    fn flat_line_produces_unicode_only() {
        let words = vec![
            word(10.0, 50.0, 12.0, "a"),
            word(25.0, 50.0, 12.0, "+"),
            word(40.0, 50.0, 12.0, "b"),
        ];
        assert_eq!(render(&words, "a + b"), "a + b");
    }

    #[test]
    fn subscript_baseline_drop() {
        let words = vec![word(10.0, 50.0, 12.0, "a"), word(30.0, 56.0, 9.0, "n")];
        assert_eq!(render(&words, "a_n"), "a _{n}");
    }

    #[test]
    fn superscript_baseline_rise() {
        let words = vec![word(10.0, 50.0, 12.0, "x"), word(30.0, 42.0, 9.0, "2")];
        assert_eq!(render(&words, "x^2"), "x ^{2}");
    }

    #[test]
    fn fraction_two_stacked_rows() {
        let words = vec![
            word(20.0, 40.0, 11.0, "a"),
            word(40.0, 40.0, 11.0, "+"),
            word(55.0, 40.0, 11.0, "b"),
            word(20.0, 62.0, 11.0, "c"),
            word(40.0, 62.0, 11.0, "+"),
            word(55.0, 62.0, 11.0, "d"),
        ];
        assert_eq!(render(&words, "a + b c + d"), "\\frac{a + b}{c + d}");
    }

    #[test]
    fn large_operator_with_limits() {
        let words = vec![
            word(20.0, 40.0, 8.0, "n"),
            word(50.0, 52.0, 16.0, "∑"),
            word(80.0, 52.0, 12.0, "i"),
            word(55.0, 66.0, 9.0, "i"),
            word(50.0, 66.0, 9.0, "="),
            word(45.0, 66.0, 9.0, "1"),
        ];
        let result = render(&words, "n ∑ i i = 1");
        assert!(result.contains("\\sum"), "expected \\sum, got: {result}");
    }

    #[test]
    fn empty_words_falls_back() {
        let words: Vec<FormulaWord> = Vec::new();
        assert_eq!(
            geometric_latex(&words, "E = mc^2", |s| s.to_string()),
            "E = mc^2"
        );
    }

    #[test]
    fn single_word_passes_through() {
        let words = vec![word(10.0, 50.0, 12.0, "E")];
        assert_eq!(render(&words, "E"), "E");
    }

    #[test]
    fn prose_text_uses_fallback_not_geometric() {
        let words = vec![
            word(10.0, 50.0, 12.0, "trained"),
            word(70.0, 56.0, 9.0, "left-to-right"),
        ];
        // No math content gate should trigger, so fallback is used.
        assert_eq!(
            geometric_latex(&words, "trained left-to-right", |s| s.to_string()),
            "trained left-to-right"
        );
    }

    #[test]
    fn is_formula_content_detects_operators() {
        let words = vec![word(10.0, 50.0, 12.0, "∑")];
        assert!(super::is_formula_content(&words, "∑"));

        let words = vec![
            word(10.0, 50.0, 12.0, "a"),
            word(30.0, 50.0, 12.0, "="),
            word(50.0, 50.0, 12.0, "b"),
        ];
        assert!(super::is_formula_content(&words, "a = b"));

        // Stray = in prose is NOT a formula
        let words = vec![word(10.0, 50.0, 12.0, "eters=110M")];
        assert!(!super::is_formula_content(&words, "eters=110M"));
    }

    #[test]
    fn is_formula_content_rejects_prose() {
        let words = vec![
            word(10.0, 50.0, 12.0, "trained"),
            word(70.0, 50.0, 12.0, "BERTBASE"),
        ];
        assert!(!super::is_formula_content(&words, "trained BERTBASE"));
    }

    #[test]
    fn is_formula_noise_word_filters_equation_numbers() {
        assert!(super::is_formula_noise_word("(3)"));
        assert!(super::is_formula_noise_word("(12)"));
        assert!(!super::is_formula_noise_word("x"));
        assert!(!super::is_formula_noise_word("∑"));
    }

    #[test]
    fn is_formula_noise_word_filters_urls() {
        assert!(super::is_formula_noise_word("https://github.com"));
        assert!(!super::is_formula_noise_word("github"));
    }

    #[test]
    fn is_formula_noise_word_filters_bullets() {
        assert!(super::is_formula_noise_word("•"));
        assert!(!super::is_formula_noise_word("item"));
    }
}
