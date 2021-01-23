#!/bin/bash
set -euo pipefail

main() {
  local -r CURRENT_VER="$(rg "kube =" README.md | head -n 1 | awk -F"\"" '{print $2}')"
  git tag -a "${CURRENT_VER}" -m "${CURRENT_VER}"
  git push
  git push --tags
}

# post release script run manually after cargo-release
# shellcheck disable=SC2068
main $@
