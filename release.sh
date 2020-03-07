#!/bin/bash
set -euxo pipefail

# WIP release script

git-tag() {
  git diff --exit-code || return 1
  git diff --cached --exit-code || return 1
  # TODO: check no stray files!
  local -r ver="$(grep version kube/Cargo.toml | awk -F"\"" '{print $2}' | head -n 1)"
  git tag -a "${ver}" -m "${ver}"
  git push
  git push --tags
}

# TODO: cargo-bump does not work with workspaces atm
bump-toml() {
  cargo bump -p kube-derive minor
  cargo bump -p kube minor
}

cargo-publish() {
  cd kube-derive && cargo publish
  cd kube && cargo publish
}
