# Wiki Conventions

Frontmatter, voice, and style rules for wiki pages in the `pdf-processor` project wiki.

## Required Frontmatter

Every wiki page must have YAML frontmatter with these fields:

```yaml
---
title: "<descriptive title>"
kind: "<reference | knowledge | index | roadmap>"
category: "wiki"
summary: "<one-paragraph summary of what this page covers>"
entities: [key entities or tools discussed — empty list ok]
---
```

### Field meanings

| Field | Values | Notes |
|---|---|---|
| `title` | string | Descriptive page title in Title Case |
| `kind` | `reference`, `knowledge`, `index`, `roadmap` | `reference` = comparison/catalogue of tools, algorithms, projects. `knowledge` = conceptual explanation of a topic. `index` = MOC / area overview. `roadmap` = prioritised improvement list. |
| `category` | always `"wiki"` | Used for tool-based filtering |
| `summary` | string | One paragraph. No markdown formatting. |
| `entities` | YAML list of strings | Tools, libraries, algorithms, projects, or standards referenced on the page. Empty list `[]` is allowed. |

### Optional fields

| Field | When to use |
|---|---|
| `id` | Only on index pages (`kind: index`) — use `<area>-index`, e.g. `algorithms-index`, `tools-index` |
| `updated` | ISO date (`2026-05-05`) when a page gets significant revision |

## Voice

- Technical, direct, no filler
- Prefer concrete failure modes and design decisions over vague summaries
- Use the project name `pdfp` consistently — not `cnv` (old name)
- File paths and CLI flags in backticks

## Section Conventions

- Start with **why** the topic matters for `pdfp`, then explain the details
- Use `---` to separate major sections
- End with a **Related pages** or **See also** section with relative markdown links
- Use relative paths for internal wiki cross-links: `[Page Title](path/to/page.md)`
- Use `../` to escape up to the area directory when linking across areas

## Linking

- Within the same area: `[Page Title](page-name.md)`
- Across areas: `[Page Title](../area-name/page-name.md)`
- Out of wiki to repo: `[README](../README.md)` (relative from wiki root)
- Never use `[[wikilinks]]` — this wiki uses standard markdown relative links
