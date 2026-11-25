#!/usr/bin/env bash
#
# Build the Timberbot C# mod against a local Timberborn 1.1 Managed/ directory
# and package ONLY Timberbot's own GPL-3.0 artifacts into:
#
#   dist/timberbot-mod-<version>.zip
#
# The archive contains a top-level `Timberbot/` folder ready to extract
# directly into Timberborn's `Mods/` directory or upload to a GitHub Release.
# No Timberborn, Unity, Bindito, or other Managed/ game DLLs are ever included.
#
# Usage:
#   scripts/package-mod.sh --game-managed-dir "/path/to/Timberborn_Data/Managed"
#   scripts/package-mod.sh --game-managed-dir "/path/to/Managed" --out-dir dist
#   GAME_MANAGED_DIR="/path/to/Managed" scripts/package-mod.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="$REPO_ROOT/timberbot/src"
CSPROJ="$SRC_DIR/Timberbot.csproj"
MANIFEST="$SRC_DIR/manifest.json"

GAME_MANAGED_DIR="${GAME_MANAGED_DIR:-}"
OUT_DIR="$REPO_ROOT/dist"
VERSION_OVERRIDE=""
SKIP_BUILD=0

usage() {
    cat <<'EOF'
Usage: scripts/package-mod.sh [OPTIONS]

Build Timberbot.csproj against a local Timberborn Managed/ folder and package
ONLY Timberbot's own GPL-3.0 artifacts into timberbot-mod-<version>.zip.

Options:
  --game-managed-dir <PATH>  Path to Timberborn_Data/Managed (or env GAME_MANAGED_DIR)
  --out-dir <DIR>            Output directory for the ZIP (default: ./dist)
  --version <VER>            Override version from timberbot/src/manifest.json
  --skip-build               Package an already-built Release Timberbot.dll
  -h, --help                 Show this help message
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --game-managed-dir)
            GAME_MANAGED_DIR="${2:-}"
            shift 2
            ;;
        --out-dir)
            OUT_DIR="${2:-}"
            shift 2
            ;;
        --version)
            VERSION_OVERRIDE="${2:-}"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "package-mod: unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [[ ! -f "$CSPROJ" || ! -f "$MANIFEST" ]]; then
    echo "package-mod: missing $CSPROJ or $MANIFEST" >&2
    exit 1
fi

# Auto-detect common Steam Managed/ paths on macOS/Linux when not provided
if [[ -z "$GAME_MANAGED_DIR" && "$SKIP_BUILD" -eq 0 ]]; then
    CANDIDATES=(
        "$HOME/Library/Application Support/Steam/steamapps/common/Timberborn/Timberborn.app/Contents/Resources/Data/Managed"
        "$HOME/.local/share/Steam/steamapps/common/Timberborn/Timberborn_Data/Managed"
        "$HOME/.steam/steam/steamapps/common/Timberborn/Timberborn_Data/Managed"
    )
    for cand in "${CANDIDATES[@]}"; do
        if [[ -d "$cand" ]]; then
            GAME_MANAGED_DIR="$cand"
            break
        fi
    done
fi

if [[ "$SKIP_BUILD" -eq 0 ]]; then
    if [[ -z "$GAME_MANAGED_DIR" || ! -d "$GAME_MANAGED_DIR" ]]; then
        echo "package-mod: Timberborn Managed/ directory not found." >&2
        echo "Pass --game-managed-dir \"/path/to/Timberborn_Data/Managed\" (or set GAME_MANAGED_DIR)." >&2
        exit 1
    fi
    if [[ ! -f "$GAME_MANAGED_DIR/Timberborn.Common.dll" ]]; then
        echo "package-mod: $GAME_MANAGED_DIR does not contain Timberborn.Common.dll" >&2
        exit 1
    fi
fi

# Read version from manifest.json unless overridden
VERSION="$VERSION_OVERRIDE"
if [[ -z "$VERSION" ]]; then
    if command -v python3 >/dev/null 2>&1; then
        VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["Version"])' "$MANIFEST")"
    else
        VERSION="$(sed -n 's/.*"Version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$MANIFEST" | head -n 1)"
    fi
fi
if [[ -z "$VERSION" ]]; then
    echo "package-mod: failed to read Version from $MANIFEST" >&2
    exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

