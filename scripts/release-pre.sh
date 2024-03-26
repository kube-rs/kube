#!/bin/bash
set -euo pipefail

replace-docs() {
  # Swap UNRELEASED header with a versioned and dated one, and remove compare url in it
  sd "UNRELEASED" "${NEW_VERSION} / $(date +%Y-%m-%d)" CHANGELOG.md
  sd " \* see https://github.com/kube-rs/kube/compare/.*...main\n" "" CHANGELOG.md
  # Create a new UNRELEASED header, and add compare url to it
  sd "<!-- next-header -->" "<!-- next-header -->\nUNRELEASED\n===================\n * see https://github.com/kube-rs/kube/compare/${NEW_VERSION}...main\n" CHANGELOG.md
  # Replace all space-prefixed issue links with a dumb one to this repo
  # This may link to an issue when it's a pull, but github redirects
  # shellcheck disable=SC2016
  sd ' \#(\d+)' ' [#$1](https://github.com/kube-rs/kube/issues/$1)' CHANGELOG.md
  sd "${PREV_VERSION}" "${NEW_VERSION}" kube-derive/README.md
  sd "${PREV_VERSION}" "${NEW_VERSION}" README.md
}

sanity() {
  CARGO_TREE_OPENAPI="$(cargo tree -i k8s-openapi --depth=0 -e=normal | choose 1)"
  USED_K8S_OPENAPI="${CARGO_TREE_OPENAPI:1}"
  RECOMMENDED_K8S_OPENAPI="$(rg "k8s-openapi =" README.md | head -n 1)" # only check first instance
  if ! [[ $RECOMMENDED_K8S_OPENAPI =~ $USED_K8S_OPENAPI ]]; then
    echo "prerelease: abort: recommending k8s-openapi pinned to a different version to what we use"
    echo "${RECOMMENDED_K8S_OPENAPI} vs. used: ${USED_K8S_OPENAPI}"
    exit 1
  fi
  # TODO: verify versions of tools for release?
}

main() {
  # We only want this to run ONCE at workspace level
  cd "$(dirname "${BASH_SOURCE[0]}")" && cd .. # aka $WORKSPACE_ROOT
  local -r CURRENT_VER="$(rg 'kube = \{ version = "(\S*)"' -or '$1' README.md | head -n1)"

  # If the main README has been bumped, assume we are done:
  if [[ "${NEW_VERSION}" = "${CURRENT_VER}" ]]; then
    echo "prerelease: ${CRATE_NAME} nothing to do"
  else
    echo "prerelease: ${CRATE_NAME} bumping docs from ${PREV_VERSION} -> ${NEW_VERSION}"
    sanity
    replace-docs
  fi
  exit 0
}

# helper hook run by cargo-release before cargo publish
#
# This is actually invoked from every member crate before their publishes.
# Evars: PREV_VERSION + NEW_VERSION injected by cargo-release
# https://github.com/sunng87/cargo-release/blob/master/docs/reference.md#hook-environment-variables
#
# shellcheck disable=SC2068
main $@
