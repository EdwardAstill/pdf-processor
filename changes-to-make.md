# Changes to make

Identified from formula-fixing pass on 10 literature markdowns (thesis project, May 2026).
All four changes are in `src/pipeline.rs` and `src/pdf/text_cleanup.rs`.

---

## A. Strip equation number from source text and inject `\tag{N}`

**File:** `src/pipeline.rs` — `formula_candidates_to_blocks` (~line 353)

`extract_equation_number()` correctly detects trailing `(3)` at the end of a formula line and stores it in `candidate.equation_number`, but it is never stripped from `source_text` or moved into the LaTeX output. The number ends up duplicated — once in the formula body, once dangling.

**Fix:** when constructing the LaTeX string from `source_text`, if `candidate.equation_number` is `Some(n)`, strip the trailing number from the text and append `\\tag{N}` (stripping the outer parens from `n`).

```rust
let latex = candidate.latex.clone().unwrap_or_else(|| {
    let mut text = candidate.source_text.clone();
    if let Some(ref eq_num) = candidate.equation_number {
        // Remove trailing equation number from body, e.g. "(3)" or "(1.2)"
        if let Some(stripped) = text.trim_end().strip_suffix(eq_num.as_str()) {
            let tag_inner = eq_num.trim_matches(|c| c == '(' || c == ')');
            text = format!("{} \\tag{{{}}}", stripped.trim_end(), tag_inner);
        }
    }
    text
});
```

---

## B. Unicode → LaTeX normalisation on source text

**File:** `src/pipeline.rs` — `formula_candidates_to_blocks` (~line 353), applied after the equation-number strip in change A

When `candidate.latex` is `None`, the fallback `source_text` is raw PDF-extracted Unicode. It contains Greek letters (σ, Δ, …), math operators (∑, ∫, ∂), and the sqrt glyph — none of which are valid LaTeX commands. A small lookup pass before emitting the `$$` block fixes this.

**Fix:** add a `unicode_to_latex(s: &str) -> String` helper and call it on `source_text` when `candidate.latex` is `None`.

Minimum viable table (extend as needed):

```rust
fn unicode_to_latex(s: &str) -> String {
    s.chars().fold(String::with_capacity(s.len() + 16), |mut out, c| {
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
            '√' => out.push_str("\\sqrt{} "),  // see note below
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
            '−' => out.push('-'),  // math minus → ASCII minus
            _ => out.push(c),
        }
        out
    })
}
```

> **Note on `√`:** After change C strips the combining overbars, a bare `√` means `\sqrt{}` with an unknown argument (the argument was encoded as the text under the vinculum). Emitting `\sqrt{}` is a valid placeholder — better than garbled output. A smarter pass could try to identify the next token group as the argument, but that's a bigger change.

---

## C. Strip combining overline (U+0305) in text_cleanup

**File:** `src/pdf/text_cleanup.rs` — `cleanup_extracted_text`

The PDF Unicode encoding of `\sqrt{x}` uses a `√` glyph (U+221A) followed by one U+0305 (combining overline) per character under the vinculum. These combining chars pass through the current cleanup untouched, producing `√̅̅̅̅̅̅̅̅̅` in the output — completely unrenderable and confusing.

**Fix:** add U+0305 to the strip list in the `match` block.

```rust
| '\u{0305}' // combining overline — used by PDF sqrt vinculum encoding; strip here,
             // the √ glyph itself is handled by unicode_to_latex in the formula path
```

No other combining diacritics need stripping — U+0305 specifically is a PDF math artefact, not real text.

---

## D. `auto` mode should emit `$$` blocks for high-confidence candidates

**File:** `src/pipeline.rs` — `formula_candidates_to_blocks` (~line 342)

Currently `auto` mode (the default) detects formula candidates but emits nothing — the regions fall through to regular text extraction and come out as raw inline text without delimiters. Most users never pass `--formulas local`, so the math ends up unformatted in every `auto`-mode conversion.

**Fix:** change the mode guard so high-confidence candidates (≥ 70) are emitted as `$$` blocks in `auto` mode too. Low-confidence candidates (35–69) can emit a fenced comment for review.

```rust
fn formula_candidates_to_blocks(
    page_num: usize,
    candidates: Vec<FormulaCandidate>,
    mode: cli::FormulaMode,
) -> Vec<Block> {
    if matches!(mode, cli::FormulaMode::Off) {
        return Vec::new();
    }

    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(idx, candidate)| {
            // In auto mode only promote high-confidence candidates.
            if matches!(mode, cli::FormulaMode::Auto)
                && candidate.confidence < 70
            {
                return None;
            }

            let latex = build_latex(&candidate);  // changes A + B live here
            Some(Block {
                id: 3_000_000 + idx,
                bbox: candidate.bbox,
                text: candidate.source_text,
                kind: BlockKind::Formula {
                    latex,
                    display: true,
                },
                font_size: 0.0,
                font_name: "formula-candidate".to_string(),
                page_num,
                reading_order: 0,
            })
        })
        .collect()
}
```

> **Caution:** promoting `auto` to emit blocks means the `source_text` suppression logic (which currently prevents formula candidate text from being double-rendered by the regular text path) must be verified to cover `auto` mode too — check `suppress_text_covered_by_*` call sites.
