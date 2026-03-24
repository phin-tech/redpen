#!/usr/bin/env bash
set -euo pipefail

# Update the phin-tech/homebrew-tap repo with the latest release assets.
# Usage: ./scripts/update-homebrew-tap.sh [version]
# If version is omitted, reads from src-tauri/tauri.conf.json

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [ -n "${1:-}" ]; then
  VERSION="$1"
else
  VERSION=$(python3 -c 'import json; print(json.load(open("'"$REPO_ROOT"'/src-tauri/tauri.conf.json"))["version"])')
fi

TAG="v${VERSION}"
echo "Updating homebrew tap for ${TAG}..."

# --- Download release assets and compute SHA256 ---

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

CLI_TARBALL="redpen-${VERSION}-aarch64-apple-darwin.tar.gz"
DMG="Red Pen_${VERSION}_aarch64.dmg"

echo "Downloading release assets..."
gh release download "$TAG" \
  --pattern "$CLI_TARBALL" \
  --pattern "$DMG" \
  --dir "$TMPDIR" \
  --repo phin-tech/redpen

CLI_SHA=$(shasum -a 256 "$TMPDIR/$CLI_TARBALL" | awk '{print $1}')
DMG_SHA=$(shasum -a 256 "$TMPDIR/$DMG" | awk '{print $1}')

echo "CLI SHA256: $CLI_SHA"
echo "DMG SHA256: $DMG_SHA"

# --- Clone or update tap repo ---

TAP_DIR="$TMPDIR/homebrew-tap"
echo "Cloning homebrew-tap..."
gh repo clone phin-tech/homebrew-tap "$TAP_DIR" 2>/dev/null || {
  echo "ERROR: phin-tech/homebrew-tap does not exist."
  echo "Create it first: gh repo create phin-tech/homebrew-tap --public"
  exit 1
}

mkdir -p "$TAP_DIR/Formula" "$TAP_DIR/Casks"

# --- Generate Formula (CLI) ---

cat > "$TAP_DIR/Formula/redpen.rb" <<RUBY
class Redpen < Formula
  desc "CLI for Red Pen code review annotations"
  homepage "https://github.com/phin-tech/redpen"
  version "${VERSION}"
  license "MIT"

  url "https://github.com/phin-tech/redpen/releases/download/v#{version}/redpen-#{version}-aarch64-apple-darwin.tar.gz"
  sha256 "${CLI_SHA}"

  depends_on arch: :arm64

  def install
    bin.install "redpen"
  end

  test do
    assert_match "redpen", shell_output("#{bin}/redpen --help")
  end
end
RUBY

echo "Wrote Formula/redpen.rb"

# --- Generate Cask (Desktop App) ---

cat > "$TAP_DIR/Casks/red-pen.rb" <<RUBY
cask "red-pen" do
  version "${VERSION}"
  sha256 "${DMG_SHA}"

  url "https://github.com/phin-tech/redpen/releases/download/v#{version}/Red%20Pen_#{version}_aarch64.dmg"
  name "Red Pen"
  desc "Code review annotation tool"
  homepage "https://github.com/phin-tech/redpen"

  depends_on arch: :arm64
  depends_on macos: ">= :ventura"

  app "Red Pen.app"

  zap trash: [
    "~/Library/Application Support/com.redpen.app",
    "~/Library/Caches/com.redpen.app",
  ]
end
RUBY

echo "Wrote Casks/red-pen.rb"

# --- Commit and push ---

cd "$TAP_DIR"
git add Formula/redpen.rb Casks/red-pen.rb
if git diff --cached --quiet; then
  echo "No changes to commit — tap is already up to date."
  exit 0
fi

git commit -m "Update to ${TAG}"
git push origin main

echo "Homebrew tap updated to ${TAG}"
echo ""
echo "Users can install with:"
echo "  brew tap phin-tech/tap"
echo "  brew install redpen           # CLI"
echo "  brew install --cask red-pen   # Desktop app"
