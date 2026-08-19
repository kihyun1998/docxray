# Fixture corpus

Real `.docx` files the test seam runs against (ADR-0008). Every file here must be
safe to publish.

Fixtures come in two kinds, because one file cannot be both minimal and
realistic:

- **Minimal fixtures** isolate one axis each, so a failing test points at one cause.
- **Wild fixtures** carry the mess a parser actually breaks on — revision-id
  fragments, meaningless style identifiers, unused styles, missing standard parts.

## `vendor/docx-rs/`

Fifteen fixtures taken from [docx-rs](https://github.com/bokuweb/docx-rs)'s test
corpus (MIT). Produced by real Word, LibreOffice and Word Online, so they are
ground truth in a way a document we generated ourselves could never be — see that
directory's README for the reasoning, the verified inventory, and the list of
what it does **not** cover.

## Ours

Nothing yet. What is still needed is exactly what the vendored corpus cannot
supply, and all of it requires a person with Word:

| Needed | Why nothing vendored covers it |
| ------ | ------------------------------ |
| A document authored in Korean Word | Nothing vendored pairs a Latin with a Hangul font in `w:rFonts`, or carries the meaningless style identifiers a localised Word produces |
| A document converted from HWP | Absent from every public corpus checked |
| A trimmed real-world document | The generator shape that omits `docProps` entirely — the one that produced the outline-level war story |
| Something long | Nothing vendored is more than a few pages, so nothing exercises `outline` at the scale that motivates it |

## Sanitising, before anything of ours is committed

This repository is public, and a `.docx` carries more than it shows on screen.

1. Turn tracked changes off and accept all changes **before** trimming — deleted
   text otherwise survives inside deletion markup and gets published.
2. Remove comments unless the fixture exists to exercise comments.
3. Clear author, company and other document properties.
4. Check the **unzipped XML**, not the rendered document.

Untrimmed originals live in `tests/fixtures-local/`, which is gitignored. Trim
and sanitise there; save the result here.

## The inventory

Every fixture is listed with what it exercises and where it came from — vendored
ones in `vendor/docx-rs/README.md`, ours in the table above once they exist.
State what a file actually contains, verified by unzipping it, rather than what
its name suggests.
