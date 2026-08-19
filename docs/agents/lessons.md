# Lessons

Concrete occasions a rule caught something real. A rule with no war story here is
still an abstraction. Referenced by `docs/agents/theflow.md`.

---

## Outline level is not enough to detect a heading

**2026-08-19, while inspecting the first real fixture — before #4 or #5 existed.**

`docs/agents/theflow.md` and the v1 spec both named `w:outlineLvl` the most
reliable heading signal, reasoning from documents saved by a localised Word: the
style *name* is translated, the style *id* is often meaningless, so the outline
level is what survives.

The first real document we opened — a Korean product manual, roughly 40 MB, 662
paragraphs, 285 images — carries **no `w:outlineLvl` anywhere**. Not on
`Heading1` through `Heading6`, not in the document body. It had been produced by
a generator rather than saved by Word: no `docProps`, `[Content_Types].xml` last
in the archive instead of first, a minimal `settings.xml` with no revision table.
Its style ids, meanwhile, were clean English `Heading1`…`Heading6`.

Had we implemented the stated rule, every heading in that document would have
rendered as body text, and the failure would have looked like a parser bug rather
than a wrong premise.

**What it changed.** Heading detection is a cascade, not a signal: outline level,
then a known built-in style id, then a normalised style name, then the `basedOn`
ancestor chain, then heuristics.

**The rule it earned.** *"Wild" is not one thing.* Word-edited documents and
generator output are strange in opposite directions — the first has revision
fragments and localised style names, the second is missing standard parts and
attributes entirely. A corpus assembled from one kind proves nothing about the
other, so fixtures are counted in *kinds of wild*, not in files.

**The second thing it showed.** The same document's runs turned out to be only 1%
mergeable once revision identifiers were ignored — its fragmentation was genuine
formatting, not the spurious splitting we had recorded as a certainty. That
gotcha is real, but it belongs to documents with editing history, and this
corpus cannot demonstrate it.

---

## A gate that cannot fail is not a gate — the wasm check did not enforce ADR-0002

**2026-08-19, #2.**

`docs/agents/theflow.md` had committed the gate matrix a few hours earlier, and
its wasm line carried a confident rationale: `cargo test --workspace` builds for
the host, so `cargo check -p docxray --target wasm32-unknown-unknown` was what
kept a `std::fs` from creeping into the IO-agnostic core (ADR-0002). The
reasoning was clean and the claim was false.

Probed it before trusting it — added a function to the core taking a
`std::path::Path` and calling `std::fs::read`, then ran the gate:

```
=== wasm check (should go red) ===
    Finished `dev` profile ... exit=0
```

`wasm32-unknown-unknown` ships a `std` where `std::fs` **compiles**; the
operations fail at runtime with `Unsupported`. So the gate we had written down as
the guardian of the project's central architectural promise would have passed a
core that read files.

**What it changed.** `scripts/check-core-io-free.sh` scans the core for
`std::fs`, `std::path`, `std::net`, `std::env`, `std::process` and `PathBuf`,
ignoring comments, and fails the build when any appear. Verified with the same
probe: red with it, green without. The wasm check stays in the matrix, with its
rationale corrected — it catches dependencies that genuinely cannot build for
wasm, which is a real failure, just not the one it was credited with.

**The rule it earned.** *Probe a new gate the way you probe a new test.* The
test-trust gate already says a test you never saw fail proves nothing; a gate is
a test of the codebase and inherits the rule. The failure mode is worse for
gates, because a green gate is read as evidence by everyone downstream and
nobody re-derives the reasoning behind it. Cost of checking: one probe. Cost of
not checking: an architectural promise that erodes invisibly until the first
WASM build, months later.

---

## The main document part is not always `word/document.xml`

**2026-08-19, #3.**

A fixture-integrity gate was added to catch truncated downloads — one fixture had
already arrived as a 14-byte 404 body while looking like a successful fetch. The
check was: is it a readable zip, and does it contain `word/document.xml`?

It failed on its first run, against `indent_word_online.docx`:

```
no word/document.xml: tests/fixtures/vendor/docx-rs/indent_word_online.docx
```

