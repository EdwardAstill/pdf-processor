//! Layout-level inference for implicit tables and form fields.
//!
//! This module detects structure from already-classified text blocks. It does
//! not render Markdown. Callers receive structural data and choose how to
//! serialise it.

use crate::document::types::{Block, BlockKind};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StructuredRegion {
    pub(crate) kind: StructuredRegionKind,
    pub(crate) next_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum StructuredRegionKind {
    /// Consecutive explicit table-cell blocks. The renderer should serialise
    /// `blocks[start..next_index]` using its normal table-cell renderer.
    TableCells,
    /// Paragraph/list-item run that forms a numeric table.
    NumericTable {
        headers: Vec<String>,
        rows: Vec<ParsedNumericRow>,
        total: Option<String>,
    },
    /// Paragraph/list-item run that forms a label/value field list.
    FormFields { fields: Vec<FormField> },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParsedNumericRow {
    pub(crate) label: String,
    pub(crate) values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FormField {
    pub(crate) label: String,
    pub(crate) value: Option<String>,
}

pub(crate) fn detect_structured_region(
    blocks: &[&Block],
    start: usize,
) -> Option<StructuredRegion> {
    match blocks[start].kind {
        BlockKind::TableCell { .. } => {
            let mut end = start + 1;
            while end < blocks.len() && matches!(blocks[end].kind, BlockKind::TableCell { .. }) {
                end += 1;
            }
            Some(StructuredRegion {
                kind: StructuredRegionKind::TableCells,
                next_index: end,
            })
        }
        BlockKind::Paragraph | BlockKind::ListItem { .. } => {
            detect_implicit_numeric_table(blocks, start)
                .or_else(|| detect_form_field_blocks(blocks, start))
        }
        _ => None,
    }
}

fn detect_implicit_numeric_table(blocks: &[&Block], start: usize) -> Option<StructuredRegion> {
    let end = collect_textish_run(blocks, start, 18);
    let run = &blocks[start..end];
    if run.len() < 3 {
        return None;
    }

    let mut count_histogram = std::collections::BTreeMap::new();
    let mut parsed_rows: Vec<Option<ParsedNumericRow>> = Vec::with_capacity(run.len());

    for block in run {
        let parsed = parse_numeric_row(block.text.trim());
        if let Some(row) = &parsed {
            if row.values.len() >= 2 {
                *count_histogram.entry(row.values.len()).or_insert(0usize) += 1;
            }
        }
        parsed_rows.push(parsed);
    }

    let dominant_count = count_histogram
        .into_iter()
        .max_by_key(|(count, hits)| (*hits, *count))
        .and_then(|(count, hits)| if hits >= 3 { Some(count) } else { None })?;
    let parsed_rows: Vec<Option<ParsedNumericRow>> = parsed_rows
        .into_iter()
        .map(|row| row.map(|row| normalize_numeric_row(row, dominant_count)))
        .collect();

    let mut header_index = None;
    if parsed_rows
        .first()
        .and_then(|row| row.as_ref())
        .map(|row| row.values.is_empty())
        .unwrap_or(false)
        && looks_like_table_header(run[0].text.trim(), dominant_count)
    {
        header_index = Some(0usize);
    }

    let data_start = header_index.map(|idx| idx + 1).unwrap_or(0);
    let mut data_end = data_start;
    while data_end < parsed_rows.len() {
        match &parsed_rows[data_end] {
            Some(row) if row.values.len() == dominant_count => data_end += 1,
            _ => break,
        }
    }

    if data_end - data_start < 3 {
        return None;
    }

    let invoice_header = header_index
        .map(|idx| looks_like_invoice_header(run[idx].text.trim()))
        .unwrap_or(false);
    let headers = derive_table_headers(
        header_index.map(|idx| run[idx].text.trim()),
        dominant_count,
        invoice_header,
    );
    let rows = parsed_rows[data_start..data_end]
        .iter()
        .flatten()
        .cloned()
        .collect();

    let mut consumed = data_end;
    let total = run
        .get(data_end)
        .and_then(|block| extract_single_numeric_value(block.text.trim()))
        .filter(|_| invoice_header);
    if total.is_some() {
        consumed += 1;
    }

    Some(StructuredRegion {
        kind: StructuredRegionKind::NumericTable {
            headers,
            rows,
            total,
        },
        next_index: start + consumed,
    })
}

fn detect_form_field_blocks(blocks: &[&Block], start: usize) -> Option<StructuredRegion> {
    let end = collect_textish_run(blocks, start, 12);
    let run = &blocks[start..end];
    if run.len() < 3 {
        return None;
    }

    let mut fields = Vec::new();
    let mut idx = 0usize;

    while idx < run.len() {
        let text = normalize_structured_text(run[idx].text.trim());
        if !looks_like_field_label(&text) {
            break;
        }

        let label = normalize_field_label(&text);
        let mut value = extract_inline_field_value(&text).map(str::to_string);
        let mut consumed = 1usize;

        if value.is_none() {
            if let Some(next_text) = run
                .get(idx + 1)
                .map(|block| normalize_structured_text(block.text.trim()))
            {
                if looks_like_choice_line(&next_text) {
                    value = Some(next_text.split_whitespace().collect::<Vec<_>>().join(" | "));
                    consumed = 2;
                } else if looks_like_field_value(&next_text) {
                    value = Some(next_text);
                    consumed = 2;
                }
            }
        }

        fields.push(FormField { label, value });
        idx += consumed;
    }

    if fields.len() >= 3 {
        Some(StructuredRegion {
            kind: StructuredRegionKind::FormFields { fields },
            next_index: start + idx,
        })
    } else {
        None
    }
}

fn collect_textish_run(blocks: &[&Block], start: usize, limit: usize) -> usize {
    let mut end = start;
    while end < blocks.len()
        && end - start < limit
        && matches!(
            blocks[end].kind,
            BlockKind::Paragraph | BlockKind::ListItem { .. }
        )
    {
        end += 1;
    }
    end
}

fn parse_numeric_row(text: &str) -> Option<ParsedNumericRow> {
    let trimmed = strip_leading_list_marker(&normalize_structured_text(text));
    if trimmed.is_empty() {
        return None;
    }

    let matches: Vec<_> = numeric_value_re().find_iter(&trimmed).collect();
    if matches.is_empty() {
        return Some(ParsedNumericRow {
            label: trimmed,
            values: Vec::new(),
        });
    }

    let mut trailing: Vec<(usize, usize, String)> = Vec::new();
    let mut next_end = trimmed.len();
    for found in matches.into_iter().rev() {
        if !is_numeric_separator(&trimmed[found.end()..next_end]) {
            if trailing.is_empty() {
                continue;
            }
            break;
        }
        trailing.push((found.start(), found.end(), found.as_str().to_string()));
        next_end = found.start();
    }
    trailing.reverse();

    let (first_start, _, _) = trailing.first()?;
    let label = trimmed[..*first_start]
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_string();
    if label.is_empty() {
        return None;
    }

    Some(ParsedNumericRow {
        label,
        values: trailing.into_iter().map(|(_, _, value)| value).collect(),
    })
}

pub(crate) fn normalize_numeric_row(
    mut row: ParsedNumericRow,
    dominant_count: usize,
) -> ParsedNumericRow {
    if row.values.len() == 1 {
        let value = row.values[0].trim();
        if value.len() == dominant_count && value.chars().all(|ch| ch == '0') {
            row.values = vec!["0".to_string(); dominant_count];
        }
    }
    row
}

fn looks_like_table_header(text: &str, value_count: usize) -> bool {
    let upper = text.chars().filter(|ch| ch.is_ascii_uppercase()).count();
    let alpha = text.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    alpha > 8
        && upper * 2 >= alpha
        && (looks_like_invoice_header(text)
            || (value_count >= 2 && text.split_whitespace().count() > value_count))
}

fn derive_table_headers(
    header_text: Option<&str>,
    value_count: usize,
    invoice_header: bool,
) -> Vec<String> {
    if invoice_header {
        let mut headers = vec!["Item".to_string()];
        headers.extend(
            ["Quantity", "Price", "Amount"]
                .iter()
                .take(value_count)
                .map(|s| s.to_string()),
        );
        if value_count > 3 {
            for idx in 4..=value_count {
                headers.push(format!("Value {idx}"));
            }
        }
        return headers;
    }

    if let Some(text) = header_text {
        if text.to_ascii_lowercase().contains("descr") && value_count == 4 {
            return vec![
                "Item".to_string(),
                "Value 1".to_string(),
                "Value 2".to_string(),
                "Value 3".to_string(),
                "Value 4".to_string(),
            ];
        }
    }

    let mut headers = vec!["Item".to_string()];
    for idx in 1..=value_count {
        headers.push(format!("Value {idx}"));
    }
    headers
}

fn numeric_value_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\(?-?(?:\d{1,3}(?:[.,]\d{3})+(?:[.,]\d{2})?|\d+(?:[.,]\d{2})|\d+)\)?").unwrap()
    })
}

