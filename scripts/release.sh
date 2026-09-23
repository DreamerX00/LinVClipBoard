#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
#  LinVClipBoard release pilot
#
#  Interactive, end-to-end release: asks for the version, headline and notes,
#  bumps every synced version file, updates CHANGELOG.md, commits, tags, pushes,
#  builds the artifacts (locally and/or on GitHub Actions) and publishes the
#  GitHub release with your notes, the Linux packages and the Windows installer.
#
#    make release                 # or: scripts/release.sh
#    scripts/release.sh --dry-run # walk through the prompts, change nothing
#
#  Flags (all optional; anything not given is asked interactively):
#    --version X.Y.Z[-pre]   release version (tag becomes vX.Y.Z)
#    --headline "text"       short headline appended to the release title
#    --notes-file FILE       markdown release notes instead of the editor
#    --mode ci|local|both    where to build: GitHub Actions, this machine, or both
#    --prerelease            mark as pre-release (implied by a "-" in the version)
#    --draft                 leave the GitHub release as a draft
#    --no-changelog          do not write the notes into CHANGELOG.md
#    --no-auto-notes         do not append GitHub's generated "What's Changed" list
#    --skip-build            local mode: publish artifacts already in target/
#    --no-push               stop after commit + tag (nothing leaves this machine)
#    --yes                   accept the summary without the final confirmation
#    --dry-run               show every step, execute none of the mutating ones
#
#  Requirements: git, gh (logged in), jq, cargo, node/npm. Local Windows builds
#  additionally need cargo-xwin and makensis; without them the .exe comes from CI.
# ──────────────────────────────────────────────────────────────────────────────
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UI_DIR="$ROOT/crates/linvclip-ui"
DIST_DIR="$ROOT/dist/release"
LOG_DIR="$DIST_DIR/logs/$(date +%Y%m%d-%H%M%S)"
REPO_SLUG=""            # owner/name, resolved in preflight
MAIN_BRANCH="main"
WIN_TARGET="x86_64-pc-windows-msvc"
CI_WORKFLOW="CI"
CI_TIMEOUT_MIN=120

unset GH_DEBUG DEBUG    # gh traces every request when either is set; that would drown the UI
export CARGO_TERM_COLOR=always

# ── inputs (flags fill these in; prompts fill the rest) ───────────────────────
VERSION="" HEADLINE="" NOTES_FILE="" MODE=""
PRERELEASE=false DRAFT=false UPDATE_CHANGELOG=true AUTO_NOTES=true
SKIP_BUILD=false NO_PUSH=false ASSUME_YES=false DRY_RUN=false
HEADLINE_SET=false

usage() { sed -n '2,32p' "${BASH_SOURCE[0]}" | sed 's/^#  \{0,1\}//'; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)      VERSION=${2:?}; shift 2 ;;
        --headline)     HEADLINE=${2-}; HEADLINE_SET=true; shift 2 ;;
        --notes-file)   NOTES_FILE=$(readlink -f "${2:?}"); shift 2 ;;
        --mode)         MODE=${2:?}; shift 2 ;;
        --prerelease)   PRERELEASE=true; shift ;;
        --draft)        DRAFT=true; shift ;;
        --no-changelog) UPDATE_CHANGELOG=false; shift ;;
        --no-auto-notes) AUTO_NOTES=false; shift ;;
        --skip-build)   SKIP_BUILD=true; shift ;;
        --no-push)      NO_PUSH=true; shift ;;
        --yes|-y)       ASSUME_YES=true; shift ;;
        --dry-run)      DRY_RUN=true; shift ;;
        -h|--help)      usage; exit 0 ;;
        *) echo "Unknown flag: $1 (try --help)" >&2; exit 2 ;;
    esac
done

# ══════════════════════════════════════════════════════════════════════════════
#  Terminal toolkit
# ══════════════════════════════════════════════════════════════════════════════
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    E=$'\e'
    RST="$E[0m" BOLD="$E[1m" DIM="$E[2m" ITAL="$E[3m" UL="$E[4m"
    C_PRI="$E[38;5;141m" C_ACC="$E[38;5;212m" C_INFO="$E[38;5;81m"
    C_OK="$E[38;5;114m" C_WARN="$E[38;5;221m" C_ERR="$E[38;5;203m" C_MUTED="$E[38;5;245m"
    GRADIENT=(99 105 111 117 123 159)
    IS_TTY=true
else
    RST="" BOLD="" DIM="" ITAL="" UL="" C_PRI="" C_ACC="" C_INFO="" C_OK="" C_WARN="" C_ERR="" C_MUTED=""
    GRADIENT=()
    IS_TTY=false
fi
INTERACTIVE=false; [[ -t 0 && -t 1 ]] && INTERACTIVE=true

FRAMES=(⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏)
CHILD_PID=""
STEP_N=0
PHASE_N=0
PHASE_TOTAL=0

cols() { local c; c=$(tput cols 2>/dev/null || echo 80); (( c > 100 )) && c=100; echo "$c"; }
hide_cursor() { $IS_TTY && printf '\e[?25l' || true; }
show_cursor() { $IS_TTY && printf '\e[?25h' || true; }
cursor_up()   { $IS_TTY && (( $1 > 0 )) && printf '\e[%dA' "$1" || true; }
clear_below() { $IS_TTY && printf '\e[J' || true; }
fmt_dur() { local s=$1; if (( s >= 3600 )); then printf '%dh %02dm' $((s/3600)) $((s%3600/60)); elif (( s >= 60 )); then printf '%dm %02ds' $((s/60)) $((s%60)); else printf '%ds' "$s"; fi; }
fmt_size() { local b=$1; if (( b >= 1048576 )); then awk -v b="$b" 'BEGIN{printf "%.1f MB", b/1048576}'; elif (( b >= 1024 )); then awk -v b="$b" 'BEGIN{printf "%.0f KB", b/1024}'; else printf '%d B' "$b"; fi; }
slug() { printf '%s' "$1" | tr -cs 'A-Za-z0-9' '-' | sed 's/^-//; s/-$//' | tr 'A-Z' 'a-z'; }
hline() { (( $1 > 0 )) && printf '%*s' "$1" '' | sed 's/ /─/g'; }   # tr is byte-based; ─ is multibyte
rule() { printf '  %s%s%s\n' "$C_MUTED" "$(hline $(( $(cols) - 2 )))" "$RST"; }

