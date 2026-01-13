#!/bin/bash
# Descartes Release Script
# Usage: ./scripts/release.sh <version>

set -e

VERSION=$1
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CRATE_DIR="$(dirname "$SCRIPT_DIR")"

if [ -z "$VERSION" ]; then
    echo "Usage: ./scripts/release.sh <version>"
    echo "Example: ./scripts/release.sh 0.2.0"
    exit 1
fi

echo "=== Releasing Descartes v$VERSION ==="

cd "$CRATE_DIR"

# 1. Check for uncommitted changes
if ! git diff --quiet; then
    echo "ERROR: Uncommitted changes detected. Commit or stash them first."
    exit 1
fi

# 2. Check scud dependency is NOT using path
if grep -q 'path = "../../scud' Cargo.toml; then
    echo "ERROR: scud dependency still uses path!"
    echo ""
    echo "For publishing, change:"
    echo '  scud = { package = "scud-cli", path = "../../scud/scud-cli" }'
    echo "To:"
    echo '  scud = { package = "scud-cli", version = "1.34" }'
    echo ""
    echo "After publishing, you can revert to the path dependency."
    exit 1
fi

# 3. Update version in Cargo.toml
echo "Updating version to $VERSION..."
sed -i '' "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# 4. Build and test
echo "Building..."
cargo build --release

echo "Running tests..."
cargo test

# 5. Dry run publish
echo "Dry run publish..."
cargo publish --dry-run

# 6. Confirm
echo ""
echo "Ready to publish descartes v$VERSION to crates.io"
read -p "Continue? [y/N] " confirm
if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
    echo "Aborted."
    git checkout Cargo.toml
    exit 1
fi

# 7. Publish
echo "Publishing..."
cargo publish

# 8. Commit and tag
git add Cargo.toml Cargo.lock
git commit -m "chore: release descartes v$VERSION"
git tag "descartes-v$VERSION"

echo ""
echo "=== Successfully published descartes v$VERSION ==="
echo ""
echo "Don't forget to:"
echo "  git push && git push --tags"
echo ""
echo "After pushing, you may want to revert scud to path dependency for development:"
echo '  scud = { package = "scud-cli", path = "../../scud/scud-cli" }'
