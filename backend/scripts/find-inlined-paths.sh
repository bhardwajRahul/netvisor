#!/usr/bin/env bash
# find-inlined-paths.sh
#
# Reports inlined `crate::server::...::Type` and `crate::daemon::...::Type`
# references in Rust file bodies (i.e., outside the top-of-file `use` block).
# A path appearing >= 2 times in a file is a candidate for hoisting to a
# top-level `use` import.
#
# Usage:
#   scripts/find-inlined-paths.sh                 # scan all .rs files under src/
#   scripts/find-inlined-paths.sh path/to/file.rs # scan specific files
#
# Output: tab-separated `file<TAB>count<TAB>path`, sorted by file then count desc.
# Exit code: 0 if no offenders found, 1 otherwise.

set -euo pipefail

SCAN_TARGETS=("$@")
if [[ ${#SCAN_TARGETS[@]} -eq 0 ]]; then
    SCAN_TARGETS=(src)
fi

# Collect files
FILES=()
for target in "${SCAN_TARGETS[@]}"; do
    if [[ -d "$target" ]]; then
        while IFS= read -r f; do FILES+=("$f"); done < <(find "$target" -type f -name "*.rs")
    elif [[ -f "$target" ]]; then
        FILES+=("$target")
    fi
done

found_any=0

for file in "${FILES[@]}"; do
    # Body = everything after the last top-level `use` block ends. Conservative:
    # consider the file body as the lines that aren't `use ...` statements
    # (and aren't inside a `use {...}` block continuation). Also skip lines that
    # look like comments.
    #
    # Extract path occurrences: `crate::(server|daemon)::...::Identifier`
    # where Identifier starts with an uppercase letter (so we capture types,
    # not module names). Stop the regex at non-path characters.
    #
    # Use perl for multiline-aware extraction with a state machine.
    perl -ne '
        # Track whether we are inside a top-level `use ... { ... };` block.
        BEGIN { our $in_use_block = 0; }
        if (/^\s*use\s+\S+\s*\{[^}]*$/) { $in_use_block = 1; next; }
        if ($in_use_block) {
            if (/\}\s*;/) { $in_use_block = 0; }
            next;
        }
        # Single-line `use` statement.
        next if /^\s*use\s+/;
        # Skip line comments.
        next if /^\s*\/\//;

        # Find any inlined paths.
        while (/(crate::(?:server|daemon)::[A-Za-z0-9_:#]+::[A-Z][A-Za-z0-9_]*)/g) {
            print "$1\n";
        }
    ' "$file" | sort | uniq -c | awk -v f="$file" '$1 >= 2 { printf "%s\t%d\t%s\n", f, $1, $2 }'
done | sort -k1,1 -k2,2nr | { tee /dev/stderr | grep -q .; } && found_any=1 || found_any=0

if [[ "$found_any" -eq 1 ]]; then
    exit 1
fi
exit 0
