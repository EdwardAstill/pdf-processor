# PDF Metadata Read/Write Feature Plan

Date: 2026-05-19
Status: done
Branch/worktree: `pdf-metadata-rw-plan` at `.worktrees/pdf-metadata-rw-plan`
Research: `.warden/research/pdf-metadata-rw/REPORT.md`

## Goal

Add first-class PDF metadata read/write capability to `pdfp`, with a documented
CLI, machine-readable JSON, safety behavior consistent with existing PDF
operations, and tests that prove metadata changes round-trip without damaging
page content.

## Non-Goals

- No in-place write mode in the first implementation.
- No XMP packet editing in the first implementation.
- No signature-preserving metadata edits.
- No PDF/A conformance validation.
- No promise that `pages`, `impose`, or `page resize` preserve metadata until
  the explicit preservation phase lands.

## Assumptions

| id | statement | type | source | check | if false | owner |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | `lopdf` 0.40 can load, mutate, and save the repo's test PDFs without breaking page count or embedded text search. | external | research report + docs.rs | `cargo test --test metadata` with fixture round-trips | Switch writer implementation to MuPDF low-level Info dictionary writes. | Task 3 |
| A2 | Project can accept `lopdf` 0.40's Rust 1.85 floor because local/project toolchain is newer. | repo-state | `rustc 1.94.0` observed locally | `cargo check` after adding dependency | Pin older `lopdf` or use MuPDF-only path. | Task 2 |
| A3 | MVP should write Info dictionary metadata only and warn about XMP divergence. | design | research report | XMP fixture test produces warning and succeeds | Add XMP update scope before release. | Task 4 |
| A4 | Existing safety convention forbids output path equal to input path. | policy | current page operations | metadata set/clear same-path test fails with clear error | Add explicit in-place confirmation gate, but only after user approval. | Task 4 |
| A5 | Existing docs should describe both the new capability and the remaining limits. | design | user request | `rg "metadata" README.md docs/CLI.md docs/TESTING.md` shows new docs | Add separate docs-only task before merge. | Task 8 |

## Design Decision

Constraints:

`pdfp` is a local-first Rust CLI. Existing PDF-producing commands write new
files and refuse to overwrite inputs. Current metadata reading uses MuPDF and is
limited to title/author/subject. Users need a simple command to inspect,
change, and clear metadata fields without learning PDF internals. The first
safe version should avoid incremental edits because old metadata can remain
recoverable, and it should avoid XMP editing until the repo has fixtures and
clear semantics for Info/XMP synchronization.

Alternatives:

| Option | Description | Why choose it |
| --- | --- | --- |
| MuPDF low-level mutation | Open `PdfDocument`, mutate trailer `/Info`, save with MuPDF. | Keeps one PDF engine and may preserve more MuPDF-specific behavior. |
| `lopdf` Info writer | Add `lopdf`, mutate Info dictionary as PDF objects, save full rewrite. | Safest MVP: pure Rust, proper text-string helpers, no unsafe wrapper changes. |
| External CLI backend | Shell out to ExifTool/qpdf. | Fast to prototype but violates single-binary/local dependency model. |

Trade-off table:

| Criterion | MuPDF low-level | `lopdf` Info writer | External CLI |
| --- | --- | --- | --- |
| Fits current stack | High | Medium | Low |
| Text-string encoding confidence | Medium | High | High |
| Unsafe/FFI risk | Medium | Low | Low in Rust, high operationally |
| Dependency footprint | Low | Medium | High runtime dependency |
| Full rewrite privacy posture | High | High | Mixed, depends on tool/options |
| XMP support path | Medium | Medium | High |
| Testability | Medium | High | Medium |

Recommendation:

Use `lopdf` for the MVP metadata command family. Keep MuPDF as the existing
conversion/search/page engine. This gives a narrow, testable feature and avoids
new unsafe bindings. Revisit MuPDF-only implementation only if `lopdf` fails on
repo fixtures.

## CLI Contract

Add top-level subcommand:

```sh
pdfp metadata <show|set|clear> ...
```

### `metadata show`

```sh
pdfp metadata show <INPUT> [--json] [--verbose]
```

Human stdout:

```text
source: input.pdf
pages: 12
version: 1.7
title: ...
author: ...
subject: ...
keywords: ...
creator: ...
producer: ...
created: D:20240501083000+08'00'
modified: D:20240502093000+08'00'
xmp: present
```

JSON stdout:

