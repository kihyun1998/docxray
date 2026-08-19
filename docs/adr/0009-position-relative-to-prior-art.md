# Position relative to prior art

docxray is not the first tool to give an agent a safe way to edit a `.docx`.
[kklimuk/docx-cli](https://github.com/kklimuk/docx-cli) had already arrived
independently at nearly this entire design — a Markdown view over the parsed XML
tree rather than a separate model, HTML-comment locators on paragraphs and cells,
in-place mutation with untouched regions bypassing re-emission, `outline` and
`validate` commands, deviation-only style hints, a merge-aware logical grid over
`gridSpan`/`vMerge`, and a refusal to bisect existing merges. It also goes
further, with tracked changes, footnotes, equations and PDF rendering.
[ruwadgroup/docxengine](https://github.com/ruwadgroup/docxengine) and
[dealfluence/adeu](https://github.com/dealfluence/adeu) cover adjacent ground.

The project continues for two reasons, and only both together make it a different
thing.

**It is a library, not a process.** The existing tools are used by spawning them.
docxray's core crate is the product and the CLI is one adapter over it
(ADR-0002), so the same engine embeds into WASM, `flutter_rust_bridge` and other
languages. A tool you can only shell out to cannot go in a browser or an app.

**It starts from what the others learned.** They discovered their designs by
building them. docxray reads them first, which is where the accuracy comes from —
the merge-aware grid and the deviation-only style notation are answers we would
otherwise have spent weeks re-deriving, probably worse.

The second reason has a consequence that outranks its convenience: **reading the
reference is not process hygiene here, it is the product promise.** Code written
without checking the prior art has broken what this project claims to be, not
merely a convention.

## Considered options

- **Stop, and use docx-cli.** A real option, and the cheapest moment to take it
  was before any code existed. Rejected because the embeddable-core niche is
  genuinely empty: docx-cli, docxengine and adeu are JS or Python, and the Rust
  entries (`docx-rs`, `pterror/ooxml`) are a writer and an immature parser, not
  agent-facing editors.
- **Differentiate on scope discipline alone.** v1 refuses to invent formatting
  the Original lacks (ADR-0005), which is a stronger safety guarantee than the
  alternatives offer. Rejected as *the* differentiator: it is a sequencing
  decision, and a competitor closes the gap with a flag.
- **Differentiate on Rust alone.** Rejected — that is "the JS one, rewritten",
  which is not a reason for anyone to switch.

## Consequences

- The Step 1 reference routing table in `docs/agents/theflow.md` is load-bearing
  rather than advisory, and skipping it is a defect.
- Divergences from the named prior art are recorded deliberately, so an
  adversarial pass does not keep proposing the reference's design back as a
  finding.
- ADR-0005's scope rule stays exactly as it is, but is positioned as a v1
  sequencing decision rather than a claim of superiority.
- Where prior art and our own measurement disagree, the tie-breaker is split by
  the kind of conflict — see the reasoning bindings. Prior art loses on OOXML
  facts, wins on design conventions, and is void where its choice was forced by
  its own stack.