say()    { printf '  %s\n' "$*"; }
info()   { printf '  %s◆%s %s\n' "$C_INFO" "$RST" "$*"; }
ok()     { printf '  %s✔%s %s\n' "$C_OK" "$RST" "$*"; }
warn()   { printf '  %s▲%s %s\n' "$C_WARN" "$RST" "$*"; }
fail()   { printf '  %s✖%s %s\n' "$C_ERR" "$RST" "$*" >&2; }
die()    { fail "$@"; exit 1; }
kv()     { printf '  %s%-16s%s %s\n' "$C_MUTED" "$1" "$RST" "$2"; }

cleanup() {
    local rc=$?
    [[ -n $CHILD_PID ]] && kill "$CHILD_PID" 2>/dev/null || true
    show_cursor
    if (( rc != 0 )) && ! $DRY_RUN; then
        echo
        [[ -d $LOG_DIR ]] && printf '  %sLogs:%s %s\n' "$C_MUTED" "$RST" "$LOG_DIR"
    fi
    exit "$rc"
}
trap cleanup EXIT
trap 'echo; fail "Interrupted."; exit 130' INT TERM

banner() {
    local art=(
'██████╗ ███████╗██╗     ███████╗ █████╗ ███████╗███████╗'
'██╔══██╗██╔════╝██║     ██╔════╝██╔══██╗██╔════╝██╔════╝'
'██████╔╝█████╗  ██║     █████╗  ███████║███████╗█████╗  '
'██╔══██╗██╔══╝  ██║     ██╔══╝  ██╔══██║╚════██║██╔══╝  '
'██║  ██║███████╗███████╗███████╗██║  ██║███████║███████╗'
'╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝'
    )
    echo
    local i
    for i in "${!art[@]}"; do
        if $IS_TTY; then printf '  \e[38;5;%dm%s%s\n' "${GRADIENT[i]}" "${art[i]}" "$RST"; else printf '  %s\n' "${art[i]}"; fi
    done
    printf '  %s%sLinVClipBoard%s %s· release pilot ·%s %s%s%s\n' "$BOLD" "$C_PRI" "$RST" "$C_MUTED" "$RST" "$DIM" "$(date '+%Y-%m-%d %H:%M')" "$RST"
    $DRY_RUN && printf '  %s%s DRY RUN — nothing will be changed, pushed or published %s\n' "$C_WARN" "$BOLD" "$RST"
    echo
}

phase() {
    PHASE_N=$((PHASE_N+1))
    local title=$1 w line
    w=$(cols)
    echo
    local num="$PHASE_N"; (( PHASE_TOTAL > 0 )) && num="$PHASE_N/$PHASE_TOTAL"
    line=$(hline $(( w - ${#title} - ${#num} - 8 )))
    printf '  %s%s%s%s %s%s%s %s%s%s\n' "$BOLD" "$C_PRI" "$num" "$RST" "$BOLD" "$title" "$RST" "$C_MUTED" "$line" "$RST"
    echo
}

# run_step "label" command [args...]   — spinner + log; aborts on failure.
run_step() {
    local label=$1; shift
    STEP_N=$((STEP_N+1))
    if $DRY_RUN; then
        printf '  %s○%s %s %s→ %s%s\n' "$C_MUTED" "$RST" "$label" "$DIM" "$*" "$RST"
        return 0
    fi
    mkdir -p "$LOG_DIR"
    local log="$LOG_DIR/$(printf '%02d' "$STEP_N")-$(slug "$label").log"
    local start=$SECONDS i=0 rc=0
    { printf '$ %s\n\n' "$*"; } >"$log"
    ( cd "$ROOT" && "$@" ) >>"$log" 2>&1 &
    CHILD_PID=$!
    hide_cursor
    while kill -0 "$CHILD_PID" 2>/dev/null; do
        if $IS_TTY; then
            printf '\r  %s%s%s %s %s%s%s\e[K' "$C_ACC" "${FRAMES[i % ${#FRAMES[@]}]}" "$RST" "$label" "$C_MUTED" "$(fmt_dur $((SECONDS-start)))" "$RST"
        fi
        i=$((i+1)); sleep 0.1
    done
    wait "$CHILD_PID" || rc=$?
    CHILD_PID=""
    show_cursor
    if (( rc == 0 )); then
        printf '\r  %s✔%s %s %s%s%s\e[K\n' "$C_OK" "$RST" "$label" "$C_MUTED" "$(fmt_dur $((SECONDS-start)))" "$RST"
    else
        printf '\r  %s✖%s %s %s(exit %d after %s)%s\e[K\n' "$C_ERR" "$RST" "$label" "$C_MUTED" "$rc" "$(fmt_dur $((SECONDS-start)))" "$RST"
        echo
        printf '  %s── last 30 log lines · %s ──%s\n' "$C_MUTED" "$log" "$RST"
        tail -n 30 "$log" | sed 's/^/  │ /'
        echo
        [[ -n ${ON_STEP_FAIL:-} ]] && "$ON_STEP_FAIL"
        exit "$rc"
    fi
}

# run_ro "label" command...  — like run_step, but runs even under --dry-run (read-only work)
run_ro() { local d=$DRY_RUN; DRY_RUN=false; run_step "$@"; DRY_RUN=$d; }

# spin_wait SECONDS "label"  — animate while sleeping.
spin_wait() {
    local secs=$1 label=$2 i=0 end=$((SECONDS + secs))
    hide_cursor
    while (( SECONDS < end )); do
        $IS_TTY && printf '\r  %s%s%s %s\e[K' "$C_ACC" "${FRAMES[i % ${#FRAMES[@]}]}" "$RST" "$label"
        i=$((i+1)); sleep 0.1
    done
    $IS_TTY && printf '\r\e[K'
    show_cursor
}

# ask VAR "prompt" "default"
ask() {
    local -n _out=$1; local prompt=$2 default=${3-} ans
    if ! $INTERACTIVE; then _out=$default; return; fi
    printf '  %s?%s %s%s%s' "$C_ACC" "$RST" "$BOLD" "$prompt" "$RST"
    [[ -n $default ]] && printf ' %s(%s)%s' "$C_MUTED" "$default" "$RST"
    printf ': '
    IFS= read -r ans || ans=""
    _out=${ans:-$default}
}

# confirm "prompt" [y|n]  → 0 yes / 1 no
confirm() {
    local prompt=$1 default=${2:-n} ans hint="y/N"
    [[ $default == y ]] && hint="Y/n"
    if ! $INTERACTIVE; then [[ $default == y ]]; return; fi
    printf '  %s?%s %s%s%s %s[%s]%s ' "$C_ACC" "$RST" "$BOLD" "$prompt" "$RST" "$C_MUTED" "$hint" "$RST"
    IFS= read -r ans || ans=""
    ans=${ans:-$default}
    [[ $ans =~ ^[Yy] ]]
}

# choose "prompt" "opt 1" "opt 2" ...  → CHOICE_INDEX / CHOICE ; arrows/jk/digits/enter
CHOICE_INDEX=0; CHOICE=""
choose() {
    local prompt=$1; shift
    local opts=("$@") n=$# idx=0 key rest i
    if ! $INTERACTIVE; then CHOICE_INDEX=0; CHOICE=${opts[0]}; return; fi
    printf '  %s?%s %s%s%s %s↑/↓ · enter%s\n' "$C_ACC" "$RST" "$BOLD" "$prompt" "$RST" "$C_MUTED" "$RST"
    hide_cursor
    while :; do
        for i in "${!opts[@]}"; do
            if (( i == idx )); then printf '    %s❯ %s%s\e[K\n' "$C_ACC" "${opts[i]}" "$RST"
            else printf '      %s%s%s\e[K\n' "$C_MUTED" "${opts[i]}" "$RST"; fi
        done
        IFS= read -rsn1 key || key=""
        if [[ $key == $'\e' ]]; then IFS= read -rsn2 -t 0.05 rest || rest=""; key+=$rest; fi
        case "$key" in
            $'\e[A'|k) (( idx > 0 )) && idx=$((idx-1)) ;;
            $'\e[B'|j) (( idx < n-1 )) && idx=$((idx+1)) ;;
            ''|$'\n')  break ;;
            [1-9])     (( key <= n )) && { idx=$((key-1)); break; } ;;
        esac
        cursor_up "$n"
    done
    cursor_up $((n+1)); clear_below
    show_cursor
    CHOICE_INDEX=$idx; CHOICE=${opts[idx]}
    printf '  %s✔%s %s %s→ %s%s\n' "$C_OK" "$RST" "$prompt" "$C_INFO" "${CHOICE%% —*}" "$RST"
}

