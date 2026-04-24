# Spec — Surface encrypted-PDF errors with actionable guidance

Date: 2026-04-24

## Goal

When `cnv` encounters a password-protected (encrypted) PDF, the user-facing error must clearly state that the PDF is encrypted and point the user at an actionable next step (e.g. decrypt with `qpdf` before rerunning). Today the error is indistinguishable from any other extraction failure, so users can't tell whether to retry, file a bug, or preprocess the file.

## Why

Observed on `DNV-RP-C201_2023 - Buckling of plated structures.pdf` (known encrypted):

```
error: pdfs/DNV-RP-C201_2023 - Buckling of plated structures.pdf:
       Failed to extract pdfs/DNV-RP-C201_2023 - Buckling of plated structures.pdf
```

The underlying error enum already has a dedicated variant (`src/error.rs:32`):

```rust
#[error("PDF is password-protected, cannot process: {0}")]
PasswordProtected(PathBuf),
```

But in `src/main.rs:81` every extraction failure is wrapped uniformly:

```rust
.with_context(|| format!("Failed to extract {}", pdf_path.display()))?;
```

`anyhow`'s context wrapping hides the original error kind from the top-level printer. The specific `PasswordProtected` message never reaches the user because the generic context sits on top.

## Scope

In scope:
- Detect encrypted PDFs before or during extraction and surface the `PasswordProtected` error variant distinctly.
- Emit a user-facing message that:
  - Names the file.
  - States it is password-protected / encrypted.
  - Suggests one or more concrete remediation paths (e.g. `qpdf --decrypt in.pdf out.pdf`, or "supply the password via <mechanism> if you have one" — only if a password flag is added).
- Exit the affected file's processing cleanly (continue with the rest of the batch) rather than aborting the whole run.

Out of scope:
- Adding a `--password` flag for user-supplied decryption. Punted — design stub in "Open questions".
- Attempting common/empty passwords automatically.
- Any OCR or hybrid fallback on encrypted PDFs.

## Design

### Detection

Encryption is detectable before full extraction via the PDF header / trailer dictionary. Two viable paths:

1. **Native MuPDF check** — `mupdf` exposes whether a document requires a password (`fz_needs_password` / `pdf_needs_password` in the C API; the Rust binding likely surfaces equivalent). Preferred — no extra dependency.
2. **Pre-flight open** — attempt to open the document; map the resulting error kind to `PasswordProtected` when the underlying error indicates authentication required.

Implementation should prefer path (1) and fall back to error-kind matching in path (2) for completeness.

### Error emission

1. Stop wrapping every extraction error with the same anyhow context at `src/main.rs:81`. Instead, match on the error kind first and route the password-protected case to a dedicated branch.
2. The dedicated branch emits a formatted, multi-line warning (not a hard error for batch runs) like:

   ```
   skipped: pdfs/DNV-RP-C201_2023 - Buckling of plated structures.pdf
            reason: PDF is password-protected / encrypted.
            try: qpdf --decrypt in.pdf out.pdf   # if you have the password
                 or remove the file from the batch
   ```

3. Keep the original error chain available under `-v` / `--verbose` for debuggability; the user-facing summary stays concise by default.
4. Continue processing remaining inputs. Final run summary should list encrypted files in a dedicated "skipped (encrypted)" section so they do not get lost in `ok:` lines.

### Exit code

- Single-file run on an encrypted PDF: exit code `2` (distinct from general failure `1`) so shell scripts can treat it as a recoverable case.
- Batch run with at least one encrypted PDF and at least one success: exit code `0` (honor the successes) with the summary listing the skip. This matches the existing tolerant-batch behavior.
- Batch run where every file failed encryption: exit code `2`.

### Message guidelines

- Always name the file first.
- Use the word **encrypted** (common vocabulary) and **password-protected** (matches the error enum wording) in the same sentence.
- Give exactly one suggested remediation command; users with different tooling can adapt.
- Do not suggest the user "contact support" — this is a known, well-defined condition.

## Acceptance Criteria

1. Running `cnv <encrypted.pdf>` produces a message that explicitly calls out the PDF as encrypted/password-protected.
2. The message includes a concrete next-step command (`qpdf --decrypt ...`).
3. A batch containing a mix of normal and encrypted PDFs processes the normal ones, skips the encrypted ones with a distinct message, and the final summary lists them in a dedicated "skipped (encrypted)" section.
4. Exit code behavior matches the table in Design.
5. `cargo test` remains green; new tests cover the encrypted-PDF path.

## Test Plan

- **Unit:** add a test that constructs a `PasswordProtected` error and asserts the formatted output matches the expected pattern (file name, "encrypted" substring, suggested command).
- **Integration:** add a fixture PDF that is encrypted with a known password (e.g. an empty 1-page doc encrypted via `qpdf --encrypt "" "" 40 --`). Store under the existing ignored `test-corpus/` or a new `tests/fixtures/encrypted/` path. Test asserts:
  - Running `cnv <fixture>` exits `2` and prints the expected message.
  - Running `cnv <dir>` containing one good PDF + the encrypted fixture exits `0`, processes the good one, and reports the encrypted one in the skipped section.
- **Regression:** existing error paths (malformed PDF, missing file) continue to emit their current messages; confirm with one sanity test each.

## Risks

- **Detection false negatives:** if the MuPDF Rust binding doesn't expose the needs-password check cleanly, error-kind matching may miss edge cases (malformed encrypted PDF that fails before the encryption check). Mitigation: fall-through to the generic error path still works — the user just doesn't get the friendly message for that one case.
- **Fixture-committing risk:** shipping a deliberately encrypted PDF in the repo could make some dev workflows awkward. Mitigation: generate the fixture in a test setup script rather than committing the PDF.

## Open questions

- Should a `--password` flag be added to let users decrypt on the fly? Out of scope here. Sketch: `--password <PASSWORD>` plus `--password-file <path>` for scripts; apply to every input in the batch. File a separate spec if prioritized.
- Should the summary indicate `N skipped (encrypted)` on stdout even when verbose is off? Likely yes — single line at the end of the run.
