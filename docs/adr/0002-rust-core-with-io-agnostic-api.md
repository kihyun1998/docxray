# Rust core crate with an IO-agnostic API, CLI as the first adapter

The core is a Rust crate whose API is `&[u8]` → `Vec<u8>`; it does not know what a file path is. All file IO lives in the CLI adapter.

Rust is not the obvious pick — its docx ecosystem is thin — but patch-back cannot use a high-level docx library anyway. Libraries like pandoc and python-docx are built on "read it and produce something else", which is precisely what we refuse to do. What we actually need is a zip reader and an XML parser, and the work itself is byte-exact low-level XML surgery, which is Rust's strong suit.

## Consequences

- WASM and `flutter_rust_bridge` bindings stay possible at no extra cost. Retrofitting IO-ignorance later would be a full rewrite of the core; holding the line now is free.
- CLI comes before MCP deliberately. A CLI makes the Projection a real file a human can open and `git diff`, and it doubles as the integration test harness. MCP failures are swallowed on the far side of stdio and are miserable to debug; it lands as an adapter once the core is stable.