# ══════════════════════════════════════════════════════════════════════════════
#  Helpers
# ══════════════════════════════════════════════════════════════════════════════
current_version() { grep -m1 '^version = ' "$ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/'; }
semver_ok()       { [[ $1 =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; }
semver_gt()       { [[ $1 != "$2" && $(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1) == "$1" ]]; }
bump_part() {     # bump_part X.Y.Z major|minor|patch
    local IFS=. ; local -a p; read -ra p <<<"${1%%-*}"
    case $2 in
        major) echo "$((p[0]+1)).0.0" ;;
        minor) echo "${p[0]}.$((p[1]+1)).0" ;;
        patch) echo "${p[0]}.${p[1]}.$((p[2]+1))" ;;
    esac
}
tag_exists_local()  { git -C "$ROOT" rev-parse -q --verify "refs/tags/$1" >/dev/null 2>&1; }
tag_exists_remote() { git -C "$ROOT" ls-remote --exit-code --tags origin "refs/tags/$1" >/dev/null 2>&1; }
release_exists()    { gh release view "$1" --repo "$REPO_SLUG" >/dev/null 2>&1; }
pick_editor() {
    local e=${VISUAL:-${EDITOR:-}}
    if [[ -z $e ]]; then for e in nano vim vi; do command -v "$e" >/dev/null 2>&1 && break; e=""; done; fi
    echo "$e"
}

# ══════════════════════════════════════════════════════════════════════════════
#  1 · Preflight
# ══════════════════════════════════════════════════════════════════════════════
banner
phase "Preflight"

cd "$ROOT"
for tool in git gh jq cargo npm node sed awk sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || die "Missing required tool: $tool"
done
ok "Tools: git, gh, jq, cargo, npm"

gh auth status >/dev/null 2>&1 || die "gh is not logged in — run: gh auth login"
REPO_SLUG=$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || true)
[[ -n $REPO_SLUG ]] || REPO_SLUG=$(git remote get-url origin | sed -E 's#.*github.com[:/]##; s#\.git$##')
ok "GitHub: $REPO_SLUG as $(gh api user -q .login 2>/dev/null || echo '?')"

BRANCH=$(git rev-parse --abbrev-ref HEAD)
START_SHA=$(git rev-parse HEAD)
if [[ -n $(git status --porcelain) ]]; then
    git status --short | head -n 10 | sed 's/^/    /'
    die "Working tree is not clean — commit or discard the changes above first."
fi
ok "Working tree clean on ${BOLD}$BRANCH${RST} @ ${START_SHA:0:7}"

run_ro "Fetch origin" git fetch --quiet --tags origin
if git rev-parse -q --verify "origin/$BRANCH" >/dev/null 2>&1; then
    if [[ $(git rev-parse HEAD) != $(git rev-parse "origin/$BRANCH") ]]; then
        if ! git merge-base --is-ancestor "origin/$BRANCH" HEAD; then
            die "Local $BRANCH is behind origin/$BRANCH — pull first."
        fi
        warn "Local $BRANCH is ahead of origin/$BRANCH by $(git rev-list --count "origin/$BRANCH..HEAD") commit(s); they will be pushed with the release."
    fi
fi
if [[ $BRANCH != "$MAIN_BRANCH" ]] && ! $DRY_RUN; then
    warn "You are on ${BOLD}$BRANCH${RST}, not $MAIN_BRANCH. The release commit is pushed to this branch; the tag build runs either way."
    confirm "Release from $BRANCH anyway?" n || die "Aborted — switch to $MAIN_BRANCH and retry."
fi

FREE_GB=$(df -BG --output=avail "$ROOT" | tail -1 | tr -dc '0-9')
(( FREE_GB < 10 )) && warn "Only ${FREE_GB} GB free on this disk — a full local build needs roughly 10 GB."

