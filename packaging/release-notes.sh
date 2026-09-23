#!/usr/bin/env bash
# Print the CHANGELOG.md section for one version.
#
#   packaging/release-notes.sh v3.2.0            # or "3.2.0"
#   packaging/release-notes.sh 3.2.0 path/to/CHANGELOG.md
#
# Output is the body under "## [3.2.0] - ..." up to the next "## [" heading,
# with leading/trailing blank lines trimmed. Exits 1 (and prints nothing) when
# the version has no section, so callers can fall back to something else.
# Used by scripts/release.sh and the CI release job.
set -euo pipefail

ver=${1:?usage: release-notes.sh <version|tag> [CHANGELOG.md]}
ver=${ver#v}
file=${2:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/CHANGELOG.md"}

[ -f "$file" ] || { echo "release-notes: $file not found" >&2; exit 1; }

out=$(awk -v v="$ver" '
    /^## \[/ {
        if (found) exit
        if (index($0, "## [" v "]") == 1) { found = 1; next }
    }
    found { lines[++n] = $0 }
    END {
        if (!found) exit 1
        s = 1; e = n
        while (s <= e && lines[s] ~ /^[[:space:]]*$/) s++
        while (e >= s && lines[e] ~ /^[[:space:]]*$/) e--
        for (i = s; i <= e; i++) print lines[i]
    }
' "$file") || true

[ -n "$out" ] || exit 1
printf '%s\n' "$out"
