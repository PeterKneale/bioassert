#!/usr/bin/env bash
# Regenerate the Homebrew formula for a bioassert release.
#
# Usage: update-homebrew-formula.sh <version> <source-repo> <formula-path>
#   <version>      release version without the leading v, e.g. 3.1.2
#   <source-repo>  owner/repo holding the release assets, e.g. PeterKneale/bioassert
#   <formula-path> path to bioassert.rb in a checked-out copy of the tap
#
# Reads the four published <asset>.sha256 files from the release and writes a
# fresh prebuilt-binary formula. Requires the gh CLI (GH_TOKEN in CI).
set -euo pipefail

version="${1:?version required}"
repo="${2:?source repo required}"
formula="${3:?formula path required}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

gh release download "v${version}" --repo "$repo" --dir "$tmp" --pattern 'bioassert-*.sha256'

sha() { awk '{print $1}' "$tmp/bioassert-${version}-$1.sha256"; }
macos_arm64="$(sha macos-arm64)"
macos_x86_64="$(sha macos-x86_64)"
linux_x86_64="$(sha linux-x86_64)"
linux_arm64="$(sha linux-arm64)"

base="https://github.com/${repo}/releases/download/v${version}"

cat > "$formula" <<EOF
class Bioassert < Formula
  desc "CLI tool for asserting properties of files in bioinformatics pipelines"
  homepage "https://github.com/${repo}"
  version "${version}"
  license "MIT"

  on_macos do
    on_arm do
      url "${base}/bioassert-${version}-macos-arm64.tar.gz"
      sha256 "${macos_arm64}"
    end
    on_intel do
      url "${base}/bioassert-${version}-macos-x86_64.tar.gz"
      sha256 "${macos_x86_64}"
    end
  end

  on_linux do
    on_intel do
      url "${base}/bioassert-${version}-linux-x86_64.tar.gz"
      sha256 "${linux_x86_64}"
    end
    on_arm do
      url "${base}/bioassert-${version}-linux-arm64.tar.gz"
      sha256 "${linux_arm64}"
    end
  end

  def install
    bin.install "bioassert"
  end

  test do
    assert_match "bioassert #{version}", shell_output("#{bin}/bioassert --version")
  end
end
EOF

echo "Wrote ${formula} for v${version}"