```json
{
  "source": "input.pdf",
  "page_count": 12,
  "version": "1.7",
  "info": {
    "title": null,
    "author": null,
    "subject": null,
    "keywords": null,
    "creator": null,
    "producer": null,
    "creation_date": null,
    "modification_date": null
  },
  "xmp": {
    "present": false
  },
  "signatures": {
    "present": false
  }
}
```

### `metadata set`

```sh
pdfp metadata set <INPUT> -o <OUTPUT> \
  [--title <TEXT>] \
  [--author <TEXT>] \
  [--subject <TEXT>] \
  [--keywords <TEXT>] \
  [--creator <TEXT>] \
  [--producer <TEXT>] \
  [--creation-date <DATE>] \
  [--mod-date <DATE|now>] \
  [--no-touch-mod-date] \
  [--force-signed] \
  [--json] \
  [--verbose]
```

Behavior:

- Refuse when no metadata field is provided.
- Refuse when output path resolves to input path.
- Write a new PDF at `OUTPUT`.
- Preserve unspecified Info fields.
- Set `ModDate` to current local time unless:
  - user provided `--mod-date`, or
  - user passed `--no-touch-mod-date`.
- Warn on stderr when XMP metadata is present and not updated.
- Refuse signed PDFs unless `--force-signed` is supplied.

JSON stdout on success:

```json
{
  "input": "input.pdf",
  "output": "output.pdf",
  "changed": ["title", "author", "modification_date"],
  "cleared": [],
  "warnings": ["xmp_present_not_updated"]
}
```

### `metadata clear`

```sh
pdfp metadata clear <INPUT> -o <OUTPUT> \
  --fields <FIELD[,FIELD...]> \
  [--force-signed] \
  [--json] \
  [--verbose]
```

Accepted field names:

- `title`
- `author`
- `subject`
- `keywords`
- `creator`
- `producer`
- `creation-date`
- `mod-date`
- `all`

`all` clears user-facing descriptive fields:

- `title`
- `author`
- `subject`
- `keywords`
- `creator`

It does not clear `producer`, `creation-date`, or `mod-date` unless those fields
are explicitly named.

## Implementation Shape

New module:

- `src/processor/metadata.rs`

CLI edits:

- `src/cli.rs`
  - Add `Command::Metadata(MetadataCommand)`.
  - Add `MetadataSubcommand::{Show, Set, Clear}`.
  - Add reusable `MetadataField` enum with `ValueEnum`.
  - Add validation helper for at-least-one set field.
- `src/commands.rs`
  - Route `AppCommand::Metadata(args)` to `processor::metadata::run(&args)`.
- `src/processor/mod.rs`
  - Export `metadata`.

Dependency:

- `Cargo.toml`
  - Add `lopdf = { version = "0.40", default-features = false }` if compile
    confirms metadata load/save works with no default features.
  - If default features are required for date helpers, use
    `lopdf = { version = "0.40", default-features = false, features = ["time"] }`.

Internal types:

```rust
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PdfInfoMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetadataReport {
    pub source: String,
    pub page_count: u32,
    pub version: String,
    pub info: PdfInfoMetadata,
    pub xmp: XmpReport,
    pub signatures: SignatureReport,
}
```

Write helper behavior:

1. `load_metadata_report(path)` uses `lopdf::Document::load_metadata(path)` for
   Info/page/version fields and a full `Document::load(path)` only when XMP or
   signature detection is requested.
2. `apply_metadata_changes(input, output, changes)`:
   - loads `lopdf::Document`;
   - locates or creates trailer `/Info`;
   - writes values with `lopdf::text_string()`;
   - removes fields for `clear`;
   - creates parent output directory;
   - saves with `Document::save(output)`;
   - re-opens output and verifies changed fields.
3. `ensure_output_is_not_input(input, output)` should be shared with
   `processor::pages` or duplicated only briefly then refactored in the same
   patch.

Info dictionary creation algorithm:

```text
if trailer["Info"] is an indirect reference:
    dereference and mutate that dictionary
else if trailer["Info"] is a direct dictionary:
    mutate direct dictionary in trailer
else:
    create a new dictionary object with doc.add_object(Dictionary::new())
    set trailer["Info"] to that object reference
```

Date handling:

- Accept raw PDF date strings beginning with `D:` only if they match a strict
  supported subset.
- Accept RFC3339 strings and convert to PDF date format.
- Accept `now` for `--mod-date`.
- Prefer AWST/local offset only for current-time conversion; do not guess a
  timezone for user-supplied date-only values.