HAVE_XWIN=false HAVE_NSIS=false
command -v cargo-xwin >/dev/null 2>&1 && HAVE_XWIN=true
command -v makensis   >/dev/null 2>&1 && HAVE_NSIS=true
CAN_BUILD_WIN=false; $HAVE_XWIN && $HAVE_NSIS && CAN_BUILD_WIN=true
if $CAN_BUILD_WIN; then ok "Windows cross-build available (cargo-xwin + makensis)"
else info "Windows cross-build unavailable here ($($HAVE_XWIN || echo -n 'cargo-xwin ')$($HAVE_NSIS || echo -n 'makensis ')missing) — the .exe will come from GitHub Actions."; fi

# ══════════════════════════════════════════════════════════════════════════════
#  2 · Inputs
# ══════════════════════════════════════════════════════════════════════════════
phase "What are we shipping?"

OLD_VERSION=$(current_version)
kv "Current version" "$OLD_VERSION"
kv "Last tag" "$(git describe --tags --abbrev=0 2>/dev/null || echo none)"
kv "Latest release" "$(gh release view --repo "$REPO_SLUG" --json tagName -q .tagName 2>/dev/null || echo none)"
echo

if [[ -z $VERSION ]]; then
    P=$(bump_part "$OLD_VERSION" patch); M=$(bump_part "$OLD_VERSION" minor); J=$(bump_part "$OLD_VERSION" major)
    choose "Version to release" \
        "$P — patch: bug fixes only" \
        "$M — minor: new features, backwards compatible" \
        "$J — major: breaking changes" \
        "$OLD_VERSION — re-release the current version (no bump)" \
        "custom…"
    case $CHOICE_INDEX in
        0) VERSION=$P ;; 1) VERSION=$M ;; 2) VERSION=$J ;; 3) VERSION=$OLD_VERSION ;;
        4) ask VERSION "Version (SemVer, e.g. 3.2.0 or 3.2.0-beta.1)" "$M" ;;
    esac
fi
VERSION=${VERSION#v}
semver_ok "$VERSION" || die "'$VERSION' is not a valid SemVer version."
TAG="v$VERSION"
[[ $VERSION == *-* ]] && PRERELEASE=true
if [[ $VERSION != "$OLD_VERSION" ]] && ! semver_gt "$VERSION" "$OLD_VERSION"; then
    warn "$VERSION is lower than the current $OLD_VERSION."
    confirm "Continue anyway?" n || die "Aborted."
fi

if tag_exists_remote "$TAG"; then
    die "Tag $TAG already exists on origin. Pick a new version (or delete the remote tag deliberately first)."
fi
if tag_exists_local "$TAG"; then
    warn "Tag $TAG exists locally (not on origin) → $(git rev-parse --short "$TAG") — $(git tag -n1 "$TAG" | sed "s/^$TAG *//")"
    if confirm "Delete the local tag and recreate it for this release?" y; then
        $DRY_RUN || git tag -d "$TAG" >/dev/null
        ok "Local tag $TAG removed"
    else
        die "Aborted — cannot reuse an existing tag."
    fi
fi
RELEASE_PREEXISTS=false
if release_exists "$TAG"; then
    RELEASE_PREEXISTS=true
    warn "A GitHub release for $TAG already exists; it will be updated in place."
    confirm "Continue?" y || die "Aborted."
fi
ok "Releasing ${BOLD}$TAG${RST}$([[ $VERSION != "$OLD_VERSION" ]] && echo " (from $OLD_VERSION)")"

if ! $HEADLINE_SET && $INTERACTIVE; then
    echo
    ask HEADLINE "Headline — a few words for the release title (empty for just '$TAG')" ""
fi
TITLE="$TAG"; [[ -n $HEADLINE ]] && TITLE="$TAG — $HEADLINE"

# ── release notes ─────────────────────────────────────────────────────────────
mkdir -p "$DIST_DIR"
NOTES_TMP="$DIST_DIR/notes-$VERSION.md"
EXISTING_NOTES=$("$ROOT/packaging/release-notes.sh" "$VERSION" 2>/dev/null || true)

if [[ -n $NOTES_FILE ]]; then
    [[ -s $NOTES_FILE ]] || die "Notes file $NOTES_FILE is empty."
    cp "$NOTES_FILE" "$NOTES_TMP"
else
    echo
    EDITOR_BIN=$(pick_editor)
    notes_opts=()
    [[ -n $EDITOR_BIN ]] && notes_opts+=("Open ${EDITOR_BIN##*/} with a template")
    notes_opts+=("Type them here — finish with a line containing only '.'")
    [[ -n $EXISTING_NOTES ]] && notes_opts+=("Use the existing CHANGELOG.md section for $VERSION as-is")
    choose "Release notes (markdown, shown on the GitHub release page)" "${notes_opts[@]}"
    picked=${notes_opts[CHOICE_INDEX]}
    if [[ $picked == Open* ]]; then
        {
            if [[ -n $EXISTING_NOTES ]]; then printf '%s\n' "$EXISTING_NOTES"; else
                cat <<'TPL'
### ✨ New

- 

### 🐛 Fixed

- 

### 🔧 Changed

- 
TPL
            fi
            cat <<TPL

%% Release notes for $TITLE — markdown, shown on the GitHub release page
%% and written into CHANGELOG.md. Lines starting with %% are dropped.
%% Empty sections ("- " with nothing after it) are dropped too.
%% Save and quit when done; an empty file aborts the release.
TPL
        } >"$NOTES_TMP"
        "$EDITOR_BIN" "$NOTES_TMP" </dev/tty >/dev/tty
        # drop comment lines, empty bullets, and headings left without content
        awk '
            /^%%/ { next }
            /^-[[:space:]]*$/ { next }
            { lines[++n] = $0 }
            END {
                for (i = 1; i <= n; i++) {
                    if (lines[i] ~ /^#+ /) {
                        keep = 0
                        for (j = i + 1; j <= n && lines[j] !~ /^#+ /; j++) if (lines[j] !~ /^[[:space:]]*$/) { keep = 1; break }
                        if (!keep) continue
                    }
                    print lines[i]
                }
            }' "$NOTES_TMP" | cat -s | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' >"$NOTES_TMP.clean"
        mv "$NOTES_TMP.clean" "$NOTES_TMP"
    elif [[ $picked == Type* ]]; then
        say "${C_MUTED}Markdown welcome. A line with just '.' ends the input.${RST}"
        : >"$NOTES_TMP"
        while IFS= read -r line; do [[ $line == "." ]] && break; printf '%s\n' "$line" >>"$NOTES_TMP"; done
    else
        printf '%s\n' "$EXISTING_NOTES" >"$NOTES_TMP"
    fi
fi
sed -i -e 's/[[:space:]]*$//' "$NOTES_TMP"
[[ -s $NOTES_TMP && -n $(tr -d '[:space:]' <"$NOTES_TMP") ]] || die "Release notes are empty — aborting."
NOTES_FILE=$NOTES_TMP
echo
printf '  %s┌─ release notes ─%s\n' "$C_MUTED" "$RST"
sed 's/^/  │ /' "$NOTES_FILE"
printf '  %s└─%s\n' "$C_MUTED" "$RST"

# ── channel / build mode / options ────────────────────────────────────────────
echo
if ! $PRERELEASE && $INTERACTIVE; then
    choose "Release channel" "Stable — becomes the 'latest' release users get from install.sh" "Pre-release — visible but not 'latest'"
    (( CHOICE_INDEX == 1 )) && PRERELEASE=true
fi

if [[ -z $MODE ]]; then
    local_hint="Linux .deb + .tar.gz here"; $CAN_BUILD_WIN && local_hint+=" + Windows .exe here" || local_hint+=" (no Windows .exe: cargo-xwin/makensis missing)"
    choose "Where should the artifacts be built?" \
        "GitHub Actions — recommended: real Windows runner builds the .exe, Linux .deb/.tar.gz too; watch it live from here" \
        "Local — $local_hint; publish immediately" \
        "Both — publish local artifacts now, then let Actions add its own"
    case $CHOICE_INDEX in 0) MODE=ci ;; 1) MODE=local ;; 2) MODE=both ;; esac
