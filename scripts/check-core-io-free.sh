#!/usr/bin/env bash
# The core is IO-agnostic by contract (ADR-0002): bytes in, bytes out, no
# knowledge of paths. Nothing in the ordinary gate matrix enforces that.
# `cargo check --target wasm32-unknown-unknown` looks like it should, but
# wasm32-unknown-unknown ships a std where `std::fs` compiles and merely fails
# at runtime — verified by probe, see docs/agents/lessons.md.
set -uo pipefail

FORBIDDEN='std::(fs|path|net|env|process)\b|PathBuf'

# Strip line and doc comments first: the contract is discussed in prose there.
hits=$(find core/src -name '*.rs' -print0 \
  | xargs -0 grep -nE "$FORBIDDEN" \
  | grep -vE ':[0-9]+:[[:space:]]*//')

if [ -n "$hits" ]; then
  echo "core must stay IO-agnostic (ADR-0002) — found:"
  echo "$hits"
  exit 1
fi
echo "core is IO-free"
