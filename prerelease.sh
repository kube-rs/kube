#!/bin/bash
set -euo pipefail

replace-docs() {
  sd "UNRELEASED" "${NEW_VERSION} / $(date +%Y-%m-%d)" CHANGELOG.md
  sd " \* see https://github.com/clux/kube-rs/compare/.*...master\n" "" CHANGELOG.md
  sd "<!-- next-header -->" "<!-- next-header -->\nUNRELEASED\n===================\n * see https://github.com/clux/kube-rs/compare/${NEW_VERSION}...master\n" CHANGELOG.md
  sed -i "s/${PREV_VERSION}/${NEW_VERSION}/g" kube-derive/README.md
  sed -i "s/${PREV_VERSION}/${NEW_VERSION}/g" README.md
}

sanity() {
  CARGO_TREE_OPENAPI="$(cargo tree -i k8s-openapi | head -n 1 | awk '{print $2}')"
  USED_K8S_OPENAPI="${CARGO_TREE_OPENAPI:1}"
  RECOMMENDED_K8S_OPENAPI="$(rg "k8s-openapi =" README.md | head -n 1)" # only check first instance
  if ! [[ $RECOMMENDED_K8S_OPENAPI =~ $USED_K8S_OPENAPI ]]; then
    echo "prerelease: abort: recommending k8s-openapi pinned to a different version to what we use"
    exit 1
  fi
}

main() {
  # We only want this to run ONCE at workspace level
  cd "$(dirname "${BASH_SOURCE[0]}")" # aka $WORKSPACE_ROOT
  local -r CURRENT_VER="$(rg "kube =" README.md | head -n 1 | awk -F"\"" '{print $2}')"

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