fi
[[ $MODE =~ ^(ci|local|both)$ ]] || die "--mode must be ci, local or both"
# phases so far: preflight, inputs; still to come: plan, bump, [build], push|done, [publish], [watch, finalize]
PHASE_TOTAL=4
[[ $MODE != ci ]] && PHASE_TOTAL=$((PHASE_TOTAL+1))
PHASE_TOTAL=$((PHASE_TOTAL+1))
if ! $NO_PUSH; then
    [[ $MODE != ci ]] && PHASE_TOTAL=$((PHASE_TOTAL+1))
    [[ $MODE != local ]] && PHASE_TOTAL=$((PHASE_TOTAL+2))
fi

if $INTERACTIVE; then
    if [[ $VERSION == "$OLD_VERSION" && -n $EXISTING_NOTES ]] && [[ $(cat "$NOTES_FILE") == "$EXISTING_NOTES" ]]; then
        UPDATE_CHANGELOG=false
    elif $UPDATE_CHANGELOG; then
        confirm "Write these notes into CHANGELOG.md under [$VERSION]?" y || UPDATE_CHANGELOG=false
    fi
    if $AUTO_NOTES; then
        confirm "Append GitHub's auto-generated 'What's Changed' commit list below the notes?" y || AUTO_NOTES=false
    fi
    if ! $DRAFT; then
        confirm "Publish immediately (no → leave as draft for a final look)?" y || DRAFT=true
    fi
fi

# ══════════════════════════════════════════════════════════════════════════════
#  Plan
# ══════════════════════════════════════════════════════════════════════════════
phase "Flight plan"
kv "Release"       "${BOLD}$TITLE${RST}"
kv "Tag"           "$TAG on $BRANCH → $REPO_SLUG"
kv "Version bump"  "$([[ $VERSION == "$OLD_VERSION" ]] && echo "none (already $OLD_VERSION)" || echo "$OLD_VERSION → $VERSION in Cargo.toml, package.json, tauri.conf.json, PKGBUILD, .spec, winget/scoop/choco, README, docs, SECURITY")"
kv "CHANGELOG.md"  "$($UPDATE_CHANGELOG && echo "add [$VERSION] section" || echo "unchanged")"
kv "Channel"       "$($PRERELEASE && echo pre-release || echo stable)$($DRAFT && echo ', draft')"
case $MODE in
    ci)    kv "Build" "GitHub Actions ($CI_WORKFLOW) — Linux .deb/.tar.gz + Windows .exe, watched live" ;;
    local) kv "Build" "this machine — .deb, .tar.gz$($CAN_BUILD_WIN && echo ', Windows .exe')$($SKIP_BUILD && echo ' (reusing target/, no rebuild)')" ;;
    both)  kv "Build" "local first ($($SKIP_BUILD && echo 'reusing target/' || echo 'fresh')), then GitHub Actions adds its artifacts" ;;
esac
kv "Auto notes"    "$($AUTO_NOTES && echo "append 'What's Changed'" || echo no)"
$NO_PUSH && kv "Push" "${C_WARN}disabled (--no-push): stops after the local tag${RST}"
echo
steps=("bump versions & lockfiles" "update CHANGELOG" "commit + annotated tag")
[[ $MODE != ci ]] && steps+=("build artifacts")
$NO_PUSH || steps+=("push branch + tag")
if ! $NO_PUSH; then
    [[ $MODE != ci ]] && steps+=("publish release + upload assets")
    [[ $MODE != local ]] && steps+=("watch GitHub Actions" "finalize release page")
fi
printf '  %s%s%s\n' "$C_MUTED" "$(printf '%s  →  ' "${steps[@]}" | sed 's/  →  $//')" "$RST"
echo
if ! $ASSUME_YES; then
    confirm "Take off?" y || die "Aborted — nothing was changed."
fi

# ══════════════════════════════════════════════════════════════════════════════
#  3 · Version bump + changelog
# ══════════════════════════════════════════════════════════════════════════════
# Anything failing between here and the push is undone on request: the tree was
# clean at start, so resetting to START_SHA only discards this script's edits.
rollback() {
    echo
    if confirm "Roll back the local release commit, edits and tag?" y; then
        git tag -d "$TAG" >/dev/null 2>&1 || true
        git reset -q --hard "$START_SHA"
        ok "Back at ${START_SHA:0:7}, tag $TAG removed. Nothing was pushed."
    else
        warn "Left as-is: HEAD is $(git rev-parse --short HEAD); tag $TAG (if created) is local only."
    fi
}
ON_STEP_FAIL=rollback

phase "Version bump"
TODAY=$(date +%F)
CHANGED_FILES=()

