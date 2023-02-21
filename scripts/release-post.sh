#!/bin/bash
set -euo pipefail

main() {
  cd "$(dirname "${BASH_SOURCE[0]}")" && cd .. # aka $WORKSPACE_ROOT
  local -r CURRENT_VER="$(rg 'kube = \{ version = "(\S*)"' -or '$1' README.md | head -n1)"
  git tag -a "${CURRENT_VER}" -m "${CURRENT_VER}"
  git push
  git push --tags
}

# post release script run manually after cargo-release
# shellcheck disable=SC2068
main $@
