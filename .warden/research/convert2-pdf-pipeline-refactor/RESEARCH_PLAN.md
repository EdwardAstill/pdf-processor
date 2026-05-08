# Convert2 PDF Pipeline Refactor Research Plan

Date checked: 2026-05-05

## Question

What should `convert2` / `cnv` borrow from current PDF-to-Markdown tools and its own wiki before a behavior-preserving refactor?

## Deliverable

A sourced research report plus a small refactor that improves the local pipeline boundary without changing CLI behavior or Markdown output.

## Sub-questions

- What do current PDF-to-Markdown tools treat as separable pipeline stages?
- Where does the local wiki say `cnv`'s current bottlenecks live?
- Which refactor improves the repo's ability to evolve toward those lessons while preserving behavior?
- What tests prove the refactor did not change behavior?

## Source Families

- External official project docs and repositories.
- Recent paper evidence on PDF-to-RAG conversion quality.
- Repo-local wiki notes under `wiki/`.
- Warden refactoring guidance under `/home/eastill/projects/warden/wiki`.

## Refactor Constraint

No feature work in this pass. The code change should only move existing behavior into a clearer module boundary and keep tests green.
