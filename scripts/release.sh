#!/usr/bin/env bash
# Cut a release:
#   1. verify the working tree is clean and the tag doesn't exist
#   2. run the full test suite
#   3. set the workspace version in app/Cargo.toml
#   4. commit, tag vX.Y.Z, and push
# GitHub Actions (.github/workflows/release.yml) then builds Windows, Linux,
# and macOS executables and publishes them to GitHub Releases.
#
# usage: scripts/release.sh <version>     e.g. scripts/release.sh 1.1.0
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:-}"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "usage: $0 <version>   e.g. $0 1.1.0" >&2
  CURRENT=$(perl -0ne 'print $1 if /\[workspace\.package\].*?version = "([^"]+)"/s' app/Cargo.toml)
  echo "current version: ${CURRENT:-unknown}" >&2
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "error: working tree is not clean — commit or stash first" >&2
  exit 1
fi
if git rev-parse -q --verify "refs/tags/v$VERSION" >/dev/null; then
  echo "error: tag v$VERSION already exists" >&2
  exit 1
fi

echo "==> Running tests"
(cd app && cargo test --workspace)

echo "==> Setting version to $VERSION"
perl -0pi -e 's/(\[workspace\.package\].*?version = ")[^"]+(")/${1}'"$VERSION"'${2}/s' app/Cargo.toml
(cd app && cargo check --quiet)   # sync Cargo.lock with the new version

git add app/Cargo.toml app/Cargo.lock
git commit -m "Release v$VERSION"
git tag "v$VERSION"

echo "==> Pushing"
git push origin HEAD "v$VERSION"

echo
echo "Tag v$VERSION pushed. GitHub Actions is now building Windows, Linux, and"
echo "macOS executables and will publish them to GitHub Releases."
echo "Watch progress under the repository's Actions tab."
