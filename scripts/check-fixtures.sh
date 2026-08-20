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

  # Capture the names, then match in the shell. Streaming `unzip -Z1 | grep -q`
  # is what this used to do, and it failed about one run in twelve: grep exits
  # the moment it matches, unzip dies of SIGPIPE with 141, and `pipefail` turns
  # that into a failed pipeline — so a perfectly good fixture was reported
  # missing, a different one each time. Measured, not deduced.
  #
  # The `case` also matches a whole name rather than a substring, so a part
  # called `notword/document.xml` can no longer satisfy a check for
  # `word/document.xml`.
  names=$(unzip -Z1 "$f" 2>/dev/null)
  case $'\n'"$names"$'\n' in
    *$'\n'"$target"$'\n'*) ;;
    *)
      echo "main part '$target' declared but missing: $f"
      fail=1
      ;;
  esac
done < <(find tests/fixtures -name '*.docx' -print0)

if [ "$count" -eq 0 ]; then
  echo "no fixtures found — the corpus is the prerequisite for every test at the seam"
  exit 1
fi

[ "$fail" -eq 0 ] && echo "$count fixtures readable"
exit "$fail"
