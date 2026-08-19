# Fixture corpus

Committed fixtures live here. Every file must be safe to publish.

- **Minimal fixtures** isolate one axis each, so a failing test points at one cause.
- **Wild fixtures** are trimmed from real documents, so the corpus contains the mess a
  parser actually breaks on — revision-id fragments, meaningless style identifiers,
  unused styles, odd nesting.

Untrimmed originals belong in `tests/fixtures-local/`, which is gitignored. Trim and
sanitise there, then save the result here.

Before committing any wild fixture: accept all tracked changes **before** trimming,
remove comments unless the fixture exists to exercise them, clear author and company
properties, and check the unzipped XML rather than the rendered document.

See #3.

| File | What it exercises | Origin |
| ---- | ----------------- | ------ |
| _(none yet)_ | | |
