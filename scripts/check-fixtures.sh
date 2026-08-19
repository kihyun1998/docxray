#!/usr/bin/env bash
# Every committed fixture must be a readable OPC package whose main document
# part actually exists. Fixtures arrive over the network, and a silent
# truncation would poison every test that later depends on them — one such
# download already returned a 14-byte 404 body while looking like success.
#
# The main part is resolved the way the spec says: follow the officeDocument
# relationship in _rels/.rels. It is NOT always word/document.xml — Word Online
# writes word/document2.xml, and a parser that hardcodes the name breaks on it.
# This check found that out the hard way; see docs/agents/lessons.md.
set -uo pipefail

fail=0
count=0

while IFS= read -r -d '' f; do
  count=$((count + 1))

  if ! unzip -tqq "$f" >/dev/null 2>&1; then
    echo "not a readable zip: $f"
    fail=1
    continue
  fi

  rels=$(unzip -p "$f" '_rels/.rels' 2>/dev/null)
  if [ -z "$rels" ]; then
    echo "no package relationships (_rels/.rels): $f"
    fail=1
    continue
  fi

  # Target of the relationship whose Type ends in /officeDocument.
  target=$(printf '%s' "$rels" \
    | tr '<' '\n' \
    | grep 'officeDocument"' \
    | sed -n 's/.*Target="\([^"]*\)".*/\1/p' \
    | head -1 \
    | sed 's#^/##')

  if [ -z "$target" ]; then
    echo "no officeDocument relationship: $f"
    fail=1
    continue
  fi

  if ! unzip -l "$f" 2>/dev/null | grep -qF " $target"; then
    echo "main part '$target' declared but missing: $f"
    fail=1
  fi
done < <(find tests/fixtures -name '*.docx' -print0)

if [ "$count" -eq 0 ]; then
  echo "no fixtures found — the corpus is the prerequisite for every test at the seam"
  exit 1
fi

[ "$fail" -eq 0 ] && echo "$count fixtures readable"
exit "$fail"