bump_versions() {
    local old=$1 new=$2 old_re=${1//./\\.}
    sed -i -E "0,/^version = \"$old_re\"/s//version = \"$new\"/" Cargo.toml
    sed -i -E "0,/\"version\": \"$old_re\"/s//\"version\": \"$new\"/" crates/linvclip-ui/package.json
    sed -i -E "0,/\"version\": \"$old_re\"/s//\"version\": \"$new\"/" crates/linvclip-ui/src-tauri/tauri.conf.json
    sed -i -E "s/^pkgver=$old_re\$/pkgver=$new/" packaging/PKGBUILD
    sed -i -E "s/^(Version:[[:space:]]+)$old_re\$/\1$new/" packaging/linvclipboard.spec
    local f
    for f in windows/publish/winget/*.yaml windows/publish/scoop.json \
             windows/publish/chocolatey/linvclipboard.nuspec windows/publish/chocolatey/tools/chocolateyInstall.ps1 \
             README.md docs/INSTALL.md SECURITY.md; do
        [[ -f $f ]] && sed -i -E "s/$old_re/$new/g" "$f"
    done
    sed -i -E "s/^ReleaseDate: .*/ReleaseDate: $TODAY/" windows/publish/winget/LinVClipBoard.installer.yaml
    # package-lock.json records the root package version twice
    node -e '
        const fs = require("fs"), p = process.argv[1], v = process.argv[2];
        const j = JSON.parse(fs.readFileSync(p, "utf8"));
        j.version = v; if (j.packages && j.packages[""]) j.packages[""].version = v;
        fs.writeFileSync(p, JSON.stringify(j, null, 2) + "\n");
    ' crates/linvclip-ui/package-lock.json "$new"
    # RPM %changelog entry
    RPM_DATE=$(LC_ALL=C date '+%a %b %d %Y') NEW="$new" HEAD="$HEADLINE" awk '
        { print }
        /^%changelog/ && !done {
            print "* " ENVIRON["RPM_DATE"] " LinVClipBoard Contributors <noreply@linvclipboard.dev> - " ENVIRON["NEW"] "-1"
            print "- " (ENVIRON["HEAD"] != "" ? ENVIRON["HEAD"] : "Release " ENVIRON["NEW"])
            done = 1
        }' packaging/linvclipboard.spec >packaging/linvclipboard.spec.new && mv packaging/linvclipboard.spec.new packaging/linvclipboard.spec
}

sync_lockfile() {
    cargo update --workspace --offline --quiet 2>/dev/null || cargo update --workspace --quiet
    cargo metadata --locked --no-deps --format-version 1 >/dev/null
}

changelog_upsert() {   # changelog_upsert VERSION DATE NOTES_FILE
    V="$1" D="$2" NOTES="$3" awk '
        function emit() {
            print "## [" ENVIRON["V"] "] - " ENVIRON["D"]; print ""
            while ((getline l < ENVIRON["NOTES"]) > 0) print l
            print ""; inserted = 1
        }
        /^## \[/ {
            if (!inserted) emit()
            if (index($0, "## [" ENVIRON["V"] "]") == 1) { skipping = 1; next } else skipping = 0
        }
        skipping { next }
        { print }
        END { if (!inserted) { print ""; emit() } }
    ' CHANGELOG.md >CHANGELOG.md.new && mv CHANGELOG.md.new CHANGELOG.md
}

if [[ $VERSION != "$OLD_VERSION" ]]; then
    run_step "Bump $OLD_VERSION → $VERSION in all synced files" bump_versions "$OLD_VERSION" "$VERSION"
    run_step "Sync Cargo.lock (--locked must still pass)" sync_lockfile
    CHANGED_FILES+=(Cargo.toml Cargo.lock crates/linvclip-ui/package.json crates/linvclip-ui/package-lock.json
        crates/linvclip-ui/src-tauri/tauri.conf.json packaging/PKGBUILD packaging/linvclipboard.spec
        windows/publish README.md docs/INSTALL.md SECURITY.md)
else
    info "Version already $VERSION — skipping the bump"
fi
if $UPDATE_CHANGELOG; then
    run_step "Update CHANGELOG.md → [$VERSION] - $TODAY" changelog_upsert "$VERSION" "$TODAY" "$NOTES_FILE"
    CHANGED_FILES+=(CHANGELOG.md)
fi

