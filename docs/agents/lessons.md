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
