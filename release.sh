#!/bin/bash
set -euo pipefail

fail() {
  echo "$@"
  exit 2
}

git-tag() {
  [ -z "$(git ls-files . --exclude-standard --others)" ] || fail "remove untracked files first"
  git commit -am "${VERSION}"
  git tag -a "${VERSION}" -m "${VERSION}"
  git push
  git push --tags
}


bump-docs() {
  sed -i "s/${OLDVER}/${VERSION}/g" README.md
  sed -i "s/${OLDVER}/${VERSION}/g" kube-derive/README.md
}

main() {
  # Current master version is always the first one in kube/Cargo.toml
  local -r VERSION="$(grep version kube/Cargo.toml | awk -F"\"" '{print $2}' | head -n 1)"
  local -r OLDVER="$(grep "kube =" README.md | head -n 1 | awk -F"\"" '{print $2}')"

  # bumping stage
  if [[ "${VERSION}" != "${OLDVER}" ]]; then
    [ -z "$(git status --porcelain)" ] || fail "deal with changes first"
    echo "Bumping from ${OLDVER} -> ${VERSION}"
    bump-docs
    git-tag
    git diff
    exit 0
  fi

}

# Usage:
#
# cargo release minor --exclude tests --exclude examples --skip-tag --skip-push --no-dev-version
# TODO: --consolidate-commits
# Then amend commits / squash.
# Finally run this script to bump readme's and then tag.
#
# ./release.sh
#
# shellcheck disable=SC2068
main $@