The file was fine. **The gate was wrong.** That package's main part is
`word/document2.xml`. The part name is not fixed by the specification — it is
resolved by following the `officeDocument` relationship in `_rels/.rels`, and
Word Online writes `document2.xml`. `[Content_Types].xml` declares the content
type independently of the name.

So the very first thing written against the corpus made exactly the assumption
docxray's parser must not make, and a real document caught it within minutes.

**What it changed.** `scripts/check-fixtures.sh` now resolves the main part
through the package relationships instead of by name. The fact went into the
hidden-state list in `docs/agents/theflow.md`, because part naming is state that
carries meaning and never appears in a Projection.

**The rule it earned.** *A naive assumption written into a check is still a naive
assumption.* Test scaffolding gets written quickly and reviewed loosely, and it
encodes beliefs about the domain just as firmly as the parser does — the
difference is that nobody thinks of it as domain code. This one was cheap because
the corpus contained a counterexample; had the corpus been synthesised from the
same assumption, it would have agreed with the gate all the way into production.
Which is the argument for vendoring real documents, arriving unprompted.

---

## Re-serialising XML destroyed 21 of 24 namespaces, and Word refused the file

**2026-08-19, #3.**

Trimming a 40 MB real document down to a publishable 9 KB fixture. The first
script parsed `word/document.xml` with a standard XML library, removed the
blocks it did not want, replaced the text, and wrote the tree back. Every check
we had passed: the zip was intact, the `officeDocument` relationship resolved,
the main part existed, no original text survived.

Word:

> 이 파일의 일부가 없거나 잘못되었으므로 해당 파일을 열 수 없습니다.
> 위치: 부분: /word/document.xml, 줄: 0, 열: 0

Two causes, both found by probing rather than guessing.

**The original declares 24 namespaces; the round-trip kept 3.** An XML library
preserves only the prefixes it has been told about, so `w14:paraId` came back as
`ns1:paraId` and `r:id` as `ns2:id` — 27 attributes renamed, and `standalone="yes"`
dropped from the declaration for good measure.

**The body ended with a table.** The kept blocks happened to end with `</w:tbl>`,
followed directly by `<w:sectPr>`. The schema permits it. Word does not.

**What it changed.** The script now splices the original XML as text and never
re-serialises it, and appends a paragraph when the body would otherwise end with
a table. Namespaces: 24 of 24. Auto-generated prefixes: zero. Word opens it.

**The rules it earned.** Two, and the second is the sharper one.

*The failure was ADR-0001, reproduced in a fixture script.* "Never round-trip,
splice in place" was written about the product, and a helper script violated it
within a day. What Word did to that script is exactly what it would do to a user's
document if the writer ever re-serialises a part.

*Schema-legal and Word-opens-it are different claims.* Nothing about the
table-at-end-of-body rule is in the specification, so no validator would have
caught it. Six gates were green; the seventh check was a person double-clicking a
file. That is why opening output in Word is a release-gate item in
`docs/agents/theflow.md` and not an aspiration.

---

## Seven green gates could not see that the fixture looked wrong

**2026-08-19, #3.**

The trimmed fixture passed every check we had — valid package, main part
resolving, no original text surviving, Word opening it without complaint. Then a
screenshot arrived. Several paragraphs rendered with their words stretched right
across the page:

> 구조&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;검증을&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;위한&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;문장.

Not a rendering quirk — a data defect. 104 of that document's paragraphs are
`w:jc="both"`, and the scrubbing step had replaced every `<w:t>` with a whole
sentence regardless of the original's size. Original text nodes had a median
length of 21 characters; the replacements ran 9 to 33, and a paragraph with
several runs grew until it wrapped. Justification then stretched every line but
the last.

**What it changed.** Replacement text now matches the original's length exactly —
1,129 characters in the small fixture, 9,594 in the long one. Line breaking
returns to where the real document had it, which matters beyond appearance: line
breaking is part of what decides how Word splits runs, and run splitting is
precisely what the parser has to cope with.

**The rule it earned.** *A corpus has a property no gate can check: whether it
still looks like the thing it was taken from.* Every check we ran reads XML, and
from XML "short text in a justified paragraph" is unremarkable. The defect was
only visible to someone looking at a rendered page — which is the same reason
opening output in Word is a release-gate item, arriving here from the other
direction: not "does it open" but "does it still look like a document".