Signature/XMP detection:

- XMP present if catalog `/Metadata` exists.
- Signature present if:
  - `/Root/AcroForm/Fields` contains field dictionaries with `/FT /Sig`, or
  - `/Root/Perms` contains `/DocMDP` or related signature permissions.

## Documentation Scope

Update `README.md`:

- Usage synopsis:
  - add `pdfp metadata <show|set|clear> ...`.
- Scope:
  - add metadata read/write to current top-level scope.
- New `### Metadata` section after Inspect/Search:
  - show metadata as human and JSON;
  - set title/author/etc. to a new PDF;
  - clear fields;
  - explain Info-only MVP and XMP warning.
- Output/safety note:
  - clarify metadata commands write new PDFs and refuse in-place output.

Update `docs/CLI.md`:

- Mental model table:
  - add Metadata row.
- Help command list:
  - add `pdfp metadata --help`, `metadata show`, `metadata set`,
    `metadata clear`.
- New `## Metadata` section:
  - CLI examples;
  - field list;
  - JSON schema;
  - date accepted formats;
  - signed/XMP/linearization caveats.
- Safety and Limits:
  - explain Info-only write support;
  - XMP not updated in MVP;
  - signed PDFs require `--force-signed`;
  - metadata writes may remove linearization because output is rewritten.

Update `docs/TESTING.md`:

- Add metadata test group:
  - show JSON schema;
  - set round-trip;
  - clear fields;
  - non-ASCII metadata;
  - same-path refusal;
  - XMP warning fixture;
  - signed-PDF refusal fixture if available.

Update CLI help smoke test:

- Add `metadata`, `metadata show`, `metadata set`, and `metadata clear` to
  `tests/cli_help.rs`.

## Task Graph

### Task 1: Confirm Writer Library Spike

Block: research
Skill: `library-docs`
Instruction:

Verify `lopdf` 0.40 with `default-features = false` can compile a minimal
metadata load/save path. If not, determine the smallest feature set needed.

Acceptance:

- Research note added to `.warden/research/pdf-metadata-rw/REPORT.md`.
- `cargo check` after temporary dependency edit identifies required feature set.

### Task 2: Add CLI Contract Types

Block: execute
Skill: `warden:rust`
Instruction:

Add clap command types for `pdfp metadata show|set|clear`, wire them into
`AppCommand`, and update command dispatch with a stub processor that returns
`unimplemented!()` only in paths not reached by help tests.

Acceptance:

- `cargo test --test cli_help` -> exits 0 and includes metadata command paths.
- `cargo run --quiet -- metadata --help` -> exits 0 and lists show/set/clear.
- `cargo run --quiet -- metadata set --help` -> exits 0 and shows metadata flags.

### Task 3: Implement Metadata Read Reports

Block: execute
Skill: `warden:rust`
Instruction:

Implement `metadata show` with human and JSON output, including full Info fields,
page count, PDF version, XMP presence, and signature presence.

Acceptance:

- `cargo test --test metadata metadata_show_json_reports_full_info` -> exits 0.
- `cargo run --quiet -- metadata show tests/fixtures/metadata-basic.pdf --json | jq '.info.title'` -> prints expected title.
- `cargo run --quiet -- metadata show tests/fixtures/metadata-basic.pdf` -> stdout includes `title:` and `pages:`.

### Task 4: Implement Metadata Set/Clear Writes

Block: execute
Skill: `warden:rust`
Instruction:

Implement Info dictionary mutation using `lopdf`, full rewrite to `-o`, same-path
refusal, parent directory creation, signed-PDF refusal, XMP warning, mod-date
touch behavior, and post-write verification.

Acceptance:

- `cargo test --test metadata metadata_set_round_trips_title_author_keywords` -> exits 0.
- `cargo test --test metadata metadata_clear_removes_selected_fields` -> exits 0.
- `cargo test --test metadata metadata_refuses_same_input_output` -> exits 0.
- `cargo test --test metadata metadata_warns_when_xmp_present` -> exits 0.
- `cargo test --test metadata metadata_refuses_signed_pdf_without_force` -> exits 0, or test is marked ignored with fixture note if no signed fixture exists.

### Task 5: Add Non-ASCII and Date Coverage

Block: execute
Skill: `warden:rust`
Instruction:

Add fixtures/tests for non-ASCII strings and date input handling. Validate
strict PDF date strings, `now`, and RFC3339 conversion if implemented.