BUILD_DEPLOY_STAGE="$WORK_DIR/msbuild-deploy"
ZIP_STAGE="$WORK_DIR/package"
MOD_STAGE="$ZIP_STAGE/Timberbot"
mkdir -p "$BUILD_DEPLOY_STAGE" "$MOD_STAGE/docs" "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"

if [[ "$SKIP_BUILD" -eq 0 ]]; then
    echo "package-mod: building Release against GameManagedDir=$GAME_MANAGED_DIR"
    dotnet build "$CSPROJ" \
        -c Release \
        -p:GameManagedDir="$GAME_MANAGED_DIR" \
        -p:ModDir="$BUILD_DEPLOY_STAGE"
fi

MOD_DLL="$SRC_DIR/bin/Release/netstandard2.1/Timberbot.dll"
if [[ ! -f "$MOD_DLL" ]]; then
    echo "package-mod: expected built DLL at $MOD_DLL" >&2
    exit 1
fi

# Explicit GPL-3.0 allowlist: copy ONLY Timberbot's own artifacts (never Managed/ game DLLs)
cp "$MOD_DLL" "$MOD_STAGE/Timberbot.dll"
cp "$MANIFEST" "$MOD_STAGE/manifest.json"
cp "$SRC_DIR/settings.json" "$MOD_STAGE/settings.json"
if [[ -f "$SRC_DIR/thumbnail.png" ]]; then
    cp "$SRC_DIR/thumbnail.png" "$MOD_STAGE/thumbnail.png"
fi
if [[ -f "$REPO_ROOT/LICENSE" ]]; then
    cp "$REPO_ROOT/LICENSE" "$MOD_STAGE/LICENSE"
fi

DOC_FILES=(
    api-reference.md
    connect-and-play.md
    developing.md
    events.md
    features.md
    getting-started.md
    timberbot.md
    websocket-protocol.md
)
for doc in "${DOC_FILES[@]}"; do
    if [[ -f "$REPO_ROOT/docs/$doc" ]]; then
        cp "$REPO_ROOT/docs/$doc" "$MOD_STAGE/docs/$doc"
    fi
done

# Explicit safety audit: refuse to package if any DLL other than Timberbot.dll is present
DLL_LIST=()
while IFS= read -r dll_file; do
    DLL_LIST+=("$dll_file")
done < <(find "$ZIP_STAGE" -type f -iname '*.dll' | sort)

if [[ "${#DLL_LIST[@]}" -ne 1 || "$(basename "${DLL_LIST[0]}")" != "Timberbot.dll" ]]; then
    echo "package-mod: SAFETY CHECK FAILED — expected ONLY Timberbot.dll, found:" >&2
    printf '  %s\n' "${DLL_LIST[@]}" >&2
    exit 1
fi

ZIP_PATH="$OUT_DIR/timberbot-mod-${VERSION}.zip"
rm -f "$ZIP_PATH"

if command -v zip >/dev/null 2>&1; then
    (cd "$ZIP_STAGE" && zip -qr "$ZIP_PATH" Timberbot)
elif command -v python3 >/dev/null 2>&1; then
    python3 - "$ZIP_STAGE" "$ZIP_PATH" <<'PY'
import os, sys, zipfile
root, out = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for dirpath, _, filenames in os.walk(root):
        for fn in sorted(filenames):
            full = os.path.join(dirpath, fn)
            rel = os.path.relpath(full, root)
            zf.write(full, rel)
PY
else
    echo "package-mod: need either 'zip' or 'python3' to create $ZIP_PATH" >&2
    exit 1
fi

echo "package-mod: created $ZIP_PATH"
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$ZIP_PATH"
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$ZIP_PATH"
fi

echo "package-mod: contents (verified: ONLY Timberbot.dll, zero game Managed/ DLLs):"
if command -v unzip >/dev/null 2>&1; then
    unzip -l "$ZIP_PATH"
elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import sys,zipfile; [print(" ", n) for n in sorted(zipfile.ZipFile(sys.argv[1]).namelist())]' "$ZIP_PATH"
fi

cat <<EOF

Next step (optional GitHub Release upload):
  gh release create v${VERSION} "${ZIP_PATH}" \\
    --title "Timberbot Mod v${VERSION}" \\
    --notes "GPL-3.0 Timberbot C# mod built against Timberborn 1.1 Managed/ DLLs (excludes all game DLLs)."
EOF