# ── commit + tag ──────────────────────────────────────────────────────────────
TAG_MSG="$DIST_DIR/tag-$VERSION.txt"
{ printf '%s\n\n' "$TITLE"; cat "$NOTES_FILE"; } >"$TAG_MSG"
COMMITTED=false
if (( ${#CHANGED_FILES[@]} )); then
    if $DRY_RUN; then
        run_step "Commit 'Release $TAG'" git commit -m "Release $TAG" -- "${CHANGED_FILES[@]}"
    else
        git add -- "${CHANGED_FILES[@]}"
        if [[ -n $(git diff --cached --name-only) ]]; then
            run_step "Commit 'Release $TAG' ($(git diff --cached --name-only | wc -l) files)" git commit -q -m "Release $TAG" -m "$TITLE"
            COMMITTED=true
        else
            info "Nothing to commit — files already at $VERSION"
        fi
    fi
fi
run_step "Create annotated tag $TAG" git tag -a "$TAG" -F "$TAG_MSG"


# ══════════════════════════════════════════════════════════════════════════════
#  4 · Local build
# ══════════════════════════════════════════════════════════════════════════════
ASSET_DIR="$DIST_DIR/$TAG"
ASSETS=()

build_locally() {
    phase "Build (local)"
    $DRY_RUN || { rm -rf "$ASSET_DIR"; mkdir -p "$ASSET_DIR"; }
    if ! $SKIP_BUILD; then
        run_step "npm ci (frontend deps)"                       bash -c "cd '$UI_DIR' && npm ci --no-audit --no-fund"
        run_step "cargo build --release clipd + clipctl"         cargo build --release --locked -p clipd -p clipctl
        run_step "tauri build linvclip-ui (frontend + binary)"   bash -c "cd '$UI_DIR' && npx tauri build --no-bundle -- --locked"
        run_step "Package .deb"                                  ./packaging/build-deb.sh --skip-build
        run_step "Package portable .tar.gz"                      ./packaging/build-tarball.sh
        if $CAN_BUILD_WIN; then
            run_step "cargo xwin build clipd.exe + clipctl.exe"  cargo xwin build --release --locked --target "$WIN_TARGET" -p clipd -p clipctl
            run_step "Stage Windows sidecars into Tauri resources" bash -c "cp 'target/$WIN_TARGET/release/clipd.exe' 'target/$WIN_TARGET/release/clipctl.exe' '$UI_DIR/src-tauri/resources/'"
            if [[ -n ${TAURI_SIGNING_PRIVATE_KEY:-} ]]; then
                run_step "tauri build NSIS installer (signed updater artifacts)" bash -c "cd '$UI_DIR' && npx tauri build --runner cargo-xwin --target '$WIN_TARGET' --bundles nsis -- --locked"
            else
                printf '{"bundle":{"createUpdaterArtifacts":false}}' >"$DIST_DIR/no-updater.json"
                warn "TAURI_SIGNING_PRIVATE_KEY not set — building the .exe without updater signature"
                run_step "tauri build NSIS installer" bash -c "cd '$UI_DIR' && npx tauri build --runner cargo-xwin --target '$WIN_TARGET' --bundles nsis --config '$DIST_DIR/no-updater.json' -- --locked"
            fi
        else
            info "Skipping Windows .exe here (needs cargo-xwin + makensis)$([[ $MODE == both ]] && echo ' — GitHub Actions will add it')"
        fi
    else
        info "--skip-build: reusing what is already in target/"
    fi

    $DRY_RUN && return 0
    shopt -s nullglob
    local f
    for f in target/debian/linvclipboard_"${VERSION}"-*_amd64.deb \
             target/tarball/linvclipboard-"${VERSION}"-linux-*.tar.gz \
             target/"$WIN_TARGET"/release/bundle/nsis/LinVClipBoard_"${VERSION}"_*-setup.exe \
             target/"$WIN_TARGET"/release/bundle/nsis/LinVClipBoard_"${VERSION}"_*-setup.exe.sig; do
        cp -f "$f" "$ASSET_DIR/"; ASSETS+=("$ASSET_DIR/$(basename "$f")")
    done
    shopt -u nullglob
    if (( ${#ASSETS[@]} == 0 )); then
        fail "No artifacts for $VERSION found under target/ — did the build produce a version other than $VERSION?"
        rollback; exit 1
    fi
    ( cd "$ASSET_DIR" && sha256sum -- * >SHA256SUMS )
    ASSETS+=("$ASSET_DIR/SHA256SUMS")
    echo
    printf '  %s%-52s %10s%s\n' "$C_MUTED" "artifact" "size" "$RST"
    for f in "${ASSETS[@]}"; do printf '  %s%-52s%s %s%10s%s\n' "$C_INFO" "$(basename "$f")" "$RST" "$C_MUTED" "$(fmt_size "$(stat -c %s "$f")")" "$RST"; done
}

[[ $MODE != ci ]] && build_locally

# ══════════════════════════════════════════════════════════════════════════════
#  5 · Push
# ══════════════════════════════════════════════════════════════════════════════
if $NO_PUSH; then
    phase "Done (local only)"
    ok "Commit and tag $TAG are ready locally. To continue by hand:"
    say "  git push origin $BRANCH && git push origin $TAG"
    exit 0
fi

phase "Push"
ON_STEP_FAIL=""     # past this point a failure must not rewind history
run_step "Push $BRANCH → origin"  git push --quiet origin "HEAD:refs/heads/$BRANCH"
run_step "Push tag $TAG → origin" git push --quiet origin "refs/tags/$TAG"
ok "Tag is live: https://github.com/$REPO_SLUG/releases/tag/$TAG (once published)"

# ══════════════════════════════════════════════════════════════════════════════
#  6 · Publish
# ══════════════════════════════════════════════════════════════════════════════
BODY_FILE="$DIST_DIR/body-$VERSION.md"
compose_body() {
    cp "$NOTES_FILE" "$BODY_FILE"
    if $AUTO_NOTES; then
        local prev gen
        prev=$(git tag --sort=-v:refname | grep -E '^v[0-9]' | grep -vx "$TAG" | head -1 || true)
        gen=$(gh api "repos/$REPO_SLUG/releases/generate-notes" -f tag_name="$TAG" ${prev:+-f previous_tag_name="$prev"} -q .body 2>/dev/null || true)
        if [[ -n $gen ]]; then printf '\n\n%s\n' "$gen" >>"$BODY_FILE"; fi
    fi
}

release_flags() {
    local -a f=(--title "$TITLE" --notes-file "$BODY_FILE")
    if $PRERELEASE; then f+=(--prerelease); elif ! $DRAFT; then f+=(--latest); fi
    printf '%s\n' "${f[@]}"
}

publish_local() {
    phase "Publish"
    run_step "Compose release body$($AUTO_NOTES && echo " (+ generated 'What's Changed')")" compose_body
    if $DRY_RUN; then
        run_step "gh release create $TAG" gh release create "$TAG" --title "$TITLE" --notes-file "$BODY_FILE" "${ASSETS[@]:-<assets>}"
        return
    fi
    local -a flags; mapfile -t flags < <(release_flags)
    if release_exists "$TAG"; then
        run_step "Update existing release $TAG" gh release edit "$TAG" --repo "$REPO_SLUG" "${flags[@]}" $($DRAFT && echo --draft)
        run_step "Upload ${#ASSETS[@]} assets (overwrite)" gh release upload "$TAG" --repo "$REPO_SLUG" --clobber "${ASSETS[@]}"
    else
        run_step "Create release $TAG with ${#ASSETS[@]} assets" gh release create "$TAG" --repo "$REPO_SLUG" --verify-tag "${flags[@]}" $($DRAFT && echo --draft) "${ASSETS[@]}"
    fi
}

# ── GitHub Actions watcher ────────────────────────────────────────────────────
watch_ci() {
    phase "GitHub Actions"
    if $DRY_RUN; then run_step "Watch workflow '$CI_WORKFLOW' for $TAG" gh run watch; return 0; fi
    local run_id="" tries=0
    while [[ -z $run_id ]]; do
        run_id=$(gh run list --repo "$REPO_SLUG" --workflow "$CI_WORKFLOW" --event push --branch "$TAG" --json databaseId -q '.[0].databaseId // empty' 2>/dev/null || true)
        [[ -n $run_id ]] && break
        (( tries++ >= 36 )) && die "No '$CI_WORKFLOW' run appeared for $TAG in 3 minutes — check https://github.com/$REPO_SLUG/actions"
        spin_wait 5 "Waiting for GitHub Actions to pick up $TAG…"
    done
    local url; url=$(gh run view "$run_id" --repo "$REPO_SLUG" --json url -q .url)
    info "Run #$run_id → ${UL}$url${RST}"
    echo

    local start=$SECONDS last_fetch=-100 json="" status="" conclusion="" lines=0 i=0 now
    local name jstatus jconc started completed icon color dur
    hide_cursor
    while :; do
        if (( SECONDS - last_fetch >= 8 )); then
            json=$(gh run view "$run_id" --repo "$REPO_SLUG" --json status,conclusion,jobs 2>/dev/null || echo "$json")
            last_fetch=$SECONDS
            status=$(jq -r '.status // ""' <<<"$json")
            conclusion=$(jq -r '.conclusion // ""' <<<"$json")
        fi
        cursor_up "$lines"; clear_below
        lines=0
        printf '  %s%s%s %s%s%s  %s%s elapsed%s\n' "$C_ACC" "${FRAMES[i % ${#FRAMES[@]}]}" "$RST" "$BOLD" "${status:-connecting}" "$RST" "$C_MUTED" "$(fmt_dur $((SECONDS-start)))" "$RST"
        lines=$((lines+1))
        while IFS=$'\t' read -r name jstatus jconc started completed; do
            [[ -z $name ]] && continue
            case "$jstatus/$jconc" in
                completed/success)   icon="✔" color=$C_OK ;;
                completed/failure)   icon="✖" color=$C_ERR ;;
                completed/cancelled) icon="⊘" color=$C_WARN ;;
                completed/skipped)   icon="○" color=$C_MUTED ;;
                completed/*)         icon="●" color=$C_WARN ;;
                in_progress/*)       icon="${FRAMES[i % ${#FRAMES[@]}]}" color=$C_ACC ;;
                *)                   icon="◌" color=$C_MUTED ;;
            esac
            dur=""
            if [[ -n $started && $started != null ]]; then
                now=$(date +%s); [[ -n $completed && $completed != null ]] && now=$(date -d "$completed" +%s)
                dur=$(fmt_dur $(( now - $(date -d "$started" +%s) )))
            fi
            printf '    %s%s%s %-40s %s%8s%s\n' "$color" "$icon" "$RST" "${name:0:40}" "$C_MUTED" "$dur" "$RST"
            lines=$((lines+1))
        done < <(jq -r '.jobs[]? | [.name, .status, (.conclusion // ""), (.startedAt // ""), (.completedAt // "")] | @tsv' <<<"$json")
        [[ $status == completed ]] && break
        if (( SECONDS - start > CI_TIMEOUT_MIN * 60 )); then show_cursor; die "Gave up after $CI_TIMEOUT_MIN minutes — the run is still going at $url"; fi
        i=$((i+1)); sleep 0.15
    done
    show_cursor
    echo
    if [[ $conclusion == success ]]; then
        ok "Workflow succeeded in $(fmt_dur $((SECONDS-start)))"
        return 0
    fi
    fail "Workflow finished with: $conclusion"
    jq -r '.jobs[] | select(.conclusion == "failure") | "    ✖ " + .name' <<<"$json" | sed "s/^/$C_ERR/; s/\$/$RST/"
    say "${C_MUTED}Failed-step logs: gh run view $run_id --log-failed${RST}"
    if confirm "Re-run the failed jobs and keep watching?" y; then
        gh run rerun "$run_id" --repo "$REPO_SLUG" --failed >/dev/null
        spin_wait 6 "Re-queued…"
        watch_ci_again "$run_id"; return $?
    fi
    return 1
}
watch_ci_again() { PHASE_N=$((PHASE_N-1)); watch_ci; }

finalize_release() {
    phase "Finalize release page"
    if $DRY_RUN; then run_step "gh release edit $TAG --title … --notes-file …" gh release edit "$TAG"; return; fi
    local tries=0
    until release_exists "$TAG"; do
        (( tries++ >= 24 )) && die "The workflow finished but no release for $TAG exists — check the 'Release' job logs."
        spin_wait 5 "Waiting for the release job to publish $TAG…"
    done
    run_step "Compose release body$($AUTO_NOTES && echo " (+ generated 'What's Changed')")" compose_body
    local -a flags; mapfile -t flags < <(release_flags)
    run_step "Set title, notes and channel on $TAG" gh release edit "$TAG" --repo "$REPO_SLUG" "${flags[@]}" $($DRAFT && echo --draft || echo --draft=false)
}

show_release() {
    $DRY_RUN && return 0
    local json; json=$(gh release view "$TAG" --repo "$REPO_SLUG" --json url,name,isDraft,isPrerelease,assets 2>/dev/null) || return 0
    echo
    rule
    printf '  %s%s🎉  %s%s\n' "$BOLD" "$C_OK" "$(jq -r .name <<<"$json")" "$RST"
    printf '  %s%s%s%s\n' "$UL" "$C_INFO" "$(jq -r .url <<<"$json")" "$RST"
    printf '  %s%s%s\n' "$C_MUTED" "$(jq -r '(if .isDraft then "draft · " else "" end) + (if .isPrerelease then "pre-release" else "stable (latest)" end)' <<<"$json")" "$RST"
    echo
    local has_exe=false has_linux=false
    while IFS=$'\t' read -r n s; do
        [[ $n == *.exe ]] && has_exe=true
        [[ $n == *.deb || $n == *.tar.gz || $n == *.rpm ]] && has_linux=true
        printf '    %s▸%s %-52s %s%10s%s\n' "$C_PRI" "$RST" "$n" "$C_MUTED" "$(fmt_size "$s")" "$RST"
    done < <(jq -r '.assets[] | [.name, .size] | @tsv' <<<"$json")
    echo
    $has_linux || warn "No Linux package attached yet."
    $has_exe   || warn "No Windows .exe attached yet$([[ $MODE == local ]] && echo " — run with --mode ci, or push-trigger CI, to add it")."
    rule
}

case $MODE in
    local)
        publish_local
        show_release
        ;;
    ci)
        watch_ci || exit 1
        finalize_release
        show_release
        ;;
    both)
        publish_local
        watch_ci || { show_release; exit 1; }
        finalize_release
        show_release
        ;;
esac

echo
ok "Release $TAG complete."
$DRY_RUN && info "Dry run — nothing was changed."
exit 0