fn invoice_header_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(item|description|quantity|price|amount|invoice)\b").unwrap()
    })
}

fn looks_like_invoice_header(text: &str) -> bool {
    invoice_header_re().find_iter(text).count() >= 2
}

fn extract_single_numeric_value(text: &str) -> Option<String> {
    let trimmed = normalize_structured_text(text);
    let matches: Vec<_> = numeric_value_re().find_iter(&trimmed).collect();
    if matches.len() == 1 && matches[0].start() == 0 && matches[0].end() == trimmed.len() {
        Some(trimmed)
    } else {
        None
    }
}

fn is_numeric_separator(text: &str) -> bool {
    text.chars()
        .all(|ch| ch.is_whitespace() || matches!(ch, '|' | '/' | '\\'))
}

fn normalize_structured_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn strip_leading_list_marker(text: &str) -> String {
    let trimmed = text.trim_start();
    let mut digit_end = 0usize;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() {
            digit_end = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    if digit_end > 0 {
        let remainder = &trimmed[digit_end..];
        if let Some(marker) = remainder.chars().next() {
            if matches!(marker, '.' | ')') {
                let after_marker = &remainder[marker.len_utf8()..];
                if after_marker.starts_with(char::is_whitespace) {
                    let rest = after_marker.trim_start();
                    if !rest.is_empty() {
                        return rest.to_string();
                    }
                }
            }
        }
    }

    trimmed.to_string()
}

fn looks_like_field_label(text: &str) -> bool {
    let trimmed = text.trim();
    if !trimmed.ends_with(':') {
        return false;
    }

    trimmed.len() <= 48 && trimmed.split_whitespace().count() <= 6 && !trimmed.contains('.')
}

pub(crate) fn looks_like_key_value_label(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 48
        && trimmed.split_whitespace().count() <= 6
        && trimmed.chars().any(|ch| ch.is_alphabetic())
        && (trimmed.ends_with(':')
            || trimmed.split_whitespace().count() >= 2
            || key_value_label_keyword_re().is_match(trimmed))
}

pub(crate) fn normalize_field_label(text: &str) -> String {
    strip_leading_list_marker(text)
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_string()
}

fn key_value_label_keyword_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\b(invoice|date|number|phone|email|address|total|due|customer|account)\b")
            .unwrap()
    })
}

