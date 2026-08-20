# docxray

Surgical DOCX editing for AI agents — project a Word document into an editable text view, edit it, and patch the changes back into the original file. Untouched content stays byte-identical.

Conversion loses everything the intermediate format has no slot for: per-run fonts, character shading, spacing, section setup, comments, tracked changes. docxray never converts. The original stays the source of truth, the projection is a disposable view, and only the blocks that changed are rewritten.

**Status: early.** A prose document projects, and patching back with no edits returns it unchanged. Applying an actual edit is the next slice and is refused until it exists. Tables, `outline` and `check` are not written yet. See [issue #1](https://github.com/kihyun1998/docxray/issues/1) for the v1 spec and its tickets.

## The loop

```sh
docxray open report.docx     # -> report.dxr, and .docxray/report.json beside it
                             #    edit report.dxr with ordinary text tools
docxray apply report.dxr     # -> report.out.docx
```

`apply` never writes over the Original: a `.docx` is often the only copy of something. It refuses a Projection whose Original moved underneath it, and it compares its own output against the Original part by part before handing it back.

## Layout

| | |
| --- | --- |
| `core/` — the `docxray` crate | The product. Bytes in, bytes out; it does not know what a file path is, so the same engine runs under WebAssembly and behind foreign-language bindings |
| `cli/` — the `docxray` binary | One adapter over the core. Argument parsing, file IO, exit codes |

## Building

```sh
cargo test --workspace
cargo run -p docxray-cli -- --version
```

Requires Rust 1.85 or newer (edition 2024).

## Gates

CI runs these on every push and pull request; run them locally before opening one.

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps -p docxray
cargo check -p docxray --target wasm32-unknown-unknown
bash scripts/check-core-io-free.sh
bash scripts/check-fixtures.sh
```

`check-core-io-free.sh` enforces the core's IO-free contract. The wasm check does not — `wasm32-unknown-unknown` compiles `std::fs` happily and only fails at runtime, which is why both are here. `check-fixtures.sh` guards the corpus every test at the seam depends on.

## Design

Decisions live in [`docs/adr/`](docs/adr/) and the vocabulary in [`CONTEXT.md`](CONTEXT.md). Where a document and an ADR disagree, the ADR wins.

docxray is not the first tool in this space, and it does not pretend to be — [ADR-0009](docs/adr/0009-position-relative-to-prior-art.md) names the prior art it reads and says why the project continues anyway.

## Licence

MIT or Apache-2.0, at your option.
