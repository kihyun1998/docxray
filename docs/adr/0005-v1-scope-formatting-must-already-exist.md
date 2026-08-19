# v1 accepts only edits whose formatting already exists in the Original

v1 supports replace, delete, move, restyle, and insert-with-Neighbour-Inheritance. The single thing cut is any edit that requires inventing formatting the Original does not contain.

The intuitive scope axis — "content edits are easy, structural edits are hard" — is wrong. Moving a Block carries its formatting along and is trivial; adding one is the only case with nothing to point at. Sorting by *where the formatting comes from* rather than *what the edit does* answers "is this in v1?" for every future request without further argument.

## Considered options

- **Reference docx** (a second document supplying styles, as pandoc's `--reference-doc` does) — a good idea, and the user's own, but dropped from v1. Importing a style drags in `basedOn` ancestor chains, `numbering.xml` definitions, `styleId` collisions and font table merging. With it gone, the Style Catalogue has exactly one source and v1 gets simpler. The Catalogue abstraction stays, so adding a second source later does not disturb the architecture.

## Consequences

- Tables are fully supported in v1; they are too central to real documents to defer.
- Input documents are assumed fully wild — files made in Korean Word, converted from HWP, exported from Google Docs, with tracked changes left on. Preserving unmodelled elements as Opaque Nodes is therefore a day-one requirement, not a later hardening pass.
- Sample docx files are still needed as test fixtures. That is a corpus, not a feature, and is unaffected by this scope decision.