fn extract_inline_field_value(text: &str) -> Option<&str> {
    let (label, value) = text.split_once(':')?;
    if !looks_like_field_label(&format!("{}:", label.trim())) {
        return None;
    }
    let value = value.trim();
    if value.is_empty() || value.len() > 60 || value.contains('.') {
        None
    } else {
        Some(value)
    }
}

fn looks_like_choice_line(text: &str) -> bool {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    (2..=8).contains(&tokens.len())
        && tokens.iter().all(|token| {
            !token.contains(':')
                && token.len() <= 18
                && token.chars().any(|ch| ch.is_alphanumeric())
                && !token.chars().any(|ch| matches!(ch, '.' | ','))
        })
}

fn looks_like_field_value(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && !looks_like_field_label(trimmed)
        && !looks_like_choice_line(trimmed)
        && trimmed.len() <= 48
        && trimmed.split_whitespace().count() <= 6
}

pub(crate) fn normalize_field_value(text: &str) -> String {
    strip_leading_list_marker(&normalize_structured_text(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_zero_shorthand_rows_to_dominant_width() {
        let row = ParsedNumericRow {
            label: "11) Variazioni rimanenze materie prime".to_string(),
            values: vec!["0000".to_string()],
        };
        let normalized = normalize_numeric_row(row, 4);
        assert_eq!(normalized.values, vec!["0", "0", "0", "0"]);
    }

    #[test]
    fn strip_leading_list_marker_removes_simple_number_prefixes() {
        assert_eq!(
            strip_leading_list_marker("1. Revenue 120 220"),
            "Revenue 120 220"
        );
        assert_eq!(
            strip_leading_list_marker("12) Accantonamenti per rischi000240.000"),
            "Accantonamenti per rischi000240.000"
        );
        assert_eq!(
            strip_leading_list_marker("17-bis) Utili e perdite"),
            "17-bis) Utili e perdite",
            "non-simple labels should be preserved"
        );
    }
}
