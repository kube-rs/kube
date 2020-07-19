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

publish-changed-crate() {
  local -r crate="$1"
  if git diff --name-only origin/master "${crate}/Cargo.toml"; then
    (cd "$crate"; cargo publish)
  fi
}

publish() {
  git-tag
  publish-changed-crate kube
  publish-changed-crate kube-derive
  # TODO: change kube-derive to pin a version of kube
  #publish-changed-crate kube-runtime
}

bump-files() {
  sed -i "s/${VERSION}/${NEWVER}/g" README.md
  sed -i "0,/version/s/version = \".*\"/version = \"${NEWVER}\"/" kube/Cargo.toml
  sed -i "0,/version/s/version = \".*\"/version = \"${NEWVER}\"/" kube-derive/Cargo.toml
 # sed -i "0,/version/s/version = \".*\"/version = \"${NEWVER}\"/" kube-runtime/Cargo.toml
}

main() {
  # Current master version is always the first one in kube/Cargo.toml
  local -r VERSION="$(grep version kube/Cargo.toml | awk -F"\"" '{print $2}' | head -n 1)"
  local -r SEMVER=(${VERSION//./ }) # <- parse subset of semver here
  # NB: can maybe use cargo-bump in the future  if it starts supporting workspaces
  local NEWVER="" # set if we are bumping

  local -r mode="$1"
  if [[ "${mode}" == "major" ]]; then
    NEWVER="$((SEMVER[0]+1)).${SEMVER[1]}.${SEMVER[2]}"
  elif [[ "${mode}" == "minor" ]]; then
    NEWVER="${SEMVER[0]}.$((SEMVER[1]+1)).${SEMVER[2]}"
  elif [[ "${mode}" == "patch" ]]; then
    NEWVER="${SEMVER[0]}.${SEMVER[1]}.$((SEMVER[2]+1))"
  fi

  # bumping something:
  if [ -n "${NEWVER}" ]; then
    [ -z "$(git status --porcelain)" ] || fail "deal with changes first"
    echo "Bumping from ${VERSION} -> ${NEWVER}"
    bump-files
    git diff
    exit 0
  fi

  # publish
  if [[ "${mode}" == "publish" ]]; then
    [ -n "$(git diff --name-only origin/master ./*/Cargo.toml)" ] || fail "./release.sh minor" first
    publish
  fi
}

# Usage: 2 stage
#
# ./release.sh minor
#
# Then check git output, and:
#
# ./release.sh publish
#
# shellcheck disable=SC2068
main $@