Acceptance:

- `cargo test --test metadata metadata_set_preserves_unicode_title` -> exits 0.
- `cargo test --test metadata metadata_rejects_invalid_pdf_date` -> exits 0.
- `cargo test --test metadata metadata_mod_date_now_is_written` -> exits 0.

### Task 6: Integrate Metadata Preservation With Page Operations

Block: execute
Skill: `warden:rust`
Instruction:

Decide whether extract/delete/reorder/merge should preserve source metadata now
that metadata read/write support exists. Implement the minimum safe preservation
for copied-page outputs if feasible; otherwise leave an explicit documented gap.

Acceptance if implemented:

- `cargo test --test metadata pages_extract_preserves_info_metadata` -> exits 0.
- `cargo test --test metadata pages_merge_uses_first_input_info_metadata` -> exits 0.

Acceptance if deferred:

- `rg -n "metadata.*not.*guaranteed|metadata preservation" README.md docs/CLI.md docs/TESTING.md` -> exits 0 and states the remaining gap.

### Task 7: Review Error and Stream Contracts

Block: review
Skill: `warden:reviewer`
Instruction:

Review metadata CLI behavior for stdout/stderr separation, JSON stability,
same-path safety, signed/XMP warnings, and date validation.

Acceptance:

- Findings either fixed or recorded as follow-up before docs finalization.

### Task 8: Update Documentation

Block: execute
Skill: `warden:writing`
Instruction:

Update README and CLI/testing docs with the metadata command family, examples,
JSON schema, field list, date behavior, and explicit MVP limits.

Acceptance:

- `rg -n "pdfp metadata" README.md docs/CLI.md docs/TESTING.md` -> exits 0.
- `rg -n "XMP|Info dictionary|signed" docs/CLI.md` -> exits 0.
- `cargo test --test cli_help` -> exits 0 after help-command list update.

### Task 9: Final Verification

Block: review
Skill: `warden:verification-before-completion`
Instruction:

Run focused metadata tests, full test suite, formatting, and linting with the
repo's existing feature matrix.

Acceptance:

- `cargo fmt --check` -> exits 0.
- `cargo test --test metadata` -> exits 0.
- `cargo test --test cli_help` -> exits 0.
- `cargo test` -> exits 0.
- `cargo clippy --all-targets --features "pdfium-metadata onnx-ocr" -- -D warnings` -> exits 0.
- `git diff --check` -> exits 0.

## Fixture Plan

Create generated metadata fixtures under `tests/fixtures/metadata/` or a new
ignored/generated fixture script if binary PDFs should not be tracked.

Required fixtures:

- `metadata-basic.pdf`
  - Info fields populated.
- `metadata-empty.pdf`
  - no Info dictionary.
- `metadata-unicode.pdf`
  - non-ASCII title/author.
- `metadata-xmp.pdf`
  - Info + `/Root/Metadata` XMP stream.
- `metadata-signed.pdf`
  - signed sample, optional/ignored if too hard to generate locally.

Preferred generation approach:

- Use a small Rust test helper with `lopdf` to generate minimal PDFs in temp
  dirs during tests.
- Avoid committing opaque binary fixtures unless they are needed for XMP/signed
  cases.

## Expected User Documentation Examples

```sh
# Show metadata
pdfp metadata show report.pdf

# Show metadata as JSON
pdfp metadata show report.pdf --json

# Write updated metadata to a new PDF
pdfp metadata set report.pdf -o report-tagged.pdf \
  --title "Mooring Analysis Report" \
  --author "PySeas Engineering" \
  --keywords "mooring, analysis, DNV"

# Clear selected fields
pdfp metadata clear report.pdf -o report-clean.pdf --fields author,keywords
```

## Rollout Order

1. Implement read-only `metadata show`.
2. Implement set/clear writes for unencrypted, unsigned PDFs.
3. Add XMP/signed detection warnings.
4. Update docs.
5. Decide page-operation preservation as a separate small patch if MVP is stable.

## Deferred Follow-Ups

- `pdfp metadata xmp show` to print or extract raw XMP.
- `pdfp metadata sync-xmp` to mirror Info fields into XMP.
- `pdfp metadata copy --from source.pdf target.pdf -o output.pdf`.
- Metadata preservation for `ocr`, `pages`, `impose`, and `page resize`.
- `--linearize` option if qpdf or MuPDF linearization becomes supported.
- Password-protected PDF metadata reads/writes.
