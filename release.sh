#!/bin/bash
set -euo pipefail

git-tag() {
  git diff --exit-code || return 1
  git diff --cached --exit-code || return 1
  # TODO: check no stray files (cargo publish dislikes it)
  git tag -a "${VERSION}" -m "${VERSION}"
  git push
  git push --tags
}

bump-files() {
  sed -i "s/${VERSION}/${NEWVER}/g" README.md
  sed -i "0,/version/s/version = \".*\"/version = \"${NEWVER}\"/" kube/Cargo.toml
  sed -i "0,/version/s/version = \".*\"/version = \"${NEWVER}\"/" kube-derive/Cargo.toml
}

main() {
  # Current master version is always the first one in kube/Cargo.toml
  local -r VERSION="$(grep version kube/Cargo.toml | awk -F"\"" '{print $2}' | head -n 1)"
  local -r SEMVER=(${VERSION//./ }) # <- parse subset of semver here
  # NB: can maybe use cargo-bump in the future  if it starts supporting workspaces

  local -r mode="$1"
  if [[ "${mode}" == "major" ]]; then
    local -r NEWVER="$((SEMVER[0]+1)).${SEMVER[1]}.${SEMVER[2]}"
  elif [[ "${mode}" == "minor" ]]; then
    local -r NEWVER="${SEMVER[0]}.$((SEMVER[1]+1)).${SEMVER[2]}"
  elif [[ "${mode}" == "patch" ]]; then
    local -r NEWVER="${SEMVER[0]}.${SEMVER[1]}.$((SEMVER[2]+1))"
  fi

  # bumping something:
  if [[ "${VERSION}" != "${NEWVER}" ]]; then
    echo "Bumping from ${VERSION} -> ${NEWVER}"
    bump-files
    git diff
    exit 0
  fi

  # release
  if [[ "${mode}" == "release" ]]; then
    if ! git diff --name-only origin/master ./*/Cargo.toml; then
      echo "Please run \"./release.sh minor\" first"
      exit 2
    fi
    git commit -am "${VERSION}"
    git-tag
    echo "Please commit and run script with tag mode"
    if git diff --name-only --origin/master kube/Cargo.toml; then
      echo "Please cargo publish inside kube/"
    fi
    if git diff --name-only --origin/master kube-derive/Cargo.toml; then
      echo "Please cargo publish inside kube-derive/"
    fi
  fi
}

# Usage: 3 stage (for now)
# ./release.sh minor
# ./release.sh release
# cargo publish (in required subdirectories)

# shellcheck disable=SC2068
main $@
