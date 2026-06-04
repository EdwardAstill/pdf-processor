pub(crate) fn clean_markdown(markdown: &str) -> String {
    let blocks = markdown_to_blocks(markdown);
    let mut out = String::new();
    let mut pending_paragraph = String::new();

    for block in blocks {
        if block.is_formula_review_comment() {
            continue;
        }

        if block.is_plain_paragraph() {
            let text = normalize_clean_text(&block.text);
            if text.is_empty() {
                continue;
            }
            if pending_paragraph.is_empty() {
                pending_paragraph = text;
            } else if should_merge_paragraphs(&pending_paragraph, &text) {
                if pending_paragraph.ends_with('-') {
                    pending_paragraph.pop();
                } else if !pending_paragraph.ends_with(' ') {
                    pending_paragraph.push(' ');
                }
                pending_paragraph.push_str(&text);
            } else {
                push_clean_block(&mut out, &pending_paragraph);
                pending_paragraph = text;
            }
        } else {
            if !pending_paragraph.is_empty() {
                push_clean_block(&mut out, &pending_paragraph);
                pending_paragraph.clear();
            }
            push_clean_block(&mut out, &block.cleaned_text());
        }
    }

    if !pending_paragraph.is_empty() {
        push_clean_block(&mut out, &pending_paragraph);
    }

    out
}

#[derive(Debug)]
struct MarkdownBlock {
    text: String,
    fenced: bool,
}

impl MarkdownBlock {
    fn is_formula_review_comment(&self) -> bool {
        self.text.trim_start().starts_with("<!-- formula-review:")
    }

    fn is_plain_paragraph(&self) -> bool {
        if self.fenced {
            return false;
        }
        let trimmed = self.text.trim_start();
        if trimmed.is_empty() {
            return false;
        }
        !is_structural_markdown_block(trimmed)
    }

    fn cleaned_text(&self) -> String {
        if self.fenced || self.text.trim_start().starts_with('|') {
            normalize_clean_glyphs(&self.text).trim().to_string()
        } else {
            normalize_clean_text(&self.text)
        }
    }
}

fn markdown_to_blocks(markdown: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut in_fence = false;
    let mut current_fenced = false;

    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if !in_fence && is_standalone_comment_line(trimmed) {
            if !current.is_empty() {
                blocks.push(MarkdownBlock {
                    text: current.join("\n"),
                    fenced: current_fenced,
                });
                current.clear();
                current_fenced = false;
            }
            blocks.push(MarkdownBlock {
                text: line.to_string(),
                fenced: false,
            });
            continue;
        }

        if trimmed.starts_with("```") {
            if !in_fence && !current.is_empty() {
                blocks.push(MarkdownBlock {
                    text: current.join("\n"),
                    fenced: current_fenced,
                });
                current.clear();
                current_fenced = false;
            }
            if current.is_empty() {
                current_fenced = true;
            }
            in_fence = !in_fence;
            current.push(line.to_string());
            if !in_fence {
                blocks.push(MarkdownBlock {
                    text: current.join("\n"),
                    fenced: current_fenced,
                });
                current.clear();
                current_fenced = false;
            }
            continue;
        }

        if !in_fence && line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(MarkdownBlock {
                    text: current.join("\n"),
                    fenced: current_fenced,
                });
                current.clear();
                current_fenced = false;
            }
            continue;
        }

        current.push(line.to_string());
    }

    if !current.is_empty() {
        blocks.push(MarkdownBlock {
            text: current.join("\n"),
            fenced: current_fenced,
        });
    }

    blocks
}

fn is_standalone_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("<!-- page:") || trimmed.starts_with("<!-- formula-review:")
}

fn is_structural_markdown_block(trimmed: &str) -> bool {
    trimmed.starts_with('#')
        || trimmed.starts_with("<!--")
        || trimmed.starts_with('|')
        || trimmed.starts_with("![")
        || trimmed.starts_with("$$")
        || trimmed.starts_with("- ")
        || trimmed.starts_with("*Figure")
        || trimmed.starts_with("*") && trimmed.ends_with("*")
        || starts_with_ordered_list_marker(trimmed)
}

fn starts_with_ordered_list_marker(trimmed: &str) -> bool {
    let Some((number, rest)) = trimmed.split_once('.') else {
        return false;
    };
    !number.is_empty() && number.chars().all(|ch| ch.is_ascii_digit()) && rest.starts_with(' ')
}

fn normalize_clean_text(text: &str) -> String {
    normalize_clean_glyphs(text)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_clean_glyphs(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\u{00a0}' => normalized.push(' '),
            '\u{00ad}' | '\u{200b}' | '\u{200c}' | '\u{200d}' => {}
            '\u{fb00}' => normalized.push_str("ff"),
            '\u{fb01}' => normalized.push_str("fi"),
            '\u{fb02}' => normalized.push_str("fl"),
            '\u{fb03}' => normalized.push_str("ffi"),
            '\u{fb04}' => normalized.push_str("ffl"),
            '\u{f0b7}' | '•' => normalized.push_str("- "),
            '�' => {}
            _ => normalized.push(ch),
        }
    }
    normalized
}

fn should_merge_paragraphs(previous: &str, current: &str) -> bool {
    let previous = previous.trim_end();
    let current = current.trim_start();
    if previous.is_empty() || current.is_empty() {
        return false;
    }
    if previous.ends_with('-') {
        return true;
    }
    if current
        .chars()
        .next()
        .is_some_and(|ch| ch.is_lowercase() || matches!(ch, ',' | ';' | ':' | ')' | ']'))
    {
        return true;
    }
    !ends_like_paragraph(previous)
}

fn ends_like_paragraph(text: &str) -> bool {
    text.chars()
        .rev()
        .find(|ch| !ch.is_whitespace() && !matches!(ch, '"' | '\'' | ')' | ']'))
        .is_some_and(|ch| matches!(ch, '.' | '!' | '?' | ':'))
}

fn push_clean_block(out: &mut String, block: &str) {
    let trimmed = block.trim();
    if trimmed.is_empty() {
        return;
    }
    out.push_str(trimmed);
    out.push_str("\n\n");
}
