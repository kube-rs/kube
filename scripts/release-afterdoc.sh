#!/usr/bin/env bash
set -eo pipefail

main() {
  if [ -z "$1" ]; then
    echo "please use ./release-afterdoc.sh TAG"
    echo "TAG is probably the result of: git tag -l | tail -n 1"
    exit 1
  fi
  local -r RELNAME="$1"
  local -r RELEASE="$(curl -sSL -H "Accept: application/vnd.github.v3+json" "https://api.github.com/repos/kube-rs/kube/releases/tags/${RELNAME}")"
  # local -r RELREG="$(echo "${RELNAME}" | sd -s "." "\.")"
  local -r HURL="$(echo "${RELEASE}" | jq '.html_url' -r)"
  # Skipping New Contributors highight from CHANGELOG + across repos for brevity and to avoid pinging them excessively
  local -r BODY="$(echo "${RELEASE}" | jq '.body' -r | sd "## New Contributors[\w\W]*$" "")"

  # Add in the body first
  sd "(^UNRELEASED\n===================\n \* see https://.*\.\.\.main)" "\${1}\n\nXXXYYYZZZ${BODY}" CHANGELOG.md
  # fix newlines issues caused last jq/sd combo: (^M at end of lines)
  sd "\r" "" CHANGELOG.md

  # Link the headline
  sd "(^XXXYYYZZZ)" "[${RELNAME}](${HURL}) / $(date '+%F')\n===================\n" CHANGELOG.md
  # Update compare url
  sd "(\* see https://.*/kube/compare/).+\.\.\.main" "\${1}${RELNAME}...main" CHANGELOG.md
}

# This script ports manual RELEASE notes into the CHANGELOG post publishing
# and links to the github releases so that the website has good links.
#
# This is run manually after hitting publish on the auto-created draft release.
#
# Usage:
#
# ./scripts/release-afterdoc.sh 0.68.0
# inspect diff, then commit and push if OK.
#
# shellcheck disable=SC2068
main $@
