# theflow bindings — docxray

Project-specific values that `theflow`'s seven steps defer to. Authored by
`/grill-the-flow` on 2026-08-19, before any implementation code existed; the
mechanical sections are therefore commitments that #2 must satisfy rather than
observations of code that already runs. Re-run the grill when they drift.

---

## Reasoning bindings (project-wide)

### Prior art cross-checked throughout

docxray is not first. Several projects already do most of what it plans, and
reading them is not hygiene here — it is the product. See ADR-0009.

| Project | Stack | What it is authoritative for |
| ------- | ----- | ---------------------------- |
| [kklimuk/docx-cli](https://github.com/kklimuk/docx-cli) | Bun / JS | **The primary reference.** Locator design, Markdown projection with HTML-comment anchors, deviation-only style hints, merge-aware logical grid over `gridSpan`/`vMerge`, in-place XML mutation, `outline` and `validate` command shapes |
| [ruwadgroup/docxengine](https://github.com/ruwadgroup/docxengine) | Python / MCP | Patch mechanics — run coalescing, validate-before-save, tracked-changes emission |
| [dealfluence/adeu](https://github.com/dealfluence/adeu) | Python / Node | Projection-to-markdown and edits-back-as-tracked-changes; CriticMarkup as an alternative encoding |
| [pterror/ooxml](https://github.com/pterror/ooxml) | Rust | Roundtrip-faithful parsing — unknown elements and attributes preserved, never dropped. This is our Opaque Node, already solved in Rust. **Immature (v0.1), reference only, not a dependency** |
| [bokuweb/docx-rs](https://github.com/bokuweb/docx-rs) | Rust | Rust/WASM packaging for the docx domain |
| [officeopenxml.com](http://officeopenxml.com) | Reference | The practical OOXML reference. ECMA-376 itself is the standard but is not what gets read day to day |
| **Microsoft Word** | Application | The only authority on what a document actually *does*, as opposed to what the schema permits. Reachable only by a person opening a file |

### Tie-breaker — prior art vs our own evidence

Not a single rule. Split by the kind of conflict:

| Conflict is about | Winner | Why |
| ----------------- | ------ | --- |
| **An OOXML fact** — what an element means, what Word actually does | **Our own measurement** on real documents and in Word | Prior art gets these wrong. Already caught once: we had made `w:outlineLvl` the primary heading signal, and a real generated document carries none at all |
| **A design convention** — grammar shape, command names, locator notation | **Prior art** | Already validated; renaming `outline` or `validate` for no reason costs familiarity and buys nothing |
| **Something the other stack forced** — a choice traceable to a JS runtime or parser limitation | **Void** — re-decide for Rust | Copying it imports a constraint that does not apply. Ask "why is it like this?"; if the answer is "because of the runtime", do not follow |

The third row is the one that will come up most often, because stack-forced
decisions look like design judgements from the outside.

### Deliberate divergences from the named prior art

Recorded so an adversarial pass does not report the reference's design back as a
defect. All decided in ADR-0009.

- **A library, not a process.** docx-cli is used by spawning it. docxray's core
  crate is the product and the CLI is one adapter, so the same engine embeds into
  WASM, Flutter and other languages (ADR-0002). This is the first of the two
  reasons the project exists.
- **Built on prior art, for accuracy.** The second reason. Where docx-cli and the
  others discovered their design by building it, docxray starts from what they
  learned. Skipping the reference read is therefore not a style violation here —
  it breaks the product's promise.
- **v1 cannot create formatting the Original lacks** (ADR-0005). docx-cli offers
  `styles create`, `tables merge`, footnote insertion. Ours is a v1 sequencing
  decision, not a permanent position, and not a claim of superiority.
- **CLI before MCP** (ADR-0006). docxengine and adeu ship MCP first. Ours lands
  as an adapter once the core is stable.

---

## Crate / module map

Committed shape, to be realised by #2:

- **`docxray`** — library, in `core/`. Package name and directory name differ deliberately: the crate people depend on gets the good name, as `docx-rs` does with its `docx-core/` directory. OPC handling, XML parse and serialise, Block
  identification, Anchor issuance, Projection render and parse, Patch Operation
  planning, Style Catalogue, validation, and the agent-facing text of every
  error. Public API is `&[u8]` in, bytes or a Projection out. **No filesystem or
  path types** (ADR-0002).
- **`docxray-cli`** — binary crate in `cli/`, producing a binary named `docxray`. Not published to the registry. Argument parsing, file IO, sidecar placement, exit
  codes, terminal presentation. Thin by construction (ADR-0006).

Both members sit inside the top-level test workspace. There are no
out-of-workspace members today; if one appears, add it to the gate matrix in the
same change.

---

## Step 1 — reference routing

| Change type | Read this first |
| ----------- | --------------- |
| OOXML semantics — what does this element mean | officeopenxml.com, then `pterror/ooxml` for a typed model, then ECMA-376 for the letter |
| Projection grammar, anchors, style notation | `kklimuk/docx-cli` — the closest existing design |
| Table and merged-cell representation | `kklimuk/docx-cli`'s merge-aware logical grid |
| Patch mechanics — run merge/split, validate-before-write | `ruwadgroup/docxengine` |
| Tracked-changes emission (v1.5) | `ruwadgroup/docxengine`, `dealfluence/adeu` |
| Rust/WASM packaging | `bokuweb/docx-rs` |
| **What Word actually does** — e.g. what formatting a new paragraph inherits on Enter | **Open Word and try it.** No document answers this; this route requires a person |

### Hidden state

State that carries meaning but never appears in a Projection, so an edit can
silently invalidate it. **No list exists yet — building it is part of #4.** Known
members so far:

- `settings.xml` compatibility flags and `compatSetting`
- `rsid` attributes (revision save identifiers) throughout `document.xml`
- Numbering restart state, and the `numbering.xml` definitions a `numPr` points at
- `sectPr` — section properties, including the final body-level one
- Relationship identifiers binding runs to media and hyperlinks
- `[Content_Types].xml` defaults and overrides, whose part ordering varies by producer
- **Part names themselves.** The main document part is *not* fixed at
  `word/document.xml` — it is resolved through the `officeDocument` relationship
  in `_rels/.rels`, and Word Online writes `word/document2.xml`. Confirmed
  against a real fixture; see `docs/agents/lessons.md`.

### The project's own map

**None.** docxray keeps no dependency or territory graph. `CONTEXT.md` and the
ADR index cover vocabulary and decisions but are not a map. Recorded here as an
absence rather than an unfilled section.

---

## Step 2 — boundary rule

**Core owns mechanism. The CLI, and every other adapter, owns policy and IO.**

Core:

- OPC unpack and repack, XML parse and serialise
- Block identification, Anchor issuance, sidecar *content*
- Projection render and parse
- Patch Operation planning and application
- Style Catalogue, Neighbour Inheritance, Dominant Format
- Validation rules, and **the agent-facing wording of every error**

The consumer owns by definition:

- Where files live, and all reading and writing of them
- Where the sidecar is placed (`.docxray/`)
- Argument parsing, exit codes, colour, line wrapping
- Fetching the Original's bytes for the Stale check

**Error text is core, not presentation.** The reader of a validation error is an
agent, so the wording is an interface contract — which fields appear in which
order decides what the agent does next. Errors carry structured fields *and* a
canonical rendering; a consumer that needs different text has the fields, but
gets the good wording for free. This also makes the later MCP adapter thin: it
wraps the same errors.

---

## Step 4 — proof method per layer

| Layer | Real round-trip |
| ----- | --------------- |
| `docxray-core` | Project a fixture, patch back, compare **part by part** against the Original (ADR-0008). Identity with no edits; isolation with one edit |
| `docxray-cli` | Smoke only — argument parsing and exit codes |
| WASM | v1: `cargo check --target wasm32-unknown-unknown` compiles. Upgrade to an executed round-trip when WASM actually ships |

### Traps

- **A round-trip can pass while the file is broken.** If the reader and the
  writer share a bug, project-then-apply is self-consistent and Word still
  refuses the result. Our tests cannot detect this because they only compare us
  against ourselves.
  - CI runs OOXML **schema validation** on output — cheap, every commit.
  - **Opening the output in Word is a release-gate item for a person.** Schema-legal
    and Word-opens-it are different claims, and only the second one matters.
- **Zip bytes are not a comparison surface.** Entry order, timestamps and
  compression level vary without meaning (ADR-0008).
- **Fixtures are narrower than the claim.** The corpus is organisation-form level
  while the product claims fully wild input. A green suite is not evidence about
  documents unlike the fixtures.

---

## Step 5 — unconditional completeness triggers

The completeness pass runs regardless of judgement on exactly one surface:

**Any path that writes bytes over an Original.** Concretely: Patch Operation
application, XML serialisation, zip repack, the Stale check that decides whether
writing is permitted at all, and Opaque Node preservation.

Everything else in docxray fails by refusing, and a refusal is recoverable. Only
this path can destroy a document the user may have no other copy of. The list is
short on purpose.

Two mitigations are part of the design rather than the process:

- `apply` writes to a **new file by default**; overwriting requires `--in-place`.
- Before any write, the produced bytes are re-read and compared part by part
  against the Original, and the write is abandoned if untouched parts differ.
  The test harness and the runtime safety check are the same code.

This is also the only surface where the second, refuting lens is worth its cost.

---

## Step 6 — behavior-describing surfaces

- `CONTEXT.md` — the glossary. Any new domain term lands here.
- `docs/adr/` — decision records. **0001–0009, all accepted.**
- `README.md` — the workflow and the worked example.
- `tests/fixtures/README.md` — what each fixture exercises and where it came from.
- GitHub issue #1 — the v1 spec. Where it and an ADR disagree, the ADR wins.
- `CHANGELOG.md` — does not exist yet; required by #17. Rust packages it into the
  published crate, so it **is** snapshotted at publish.

### What earns a decision record

A choice that will be re-litigated: an architectural boundary, a scope rule, a
position relative to prior art, or a rule that a future contributor would
otherwise reasonably reverse. Areas already carrying a record:

| Area | Record |
| ---- | ------ |
| Projection and patch-back over conversion | ADR-0001 |
| Rust core, IO-agnostic API | ADR-0002 |
| Hybrid Markdown projection, Block-level anchors, run redistribution | ADR-0003 |
| Patches as a typed operation list | ADR-0004 |
| v1 scope — formatting must already exist | ADR-0005 |
| CLI surface, and write semantics | ADR-0006 |
| Anchor scoping | ADR-0007 |
| Test seam and comparison strategy | ADR-0008 |
| Position relative to prior art | ADR-0009 |

Heading detection has been re-decided once already (outline level alone is not
enough) and does not yet have a record — it is the first promotion candidate.

**Tracker parent/child:** GitHub sub-issues and native issue dependencies, both
confirmed working on this repository (#1 has sixteen sub-issues and twenty
dependency edges). Neither the follow-up tree nor a spine roster has to fall back
to prose here.

---

## Step 7 — gate matrix, branch convention, release

### Gates

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps
cargo check -p docxray --target wasm32-unknown-unknown
bash scripts/check-core-io-free.sh
bash scripts/check-fixtures.sh
```

The last two lines cover the same blind spot from different sides, and only one
of them works the way it looks like it should.

`cargo test --workspace` builds for the host, so nothing in the first four lines
notices a core that has quietly started using the filesystem. The **wasm target
check does not close that gap either** — `wasm32-unknown-unknown` ships a std
where `std::fs` compiles and merely fails at runtime, confirmed by probe (see
`docs/agents/lessons.md`). It stays in the matrix because it catches a different
real failure: a dependency that cannot build for wasm at all.

**`scripts/check-fixtures.sh`** guards the corpus every test at the seam depends
on: each fixture must be a readable package whose main part, resolved through its
`officeDocument` relationship, actually exists. Fixtures arrive over the network
and one has already turned up as a 404 body wearing a `.docx` name.

**`scripts/check-core-io-free.sh` is what actually enforces ADR-0002.** It scans
the core for `std::fs`, `std::path`, `std::net`, `std::env`, `std::process` and
`PathBuf`, ignoring comments, and fails the build if any appear. Crude, but it
fails when the contract is broken and passes when it is not, which is the only
property that matters in a gate.

### Branch and PR convention

- **Code changes** — branch, PR, self-merge once gates pass. Gates have to run on
  the PR to mean anything, and on a public repository the PR is where the reason
  for a change survives.
- **Docs and configuration** — commit to `main` directly. Branch-and-merge
  ceremony for a one-line doc change is pure overhead in a solo repository.

### Release and downstream

- Published to crates.io; the CLI is additionally installable as a binary. Semver
  from 0.1.0 (#17).
- **Release gate includes a person opening output in Word.** See Step 4 traps.
- Linking a local build into a consumer for a full-suite round trip: a Cargo
  `path` dependency, or `[patch.crates-io]` for a transitive one.
- **No consumers exist yet.** The consumer list is never stored — it is derived on
  the spot in the after-merge downstream loop.

---

## War-story index

`docs/agents/lessons.md`. Each rule above should eventually point at a concrete
occasion it caught something real; a rule with no war story is still an
abstraction.
